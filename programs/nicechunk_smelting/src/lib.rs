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

pub mod cluster_config;
pub mod errors;
pub mod state;

use cluster_config::NICECHUNK_BACKPACK_PROGRAM_ID;
use errors::{require_key_eq, NicechunkSmeltingError};
use state::{
    BackpackAccountView, RecipeRecord, RecipeTable, RecipeTableInitArgs,
    DEFAULT_OUTPUT_VOLUME_DIVISOR, DEFAULT_RESOURCE_VOLUME_MM3, RECIPE_TABLE_SEED,
    SMELTING_AUTHORITY_SEED,
};

declare_id!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");

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
        3 => set_recipe_table_authority(program_id, accounts),
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
            authority: payer.key,
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
    RecipeTable::validate_authority(&data, authority.key)?;
    let recipe = RecipeRecord::unpack_args(payload, clock.slot)?;
    RecipeTable::upsert_recipe(&mut data, &recipe, clock.slot)
}

fn set_recipe_table_authority(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 3 {
        return Err(NicechunkSmeltingError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let recipe_table = next_account_info(account_info_iter)?;
    let new_authority = next_account_info(account_info_iter)?;

    if !authority.is_signer {
        return Err(NicechunkSmeltingError::UnauthorizedAuthority.into());
    }
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
    RecipeTable::validate_authority(&data, authority.key)?;
    RecipeTable::set_authority(&mut data, new_authority.key, clock.slot)
}

fn execute_smelting(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 5 || payload.len() < 10 {
        return Err(NicechunkSmeltingError::InvalidInstruction.into());
    }
    let recipe_id = read_u64(payload, 0);
    let input_count = payload[8] as usize;
    let fuel_count = payload[9] as usize;
    let has_multiplier = payload.len() == 12 + input_count + fuel_count;
    let multiplier = if has_multiplier { read_u16(payload, 10) } else { 1 };
    let index_offset = if has_multiplier { 12 } else { 10 };
    if input_count == 0
        || fuel_count == 0
        || input_count + fuel_count > 99
        || multiplier == 0
        || (!has_multiplier && payload.len() != 10 + input_count + fuel_count)
    {
        return Err(NicechunkSmeltingError::InvalidInstruction.into());
    }
    let indexes = &payload[index_offset..index_offset + input_count];
    let fuel_indexes = &payload[index_offset + input_count..];

    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let recipe_table = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let smelting_authority = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkSmeltingError::InvalidPayer.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkSmeltingError::InvalidWritableAccount.into());
    }
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
    validate_smelting_authority(program_id, smelting_authority.key)?;

    let recipe_table_data = recipe_table.try_borrow_data()?;
    let recipe = RecipeTable::find_recipe(&recipe_table_data, recipe_id)?;
    drop(recipe_table_data);

    {
        let backpack_data = backpack.try_borrow_data()?;
        BackpackAccountView::validate_recipe_inputs(&backpack_data, owner.key, indexes, fuel_indexes, &recipe, multiplier)?;
    }

    remove_backpack_resources(owner, backpack, backpack_program, indexes, fuel_indexes)?;
    for output_index in 0..recipe.output_count as usize {
        append_smelting_output_to_backpack(
            program_id,
            smelting_authority,
            owner,
            backpack,
            backpack_program,
            &recipe.outputs[output_index],
            recipe.input_count,
            multiplier,
        )?;
    }
    Ok(())
}

fn remove_backpack_resources<'a>(
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    _backpack_program: &AccountInfo<'a>,
    indexes: &[u8],
    fuel_indexes: &[u8],
) -> ProgramResult {
    let mut data = Vec::with_capacity(2 + indexes.len() + fuel_indexes.len());
    data.push(4);
    data.push((indexes.len() + fuel_indexes.len()) as u8);
    data.extend_from_slice(indexes);
    data.extend_from_slice(fuel_indexes);
    let ix = Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*owner.key, true),
            AccountMeta::new(*backpack.key, false),
        ],
        data,
    };
    invoke(&ix, &[owner.clone(), backpack.clone()])
}

fn append_smelting_output_to_backpack<'a>(
    program_id: &Pubkey,
    smelting_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    _backpack_program: &AccountInfo<'a>,
    record: &state::BackpackSlotRecord,
    recipe_input_count: u8,
    multiplier: u16,
) -> ProgramResult {
    let (_, bump) = Pubkey::find_program_address(&[SMELTING_AUTHORITY_SEED], program_id);
    let mut data = vec![0_u8; 1 + state::BACKPACK_SLOT_RECORD_LEN];
    data[0] = 5;
    let mut output = *record;
    let base_volume = if output.volume_mm3 > 0 {
        output.volume_mm3
    } else {
        DEFAULT_RESOURCE_VOLUME_MM3
            .saturating_mul(recipe_input_count as u32)
            .saturating_div(DEFAULT_OUTPUT_VOLUME_DIVISOR)
            .max(1)
    };
    output.volume_mm3 = base_volume.saturating_mul(multiplier as u32).max(1);
    output.pack(&mut data[1..])?;
    let ix = Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*smelting_authority.key, true),
            AccountMeta::new_readonly(*owner.key, false),
            AccountMeta::new(*backpack.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[smelting_authority.clone(), owner.clone(), backpack.clone()],
        &[&[SMELTING_AUTHORITY_SEED, &[bump]]],
    )
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

fn validate_smelting_authority(
    program_id: &Pubkey,
    authority: &Pubkey,
) -> Result<(), solana_program::program_error::ProgramError> {
    let (expected_authority, _) = Pubkey::find_program_address(&[SMELTING_AUTHORITY_SEED], program_id);
    require_key_eq(
        authority,
        &expected_authority,
        NicechunkSmeltingError::InvalidSmeltingAuthority,
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
