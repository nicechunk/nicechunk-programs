#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

pub mod civilization_adapter;
pub mod cluster_config;
pub mod errors;
pub mod state;

use cluster_config::{
    NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_CIVILIZATION_PROGRAM_ID, NICECHUNK_CORE_PROGRAM_ID,
    NICECHUNK_SKILLS_PROGRAM_ID, NICECHUNK_SMELTING_RECIPE_AUTHORITY,
};
use errors::{require_key_eq, NicechunkSmeltingError};
use state::{
    BackpackAccountView, PlayerProgressInitArgs, PlayerProgressState, RecipeRecord, RecipeTable,
    RecipeTableInitArgs, DEFAULT_RESOURCE_VOLUME_MM3, DURABILITY_BPS_DENOMINATOR,
    PLAYER_PROGRESS_LEN, PLAYER_PROGRESS_SEED, RECIPE_TABLE_SEED, RECIPE_YIELD_BPS_DENOMINATOR,
    SMELTING_AUTHORITY_SEED, SMELTING_SKILL_BASE_OUTPUT_BPS, SMELTING_SKILL_MAX_OUTPUT_BPS,
    SMELTING_XP_PER_INPUT,
};

const GLOBAL_CONFIG_SEED: &[u8] = b"global-config";

declare_id!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");

const PLAYER_SKILLS_SEED: &[u8] = b"player-skills-v1";
const PLAYER_SKILLS_MAGIC: [u8; 8] = *b"NCKSKL01";
const PLAYER_SKILLS_VERSION: u16 = 1;
const PLAYER_SKILLS_LEN: usize = 480;
const PLAYER_SKILLS_OWNER_OFFSET: usize = 12;
const PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET: usize = 44;
const PLAYER_SKILLS_LEVELS_OFFSET: usize = 156;
const SMELTING_SKILL_INDEX: usize = 2;
const MAX_SKILL_LEVEL: u8 = 10;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkSmeltingError::InvalidInstruction)?;

    match tag {
        0 => initialize_recipe_table(program_id, accounts, payload),
        1 => upsert_recipe(program_id, accounts, payload),
        2 => execute_smelting(program_id, accounts, payload),
        4 => apply_civilization_recipe_receipt(program_id, accounts, payload),
        _ => Err(NicechunkSmeltingError::InvalidInstruction.into()),
    }
}

fn initialize_recipe_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 8 {
        return Err(NicechunkSmeltingError::InvalidInstruction.into());
    }
    let table_id = read_u64(payload, 0);
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let recipe_table = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkSmeltingError::InvalidPayer.into());
    }
    validate_recipe_authority(payer.key)?;
    if !recipe_table.is_writable {
        return Err(NicechunkSmeltingError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkSmeltingError::InvalidSystemProgram,
    )?;
    let bump = validate_recipe_table_pda(program_id, recipe_table.key, table_id)?;
    if recipe_table.owner == program_id {
        return Err(NicechunkSmeltingError::RecipeTableAlreadyInitialized.into());
    }
    if recipe_table.owner != &system_program::ID || recipe_table.data_len() != 0 {
        return Err(NicechunkSmeltingError::InvalidSystemAccount.into());
    }

    create_recipe_table_pda(
        payer,
        recipe_table,
        system_program_account,
        program_id,
        table_id,
        bump,
    )?;

    let clock = Clock::get()?;
    let mut data = recipe_table.try_borrow_mut_data()?;
    RecipeTable::pack_empty(
        &mut data,
        &RecipeTableInitArgs {
            bump,
            table_id,
            authority: &NICECHUNK_SMELTING_RECIPE_AUTHORITY,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn upsert_recipe(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 2 {
        return Err(NicechunkSmeltingError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let recipe_table = next_account_info(account_info_iter)?;

    if !authority.is_signer {
        return Err(NicechunkSmeltingError::UnauthorizedAuthority.into());
    }
    validate_recipe_authority(authority.key)?;
    if !recipe_table.is_writable {
        return Err(NicechunkSmeltingError::InvalidWritableAccount.into());
    }
    require_key_eq(
        recipe_table.owner,
        program_id,
        NicechunkSmeltingError::InvalidRecipeTableOwner,
    )?;

    let clock = Clock::get()?;
    let mut data = recipe_table.try_borrow_mut_data()?;
    RecipeTable::validate_identity(
        &data,
        program_id,
        recipe_table.key,
        &NICECHUNK_SMELTING_RECIPE_AUTHORITY,
    )?;
    let recipe = RecipeRecord::unpack_args(payload, clock.slot)?;
    recipe.validate_outputs_for_table(recipe_table.key)?;
    RecipeTable::upsert_recipe(&mut data, &recipe, clock.slot)
}

fn apply_civilization_recipe_receipt(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 8 {
        return Err(NicechunkSmeltingError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let executor = next_account_info(account_info_iter)?;
    let recipe_table = next_account_info(account_info_iter)?;
    let civilization_program = next_account_info(account_info_iter)?;
    let rule_book = next_account_info(account_info_iter)?;
    let tally = next_account_info(account_info_iter)?;
    let receipt = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let adapter_authority = next_account_info(account_info_iter)?;

    if !executor.is_signer || !executor.is_writable {
        return Err(NicechunkSmeltingError::InvalidPayer.into());
    }
    if !recipe_table.is_writable || !rule_book.is_writable || !receipt.is_writable {
        return Err(NicechunkSmeltingError::InvalidWritableAccount.into());
    }
    if civilization_program.key == program_id {
        return Err(NicechunkSmeltingError::InvalidCivilizationProgram.into());
    }
    require_key_eq(
        civilization_program.key,
        &NICECHUNK_CIVILIZATION_PROGRAM_ID,
        NicechunkSmeltingError::InvalidCivilizationProgram,
    )?;
    require_key_eq(
        recipe_table.owner,
        program_id,
        NicechunkSmeltingError::InvalidRecipeTableOwner,
    )?;
    require_key_eq(
        rule_book.owner,
        civilization_program.key,
        NicechunkSmeltingError::InvalidCivilizationRule,
    )?;
    require_key_eq(
        tally.owner,
        civilization_program.key,
        NicechunkSmeltingError::InvalidCivilizationTally,
    )?;
    {
        let table_data = recipe_table.try_borrow_data()?;
        RecipeTable::validate_identity(
            &table_data,
            program_id,
            recipe_table.key,
            &NICECHUNK_SMELTING_RECIPE_AUTHORITY,
        )?;
    }

    {
        let rule_data = rule_book.try_borrow_data()?;
        civilization_adapter::validate_rule_book_for_smelting_patch(
            &rule_data,
            civilization_program.key,
            program_id,
            recipe_table.key,
            payload,
            civilization_adapter::CIVILIZATION_STATUS_FINALIZED,
        )?;
    }
    {
        let tally_data = tally.try_borrow_data()?;
        civilization_adapter::validate_tally_threshold(&tally_data, rule_book.key)?;
    }

    civilization_adapter::invoke_civilization_execute_receipt(
        executor,
        rule_book,
        tally,
        receipt,
        system_program_account,
        civilization_program,
        adapter_authority,
        program_id,
    )?;

    require_key_eq(
        receipt.owner,
        civilization_program.key,
        NicechunkSmeltingError::InvalidCivilizationReceipt,
    )?;
    {
        let rule_data = rule_book.try_borrow_data()?;
        civilization_adapter::validate_rule_book_for_smelting_patch(
            &rule_data,
            civilization_program.key,
            program_id,
            recipe_table.key,
            payload,
            civilization_adapter::CIVILIZATION_STATUS_EXECUTED,
        )?;
    }
    {
        let receipt_data = receipt.try_borrow_data()?;
        civilization_adapter::validate_execution_receipt(&receipt_data, rule_book.key)?;
    }

    let clock = Clock::get()?;
    let recipe = RecipeRecord::unpack_civilization_patch_args(payload, clock.slot)?;
    recipe.validate_outputs_for_table(recipe_table.key)?;
    let mut data = recipe_table.try_borrow_mut_data()?;
    RecipeTable::validate_identity(
        &data,
        program_id,
        recipe_table.key,
        &NICECHUNK_SMELTING_RECIPE_AUTHORITY,
    )?;
    RecipeTable::upsert_recipe(&mut data, &recipe, clock.slot)
}

fn execute_smelting(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 10 || payload.len() < 13 {
        return Err(NicechunkSmeltingError::InvalidInstruction.into());
    }
    let recipe_id = read_u64(payload, 0);
    let input_count = payload[8] as usize;
    let fuel_count = payload[9] as usize;
    let multiplier = read_u16(payload, 10);
    let index_offset = 12;
    // Ambient crafting recipes intentionally submit no fuel. Heated recipes
    // are still rejected later when max fuel tier is below min_heat_tier.
    if !smelting_payload_shape_is_valid(payload.len(), input_count, fuel_count, multiplier) {
        return Err(NicechunkSmeltingError::InvalidInstruction.into());
    }
    let indexes = &payload[index_offset..index_offset + input_count];
    let fuel_indexes = &payload[index_offset + input_count..];

    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let recipe_table = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let player_progress = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let smelting_authority = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let player_skills = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkSmeltingError::InvalidPayer.into());
    }
    if !backpack.is_writable || !player_progress.is_writable {
        return Err(NicechunkSmeltingError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkSmeltingError::InvalidSystemProgram,
    )?;
    require_key_eq(
        recipe_table.owner,
        program_id,
        NicechunkSmeltingError::InvalidRecipeTableOwner,
    )?;
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkSmeltingError::InvalidBackpackProgram,
    )?;
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkSmeltingError::InvalidBackpackProgram,
    )?;
    validate_global_config(global_config.key, global_config.owner)?;
    validate_smelting_authority(program_id, smelting_authority.key)?;
    let clock = Clock::get()?;
    let progress_bump = validate_player_progress_pda(
        program_id,
        player_progress.key,
        global_config.key,
        owner.key,
    )?;
    create_player_progress_if_needed(
        owner,
        player_progress,
        global_config,
        system_program_account,
        program_id,
        owner.key,
        progress_bump,
        &clock,
    )?;

    let recipe_table_data = recipe_table.try_borrow_data()?;
    RecipeTable::validate_identity(
        &recipe_table_data,
        program_id,
        recipe_table.key,
        &NICECHUNK_SMELTING_RECIPE_AUTHORITY,
    )?;
    let recipe = RecipeTable::find_recipe(&recipe_table_data, recipe_id)?;
    recipe.validate_outputs_for_table(recipe_table.key)?;
    drop(recipe_table_data);

    {
        let progress_data = player_progress.try_borrow_data()?;
        PlayerProgressState::validate(&progress_data, owner.key, global_config.key)?;
    }
    let smelting_level = player_skill_level(player_skills, global_config.key, owner.key)?;
    let skill_output_bps = PlayerProgressState::smelting_output_bps_from_level(smelting_level);

    let validated_inputs = {
        let backpack_data = backpack.try_borrow_data()?;
        BackpackAccountView::validate_recipe_inputs(
            &backpack_data,
            owner.key,
            indexes,
            fuel_indexes,
            &recipe,
            multiplier,
        )?
    };

    consume_backpack_resources(
        program_id,
        smelting_authority,
        owner,
        backpack,
        backpack_program,
        material_physics,
        indexes,
        fuel_indexes,
        &validated_inputs.consumption_quantities,
    )?;
    let recipe_input_volume_mm3 = recipe_input_volume_mm3(&recipe);
    for output_index in 0..recipe.output_count as usize {
        let output = &recipe.outputs[output_index];
        let output_volume_mm3 = smelting_output_volume_mm3(
            output.volume_mm3,
            validated_inputs.input_volume_mm3,
            recipe_input_volume_mm3,
            multiplier,
            recipe.yield_bps,
            skill_output_bps,
        );
        let output_quantity = smelting_output_quantity(
            &recipe,
            output,
            validated_inputs.consumed_input_units,
            multiplier,
            skill_output_bps,
        );
        append_smelting_output_to_backpack(
            program_id,
            smelting_authority,
            owner,
            backpack,
            backpack_program,
            material_physics,
            output,
            output_volume_mm3,
            output_quantity,
        )?;
    }
    {
        let mut progress_data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_smelting_xp(
            &mut progress_data,
            owner.key,
            global_config.key,
            validated_inputs
                .consumed_input_units
                .saturating_mul(SMELTING_XP_PER_INPUT),
            clock.slot,
        )?;
    }
    Ok(())
}

fn player_skill_level(
    player_skills: &AccountInfo,
    global_config: &Pubkey,
    owner: &Pubkey,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let (expected, _) = Pubkey::find_program_address(
        &[PLAYER_SKILLS_SEED, global_config.as_ref(), owner.as_ref()],
        &NICECHUNK_SKILLS_PROGRAM_ID,
    );
    require_key_eq(
        player_skills.key,
        &expected,
        NicechunkSmeltingError::InvalidPlayerSkillsPda,
    )?;
    if player_skills.owner == &system_program::ID && player_skills.data_len() == 0 {
        return Ok(0);
    }
    require_key_eq(
        player_skills.owner,
        &NICECHUNK_SKILLS_PROGRAM_ID,
        NicechunkSmeltingError::InvalidPlayerSkillsOwner,
    )?;
    let data = player_skills.try_borrow_data()?;
    if data.len() != PLAYER_SKILLS_LEN
        || data[0..8] != PLAYER_SKILLS_MAGIC
        || u16::from_le_bytes([data[8], data[9]]) != PLAYER_SKILLS_VERSION
        || data[11] != 1
        || &data[PLAYER_SKILLS_OWNER_OFFSET..PLAYER_SKILLS_OWNER_OFFSET + 32] != owner.as_ref()
        || &data[PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET..PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
    {
        return Err(NicechunkSmeltingError::InvalidPlayerSkillsData.into());
    }
    let level = data[PLAYER_SKILLS_LEVELS_OFFSET + SMELTING_SKILL_INDEX];
    if level > MAX_SKILL_LEVEL {
        return Err(NicechunkSmeltingError::InvalidPlayerSkillsData.into());
    }
    Ok(level)
}

fn smelting_payload_shape_is_valid(
    payload_len: usize,
    input_count: usize,
    fuel_count: usize,
    multiplier: u16,
) -> bool {
    input_count > 0
        && input_count + fuel_count <= 99
        && multiplier > 0
        && payload_len == 12 + input_count + fuel_count
}

fn consume_backpack_resources<'a>(
    program_id: &Pubkey,
    smelting_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    backpack_program: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    indexes: &[u8],
    fuel_indexes: &[u8],
    consumption_quantities: &[u32; state::BACKPACK_MAX_CAPACITY],
) -> ProgramResult {
    let (_, bump) = Pubkey::find_program_address(&[SMELTING_AUTHORITY_SEED], program_id);
    let mut data = Vec::with_capacity(3 + indexes.len() * 5 + fuel_indexes.len());
    data.push(14);
    data.push(indexes.len() as u8);
    for index in indexes {
        let quantity = consumption_quantities[*index as usize];
        if quantity == 0 {
            return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
        }
        data.push(*index);
        data.extend_from_slice(&quantity.to_le_bytes());
    }
    data.push(fuel_indexes.len() as u8);
    data.extend_from_slice(fuel_indexes);
    let data = backpack_cpi_data(&data);
    let ix = Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*smelting_authority.key, true),
            AccountMeta::new_readonly(*owner.key, true),
            AccountMeta::new(*backpack.key, false),
            AccountMeta::new_readonly(*material_physics.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[
            smelting_authority.clone(),
            owner.clone(),
            backpack.clone(),
            material_physics.clone(),
            backpack_program.clone(),
        ],
        &[&[SMELTING_AUTHORITY_SEED, &[bump]]],
    )
}

fn append_smelting_output_to_backpack<'a>(
    program_id: &Pubkey,
    smelting_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    backpack_program: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    record: &state::BackpackSlotRecord,
    output_volume_mm3: u64,
    output_quantity: u32,
) -> ProgramResult {
    let (_, bump) = Pubkey::find_program_address(&[SMELTING_AUTHORITY_SEED], program_id);
    let mut data = vec![0_u8; 1 + state::BACKPACK_SLOT_RECORD_LEN];
    data[0] = 5;
    let mut output = *record;
    output.quantity = output_quantity.max(1);
    output.volume_mm3 = output_volume_mm3.min(u32::MAX as u64).max(1) as u32;
    normalize_smelting_output_metadata(&mut output);
    output.pack(&mut data[1..])?;
    let data = backpack_cpi_data(&data);
    let ix = Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*smelting_authority.key, true),
            AccountMeta::new_readonly(*owner.key, false),
            AccountMeta::new(*backpack.key, false),
            AccountMeta::new_readonly(*material_physics.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[
            smelting_authority.clone(),
            owner.clone(),
            backpack.clone(),
            material_physics.clone(),
            backpack_program.clone(),
        ],
        &[&[SMELTING_AUTHORITY_SEED, &[bump]]],
    )
}

fn normalize_smelting_output_metadata(output: &mut state::BackpackSlotRecord) {
    if output.kind != state::BACKPACK_SLOT_KIND_ITEM {
        return;
    }
    let per_unit_durability = output.durability_max.max(1) as u64;
    let current_bps = if output.durability_max > 0 {
        (output.durability_current.max(1) as u64)
            .saturating_mul(DURABILITY_BPS_DENOMINATOR)
            .saturating_div(output.durability_max.max(1) as u64)
    } else {
        DURABILITY_BPS_DENOMINATOR
    };
    let scaled_max = per_unit_durability
        .saturating_mul(output.volume_mm3.max(1) as u64)
        .saturating_add(500_000)
        .saturating_div(1_000_000)
        .max(1)
        .min(u32::MAX as u64) as u32;
    let scaled_current = (scaled_max as u64)
        .saturating_mul(current_bps.max(1).min(DURABILITY_BPS_DENOMINATOR))
        .saturating_add(DURABILITY_BPS_DENOMINATOR / 2)
        .saturating_div(DURABILITY_BPS_DENOMINATOR)
        .max(1)
        .min(scaled_max as u64) as u32;
    output.durability_max = scaled_max;
    output.durability_current = scaled_current;
    output.grade = output.grade.max(1).min(10);
    output.quality_bps = output
        .quality_bps
        .max(1)
        .min(DURABILITY_BPS_DENOMINATOR as u16);
    let effective_durability = (output.durability_current as u64)
        .saturating_mul(output.quality_bps as u64)
        .saturating_div(DURABILITY_BPS_DENOMINATOR);
    output.item_level =
        smelting_material_item_level(effective_durability, output.volume_mm3 as u64)
            .max(output.item_level.max(1))
            .min(100);
}

fn smelting_material_item_level(effective_durability: u64, total_volume_mm3: u64) -> u8 {
    let durability_level = integer_sqrt(effective_durability / 25).min(80);
    let volume_level = (total_volume_mm3 / 500_000).min(20);
    (1_u64
        .saturating_add(durability_level)
        .saturating_add(volume_level))
    .min(100) as u8
}

fn integer_sqrt(value: u64) -> u64 {
    if value <= 1 {
        return value;
    }
    let mut estimate = value;
    let mut next = (estimate + value / estimate) / 2;
    while next < estimate {
        estimate = next;
        next = (estimate + value / estimate) / 2;
    }
    estimate
}

fn recipe_input_volume_mm3(recipe: &RecipeRecord) -> u64 {
    recipe
        .inputs
        .iter()
        .take(recipe.input_count as usize)
        .map(|input| {
            let unit_volume = if input.volume_mm3 > 0 {
                input.volume_mm3 as u64
            } else {
                DEFAULT_RESOURCE_VOLUME_MM3 as u64
            };
            unit_volume.saturating_mul(input.quantity.max(1) as u64)
        })
        .sum::<u64>()
        .max(1)
}

fn smelting_output_quantity(
    recipe: &RecipeRecord,
    output: &state::BackpackSlotRecord,
    consumed_input_units: u64,
    multiplier: u16,
    skill_output_bps: u16,
) -> u32 {
    if recipe_is_material_merge(recipe) {
        return consumed_input_units.max(1).min(u32::MAX as u64) as u32;
    }
    let quantity = (output.quantity.max(1) as u128)
        .saturating_mul(multiplier.max(1) as u128)
        .saturating_mul(skill_output_bps.clamp(
            SMELTING_SKILL_BASE_OUTPUT_BPS,
            SMELTING_SKILL_MAX_OUTPUT_BPS,
        ) as u128)
        .saturating_div(RECIPE_YIELD_BPS_DENOMINATOR as u128);
    quantity.max(1).min(u32::MAX as u128) as u32
}

fn recipe_is_material_merge(recipe: &RecipeRecord) -> bool {
    if recipe.input_count != 1 || recipe.output_count != 1 || recipe.min_heat_tier != 0 {
        return false;
    }
    let input = &recipe.inputs[0];
    let output = &recipe.outputs[0];
    input.kind == state::BACKPACK_SLOT_KIND_ITEM
        && output.kind == state::BACKPACK_SLOT_KIND_ITEM
        && input.category == state::BACKPACK_ITEM_CATEGORY_MATERIAL
        && output.category == state::BACKPACK_ITEM_CATEGORY_MATERIAL
        && input.item_code == output.item_code
}

fn smelting_output_volume_mm3(
    pda_output_volume_mm3: u32,
    input_volume_mm3: u64,
    recipe_input_volume_mm3: u64,
    multiplier: u16,
    recipe_yield_bps: u16,
    skill_output_bps: u16,
) -> u64 {
    let multiplier = multiplier.max(1) as u128;
    let skill_multiplier_bps = skill_output_bps.clamp(
        SMELTING_SKILL_BASE_OUTPUT_BPS,
        SMELTING_SKILL_MAX_OUTPUT_BPS,
    ) as u128;
    let expected_input_volume = (recipe_input_volume_mm3.max(1) as u128)
        .saturating_mul(multiplier)
        .max(1);
    let pda_volume = (pda_output_volume_mm3.max(1) as u128)
        .saturating_mul(multiplier)
        .saturating_mul(input_volume_mm3.max(1) as u128)
        .saturating_div(expected_input_volume);
    let recipe_volume = pda_volume
        .saturating_mul(recipe_yield_bps as u128)
        .saturating_div(RECIPE_YIELD_BPS_DENOMINATOR as u128);
    recipe_volume
        .saturating_mul(skill_multiplier_bps)
        .saturating_div(RECIPE_YIELD_BPS_DENOMINATOR as u128)
        .min(u32::MAX as u128)
        .max(1) as u64
}

#[cfg(feature = "unified-game")]
fn backpack_cpi_data(data: &[u8]) -> Vec<u8> {
    let mut wrapped = Vec::with_capacity(data.len() + 1);
    wrapped.push(1);
    wrapped.extend_from_slice(data);
    wrapped
}

#[cfg(not(feature = "unified-game"))]
fn backpack_cpi_data(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

fn validate_recipe_table_pda(
    program_id: &Pubkey,
    recipe_table: &Pubkey,
    table_id: u64,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let table_id_bytes = table_id.to_le_bytes();
    let (expected_recipe_table, bump) =
        Pubkey::find_program_address(&[RECIPE_TABLE_SEED, &table_id_bytes], program_id);
    require_key_eq(
        recipe_table,
        &expected_recipe_table,
        NicechunkSmeltingError::InvalidRecipeTablePda,
    )?;
    Ok(bump)
}

fn validate_recipe_authority(
    authority: &Pubkey,
) -> Result<(), solana_program::program_error::ProgramError> {
    require_key_eq(
        authority,
        &NICECHUNK_SMELTING_RECIPE_AUTHORITY,
        NicechunkSmeltingError::UnauthorizedAuthority,
    )
}

fn validate_global_config(
    global_config: &Pubkey,
    owner: &Pubkey,
) -> Result<(), solana_program::program_error::ProgramError> {
    require_key_eq(
        owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkSmeltingError::InvalidGlobalConfig,
    )?;
    let (expected_global_config, _) =
        Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], &NICECHUNK_CORE_PROGRAM_ID);
    require_key_eq(
        global_config,
        &expected_global_config,
        NicechunkSmeltingError::InvalidGlobalConfig,
    )
}

fn validate_smelting_authority(
    program_id: &Pubkey,
    authority: &Pubkey,
) -> Result<(), solana_program::program_error::ProgramError> {
    let (expected_authority, _) =
        Pubkey::find_program_address(&[SMELTING_AUTHORITY_SEED], program_id);
    require_key_eq(
        authority,
        &expected_authority,
        NicechunkSmeltingError::InvalidSmeltingAuthority,
    )
}

fn validate_player_progress_pda(
    program_id: &Pubkey,
    player_progress: &Pubkey,
    global_config: &Pubkey,
    owner: &Pubkey,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let (expected_progress, bump) = Pubkey::find_program_address(
        &[PLAYER_PROGRESS_SEED, global_config.as_ref(), owner.as_ref()],
        program_id,
    );
    require_key_eq(
        player_progress,
        &expected_progress,
        NicechunkSmeltingError::InvalidPlayerProgress,
    )?;
    Ok(bump)
}

#[allow(clippy::too_many_arguments)]
fn create_player_progress_if_needed<'a>(
    payer: &AccountInfo<'a>,
    player_progress: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    owner: &Pubkey,
    bump: u8,
    clock: &Clock,
) -> ProgramResult {
    let seeds = &[
        PLAYER_PROGRESS_SEED,
        global_config.key.as_ref(),
        owner.as_ref(),
        &[bump],
    ];

    if player_progress.owner == program_id {
        let data = player_progress.try_borrow_data()?;
        PlayerProgressState::validate(&data, owner, global_config.key)?;
        return Ok(());
    }

    if player_progress.owner != &system_program::ID || player_progress.data_len() != 0 {
        return Err(NicechunkSmeltingError::InvalidSystemAccount.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PLAYER_PROGRESS_LEN);
    if player_progress.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            player_progress.key,
            lamports,
            PLAYER_PROGRESS_LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                player_progress.clone(),
                system_program_account.clone(),
            ],
            &[seeds],
        )?;
    } else {
        if player_progress.lamports() < lamports {
            let delta = lamports - player_progress.lamports();
            let transfer = system_instruction::transfer(payer.key, player_progress.key, delta);
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    player_progress.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        let allocate =
            system_instruction::allocate(player_progress.key, PLAYER_PROGRESS_LEN as u64);
        invoke_signed(
            &allocate,
            &[player_progress.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_instruction::assign(player_progress.key, program_id);
        invoke_signed(
            &assign,
            &[player_progress.clone(), system_program_account.clone()],
            &[seeds],
        )?;
    }

    let mut data = player_progress.try_borrow_mut_data()?;
    PlayerProgressState::pack_empty(
        &mut data,
        &PlayerProgressInitArgs {
            bump,
            owner,
            global_config: global_config.key,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn create_recipe_table_pda<'a>(
    payer: &AccountInfo<'a>,
    recipe_table: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    table_id: u64,
    bump: u8,
) -> ProgramResult {
    let table_id_bytes = table_id.to_le_bytes();
    let seeds = &[RECIPE_TABLE_SEED, &table_id_bytes, &[bump]];
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(RecipeTable::LEN);
    let create = system_instruction::create_account(
        payer.key,
        recipe_table.key,
        lamports,
        RecipeTable::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            payer.clone(),
            recipe_table.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::{
        process_instruction, smelting_output_quantity, smelting_output_volume_mm3,
        smelting_payload_shape_is_valid, validate_global_config, validate_recipe_authority,
        GLOBAL_CONFIG_SEED,
    };
    use crate::state::{
        BackpackSlotRecord, PlayerProgressState, RecipeRecord, BACKPACK_ITEM_CATEGORY_MATERIAL,
        BACKPACK_SLOT_KIND_ITEM,
    };
    use crate::{
        cluster_config::{NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_SMELTING_RECIPE_AUTHORITY},
        errors::NicechunkSmeltingError,
    };
    use solana_program::{program_error::ProgramError, pubkey::Pubkey};

    #[test]
    fn recipe_governance_is_pinned_and_authority_transfer_tag_is_retired() {
        assert!(validate_recipe_authority(&NICECHUNK_SMELTING_RECIPE_AUTHORITY).is_ok());
        assert_eq!(
            validate_recipe_authority(&Pubkey::new_unique()).unwrap_err(),
            ProgramError::Custom(NicechunkSmeltingError::UnauthorizedAuthority as u32),
        );
        assert_eq!(
            process_instruction(&crate::id(), &[], &[3]).unwrap_err(),
            ProgramError::Custom(NicechunkSmeltingError::InvalidInstruction as u32),
        );
    }

    #[test]
    fn global_config_is_pinned_to_the_core_program_and_pda() {
        let (global_config, _) =
            Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], &NICECHUNK_CORE_PROGRAM_ID);
        assert!(validate_global_config(&global_config, &NICECHUNK_CORE_PROGRAM_ID).is_ok());
        assert!(validate_global_config(&Pubkey::new_unique(), &NICECHUNK_CORE_PROGRAM_ID).is_err());
        assert!(validate_global_config(&global_config, &Pubkey::new_unique()).is_err());
    }

    #[test]
    fn smelting_skill_adds_zero_to_fifty_percent_output() {
        assert_eq!(
            PlayerProgressState::smelting_output_bps_from_level(0),
            10_000
        );
        assert_eq!(
            PlayerProgressState::smelting_output_bps_from_level(1),
            10_500,
        );
        assert_eq!(
            PlayerProgressState::smelting_output_bps_from_level(u8::MAX),
            15_000
        );
    }

    #[test]
    fn smelting_skill_bonus_scales_real_output_volume() {
        let base = smelting_output_volume_mm3(1_000_000, 1_000_000, 1_000_000, 1, 10_000, 10_000);
        let maximum =
            smelting_output_volume_mm3(1_000_000, 1_000_000, 1_000_000, 1, 10_000, 15_000);

        assert_eq!(base, 1_000_000);
        assert_eq!(maximum, 1_500_000);
    }

    #[test]
    fn copper_bloom_base_output_is_six_point_two_percent_of_input() {
        let output = smelting_output_volume_mm3(3_000_000, 300_000, 3_000_000, 1, 620, 10_000);
        assert_eq!(output, 18_600);
    }

    #[test]
    fn final_recipe_payload_requires_multiplier_and_accepts_zero_fuel_indexes() {
        assert!(smelting_payload_shape_is_valid(16, 4, 0, 2));
        assert!(!smelting_payload_shape_is_valid(14, 4, 0, 1));
        assert!(!smelting_payload_shape_is_valid(15, 4, 0, 1));
        assert!(!smelting_payload_shape_is_valid(12, 0, 0, 1));
        assert!(!smelting_payload_shape_is_valid(16, 4, 0, 0));
    }

    #[test]
    fn pda_output_volume_controls_each_material_independently() {
        let cloth = smelting_output_volume_mm3(1_000_000, 5_000_000, 5_000_000, 1, 6_500, 10_000);
        let dye = smelting_output_volume_mm3(20_000, 3_000_000, 3_000_000, 1, 6_000, 10_000);

        assert_eq!(cloth, 650_000);
        assert_eq!(dye, 12_000);
    }

    #[test]
    fn actual_input_volume_scales_the_pda_output() {
        let full = smelting_output_volume_mm3(1_000_000, 3_000_000, 3_000_000, 1, 6_000, 10_000);
        let half = smelting_output_volume_mm3(1_000_000, 1_500_000, 3_000_000, 1, 6_000, 10_000);

        assert_eq!(full, 600_000);
        assert_eq!(half, 300_000);
    }

    #[test]
    fn recipe_yield_and_skill_bonus_both_scale_final_material_volume() {
        let cloth = smelting_output_volume_mm3(5_000_000, 5_000_000, 5_000_000, 1, 650, 12_000);
        let dye = smelting_output_volume_mm3(3_000_000, 3_000_000, 3_000_000, 1, 600, 12_000);

        assert_eq!(cloth, 390_000);
        assert_eq!(dye, 216_000);
    }

    #[test]
    fn batch_multiplier_scales_pda_output_without_equal_splitting() {
        let large = smelting_output_volume_mm3(1_000_000, 6_000_000, 3_000_000, 2, 10_000, 10_000);
        let small = smelting_output_volume_mm3(20_000, 6_000_000, 3_000_000, 2, 10_000, 10_000);

        assert_eq!(large, 2_000_000);
        assert_eq!(small, 40_000);
    }

    #[test]
    fn merge_recipe_preserves_real_input_volume_before_skill_bonus() {
        let output = smelting_output_volume_mm3(20_000, 8_400, 20_000, 1, 10_000, 12_000);
        assert_eq!(output, 10_080);
    }

    #[test]
    fn primary_recipe_output_quantity_tracks_yield_count_batches_and_skill() {
        let mut recipe = RecipeRecord::default();
        recipe.input_count = 1;
        recipe.output_count = 1;
        recipe.inputs[0] = material_record(1031, 2);
        recipe.outputs[0] = material_record(1032, 4);

        assert_eq!(
            smelting_output_quantity(&recipe, &recipe.outputs[0], 2, 1, 10_000),
            4,
        );
        assert_eq!(
            smelting_output_quantity(&recipe, &recipe.outputs[0], 4, 2, 10_000),
            8,
        );
        assert_eq!(
            smelting_output_quantity(&recipe, &recipe.outputs[0], 2, 1, 15_000),
            6,
        );
    }

    #[test]
    fn merge_recipe_output_quantity_preserves_all_consumed_units() {
        let mut recipe = RecipeRecord::default();
        recipe.input_count = 1;
        recipe.output_count = 1;
        recipe.inputs[0] = material_record(1031, 1);
        recipe.outputs[0] = material_record(1031, 1);

        assert_eq!(
            smelting_output_quantity(&recipe, &recipe.outputs[0], 8, 2, 15_000),
            8,
        );
    }

    fn material_record(item_code: u16, quantity: u32) -> BackpackSlotRecord {
        BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_ITEM,
            category: BACKPACK_ITEM_CATEGORY_MATERIAL,
            quantity,
            item_code,
            ..BackpackSlotRecord::default()
        }
    }
}
