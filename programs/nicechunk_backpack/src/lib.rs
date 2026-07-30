#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
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

use cluster_config::{
    NICECHUNK_BLUEPRINT_ISSUER, NICECHUNK_BOOTSTRAP_AUTHORITY, NICECHUNK_CHUNK_PROGRAM_ID,
    NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_MARKET_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
    NICECHUNK_SKILLS_PROGRAM_ID, NICECHUNK_SMELTING_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkBackpackError};
use state::{
    verified_forge_design, BackpackAccount, BackpackInitArgs, BackpackResourceRecord,
    BackpackSlotRecord, BlueprintItemAccount, ForgeMaterialRequirements, ForgedItemAccount,
    ForgedItemInitArgs, MaterialPhysicsTableState, MaterialPhysicsTableView, PlayerEquipmentView,
    PlayerProfileView, PlayerSessionView, BACKPACK_BLUEPRINT_ITEM_CODE, BACKPACK_DEFAULT_CAPACITY,
    BACKPACK_ITEM_CATEGORY_BLUEPRINT, BACKPACK_ITEM_FLAG_UNIQUE, BACKPACK_SEED,
    BACKPACK_SLOT_KIND_ITEM, BLUEPRINT_ITEM_SEED, EQUIPMENT_TRANSFER_AUTHORITY_SEED,
    FORGED_ITEM_SEED, MATERIAL_PHYSICS_SEED, MAX_VERIFIED_FORGE_CODE_BYTES,
    SESSION_ACTION_BREAK_BLOCK,
};

declare_id!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");

const CHUNK_BROKEN_MAGIC: [u8; 4] = *b"NCBK";
const CHUNK_BROKEN_VERSION: u8 = 1;
const CHUNK_BROKEN_SEED: &[u8] = b"chunk-broken";
const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
const GLOBAL_CONFIG_SEED: &[u8] = b"global-config";
const GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET: usize = 53;
const GLOBAL_CONFIG_CHUNK_SIZE_OFFSET: usize = 259;
const GLOBAL_CONFIG_LEN: usize = 293;
const PLAYER_SKILLS_SEED: &[u8] = b"player-skills-v2";
const PLAYER_SKILLS_MAGIC: [u8; 8] = *b"NCKSKL02";
const PLAYER_SKILLS_VERSION: u16 = 2;
const PLAYER_SKILLS_LEN: usize = 480;
const PLAYER_SKILLS_OWNER_OFFSET: usize = 12;
const PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET: usize = 44;
const PLAYER_SKILLS_LEVELS_OFFSET: usize = 156;
const FORGING_SKILL_INDEX: usize = 3;
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
        .ok_or(NicechunkBackpackError::InvalidInstruction)?;

    match tag {
        0 => initialize_backpack(program_id, accounts, payload),
        1 => append_mined_resource(program_id, accounts, payload),
        2 => remove_resource(program_id, accounts, payload),
        3 => append_market_resource(program_id, accounts, payload),
        4 => remove_resources(program_id, accounts, payload),
        5 => append_smelting_item(program_id, accounts, payload),
        6 => append_mined_resources_batch(program_id, accounts, payload),
        7 => Err(NicechunkBackpackError::UnverifiedForgeInstructionDisabled.into()),
        8 => forge_equipment_with_material_verification(program_id, accounts, payload),
        9 => issue_blueprint(program_id, accounts, payload),
        10 => transfer_backpack_item_to_equipment(program_id, accounts, payload),
        11 => transfer_equipment_item_to_backpack(program_id, accounts, payload),
        12 => configure_material_physics(program_id, accounts, payload),
        13 => record_mining_action(program_id, accounts, payload),
        14 => consume_smelting_resources(program_id, accounts, payload),
        _ => Err(NicechunkBackpackError::InvalidInstruction.into()),
    }
}

fn consume_smelting_resources(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() < 2 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let input_count = payload[0] as usize;
    let input_bytes = input_count
        .checked_mul(5)
        .ok_or(NicechunkBackpackError::InvalidInstruction)?;
    let fuel_count_offset = 1_usize
        .checked_add(input_bytes)
        .ok_or(NicechunkBackpackError::InvalidInstruction)?;
    if input_count == 0 || fuel_count_offset >= payload.len() {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let fuel_count = payload[fuel_count_offset] as usize;
    if payload.len() != fuel_count_offset + 1 + fuel_count {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let mut input_quantities = [0_u32; state::BACKPACK_MAX_CAPACITY as usize];
    let mut fuel_indexes = [false; state::BACKPACK_MAX_CAPACITY as usize];
    for input_index in 0..input_count {
        let offset = 1 + input_index * 5;
        let index = payload[offset] as usize;
        let quantity = read_u32(payload, offset + 1);
        if index >= input_quantities.len() || quantity == 0 || input_quantities[index] != 0 {
            return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
        }
        input_quantities[index] = quantity;
    }
    for index in &payload[fuel_count_offset + 1..] {
        let selected = *index as usize;
        if selected >= fuel_indexes.len()
            || fuel_indexes[selected]
            || input_quantities[selected] != 0
        {
            return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
        }
        fuel_indexes[selected] = true;
    }

    let account_info_iter = &mut accounts.iter();
    let smelting_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    if !smelting_authority.is_signer {
        return Err(NicechunkBackpackError::InvalidSmeltingAuthority.into());
    }
    if !owner.is_signer {
        return Err(NicechunkBackpackError::InvalidPayer.into());
    }
    let (expected_authority, _) =
        Pubkey::find_program_address(&[b"smelting-authority"], &NICECHUNK_SMELTING_PROGRAM_ID);
    require_key_eq(
        smelting_authority.key,
        &expected_authority,
        NicechunkBackpackError::InvalidSmeltingAuthority,
    )?;
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;
    validate_material_physics_pda(program_id, material_physics)?;

    let physics_data = material_physics.try_borrow_data()?;
    let physics = MaterialPhysicsTableView::new(&physics_data)?;
    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::consume_smelting_resources(
        &mut backpack_data,
        owner.key,
        &input_quantities,
        &fuel_indexes,
        &physics,
        clock.slot,
    )
}

fn record_mining_action(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != 16 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let action_id = read_u64(payload, 0);
    let chunk_x = i32::from_le_bytes(
        payload[8..12]
            .try_into()
            .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
    );
    let chunk_z = i32::from_le_bytes(
        payload[12..16]
            .try_into()
            .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
    );
    let account_info_iter = &mut accounts.iter();
    let chunk_broken = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;

    let chunk_size = {
        let data = global_config.try_borrow_data()?;
        if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
            return Err(NicechunkBackpackError::InvalidGlobalConfig.into());
        }
        let value = i32::from(read_u16(&data, GLOBAL_CONFIG_CHUNK_SIZE_OFFSET));
        if value <= 0 {
            return Err(NicechunkBackpackError::InvalidGlobalConfig.into());
        }
        value
    };
    let record = BackpackResourceRecord {
        world_x: chunk_x
            .checked_mul(chunk_size)
            .ok_or(NicechunkBackpackError::InvalidChunkAuthority)?,
        world_y: 0,
        world_z: chunk_z
            .checked_mul(chunk_size)
            .ok_or(NicechunkBackpackError::InvalidChunkAuthority)?,
    };
    let owner = validate_chunk_reward_authority(
        program_id,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        &record,
    )?;
    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::record_mining_action(&mut backpack_data, &owner, action_id, clock.slot)
}

fn configure_material_physics(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkBackpackError::InvalidAccountCount.into());
    }
    MaterialPhysicsTableState::validate_payload(payload)?;

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !authority.is_signer || !authority.is_writable {
        return Err(NicechunkBackpackError::InvalidMaterialPhysicsAuthority.into());
    }
    if !material_physics.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkBackpackError::InvalidSystemProgram,
    )?;
    validate_global_config(global_config)?;
    let global_config_data = global_config.try_borrow_data()?;
    let treasury = Pubkey::new_from_array(
        global_config_data
            [GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET..GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET + 32]
            .try_into()
            .map_err(|_| NicechunkBackpackError::InvalidGlobalConfig)?,
    );
    drop(global_config_data);

    let (expected, bump) = Pubkey::find_program_address(
        &[MATERIAL_PHYSICS_SEED, global_config.key.as_ref()],
        program_id,
    );
    require_key_eq(
        material_physics.key,
        &expected,
        NicechunkBackpackError::InvalidMaterialPhysicsPda,
    )?;

    let is_initializing = material_physics.owner != program_id;
    if is_initializing
        && (material_physics.owner != &system_program::ID || material_physics.data_len() != 0)
    {
        return Err(NicechunkBackpackError::InvalidSystemAccount.into());
    }
    if authority.key != &treasury
        && (!is_initializing || authority.key != &NICECHUNK_BOOTSTRAP_AUTHORITY)
    {
        return Err(NicechunkBackpackError::InvalidMaterialPhysicsAuthority.into());
    }

    let next_revision = read_u32(payload, 0);
    if !is_initializing {
        let data = material_physics.try_borrow_data()?;
        if data.get(9).copied() != Some(bump)
            || next_revision <= MaterialPhysicsTableState::revision(&data)?
        {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData.into());
        }
        drop(data);
    } else {
        create_material_physics_pda(
            authority,
            material_physics,
            system_program_account,
            program_id,
            global_config.key,
            bump,
        )?;
    }

    let mut data = material_physics.try_borrow_mut_data()?;
    MaterialPhysicsTableState::pack_payload(&mut data, bump, payload)
}

fn transfer_backpack_item_to_equipment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 || payload.len() != 2 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let equipment_slot = payload[0];
    let backpack_index = payload[1];
    let account_info_iter = &mut accounts.iter();
    let transfer_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;

    validate_equipment_transfer_accounts(
        program_id,
        transfer_authority,
        owner,
        backpack,
        player_equipment,
        material_physics,
    )?;
    let previous_equipment = {
        let equipment_data = player_equipment.try_borrow_data()?;
        PlayerEquipmentView::custodied_slot(&equipment_data, equipment_slot)?
    };
    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    if let Some(previous) = previous_equipment {
        let physics_data = material_physics.try_borrow_data()?;
        MaterialPhysicsTableView::new(&physics_data)?.validate_mass(&previous)?;
        drop(physics_data);
        BackpackAccount::replace_slot_at(
            &mut backpack_data,
            owner.key,
            backpack_index,
            &previous,
            clock.slot,
        )
    } else {
        BackpackAccount::remove_resource_at(
            &mut backpack_data,
            owner.key,
            backpack_index,
            clock.slot,
        )
    }
}

fn transfer_equipment_item_to_backpack(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 || payload.len() != 1 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let equipment_slot = payload[0];
    let account_info_iter = &mut accounts.iter();
    let transfer_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;

    validate_equipment_transfer_accounts(
        program_id,
        transfer_authority,
        owner,
        backpack,
        player_equipment,
        material_physics,
    )?;
    let equipment_record = {
        let equipment_data = player_equipment.try_borrow_data()?;
        PlayerEquipmentView::custodied_slot(&equipment_data, equipment_slot)?
            .ok_or(NicechunkBackpackError::EquipmentSlotEmpty)?
    };
    let physics_data = material_physics.try_borrow_data()?;
    MaterialPhysicsTableView::new(&physics_data)?.validate_mass(&equipment_record)?;
    drop(physics_data);
    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_item(&mut backpack_data, owner.key, &equipment_record, clock.slot)
}

fn validate_equipment_transfer_accounts(
    program_id: &Pubkey,
    transfer_authority: &AccountInfo,
    owner: &AccountInfo,
    backpack: &AccountInfo,
    player_equipment: &AccountInfo,
    material_physics: &AccountInfo,
) -> ProgramResult {
    let (expected_authority, _) = Pubkey::find_program_address(
        &[EQUIPMENT_TRANSFER_AUTHORITY_SEED],
        &NICECHUNK_PLAYER_PROGRAM_ID,
    );
    if !transfer_authority.is_signer || transfer_authority.key != &expected_authority {
        return Err(NicechunkBackpackError::InvalidEquipmentTransferAuthority.into());
    }
    if !owner.is_signer {
        return Err(NicechunkBackpackError::InvalidPayer.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    require_key_eq(
        player_equipment.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;
    validate_material_physics_pda(program_id, material_physics)?;
    let equipment_data = player_equipment.try_borrow_data()?;
    PlayerEquipmentView::validate(&equipment_data, player_equipment.key, owner.key)
        .map_err(Into::into)
}

fn issue_blueprint(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 5 || payload.len() != 8 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let item_id = read_u64(payload, 0);
    if item_id == 0 {
        return Err(NicechunkBackpackError::InvalidBlueprintItem.into());
    }

    let account_info_iter = &mut accounts.iter();
    let issuer = next_account_info(account_info_iter)?;
    let recipient = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let blueprint_item = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    validate_blueprint_issuer(issuer)?;
    if !backpack.is_writable || !blueprint_item.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkBackpackError::InvalidSystemProgram,
    )?;
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    validate_existing_backpack_pda(program_id, backpack, recipient.key)?;

    let item_id_bytes = item_id.to_le_bytes();
    let (expected_blueprint, bump) =
        Pubkey::find_program_address(&[BLUEPRINT_ITEM_SEED, &item_id_bytes], program_id);
    require_key_eq(
        blueprint_item.key,
        &expected_blueprint,
        NicechunkBackpackError::InvalidBlueprintPda,
    )?;
    if blueprint_item.owner == program_id {
        return Err(NicechunkBackpackError::BlueprintAlreadyIssued.into());
    }
    if blueprint_item.owner != &system_program::ID || blueprint_item.data_len() != 0 {
        return Err(NicechunkBackpackError::InvalidSystemAccount.into());
    }

    create_blueprint_item_pda(
        issuer,
        blueprint_item,
        system_program_account,
        program_id,
        item_id,
        bump,
    )?;

    let clock = Clock::get()?;
    {
        let mut data = blueprint_item.try_borrow_mut_data()?;
        BlueprintItemAccount::pack(
            &mut data,
            bump,
            item_id,
            recipient.key,
            issuer.key,
            clock.slot,
        )?;
    }

    let mut record = BackpackSlotRecord {
        kind: BACKPACK_SLOT_KIND_ITEM,
        category: BACKPACK_ITEM_CATEGORY_BLUEPRINT,
        flags: BACKPACK_ITEM_FLAG_UNIQUE,
        quantity: 1,
        resource: BackpackResourceRecord::default(),
        item_code: BACKPACK_BLUEPRINT_ITEM_CODE,
        item_id,
        item_pda: *blueprint_item.key,
        volume_mm3: 1,
        durability_current: 1,
        durability_max: 1,
        grade: 1,
        item_level: 1,
        quality_bps: 10_000,
        metadata: 0,
    };
    record.set_mass_grams(0)?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_issued_item(&mut backpack_data, recipient.key, &record, clock.slot)
}

fn validate_blueprint_issuer(issuer: &AccountInfo) -> ProgramResult {
    if !issuer.is_signer || !issuer.is_writable {
        return Err(NicechunkBackpackError::InvalidBlueprintIssuer.into());
    }
    require_key_eq(
        issuer.key,
        &NICECHUNK_BLUEPRINT_ISSUER,
        NicechunkBackpackError::InvalidBlueprintIssuer,
    )
}

fn initialize_backpack(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != 9 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let backpack_id = read_u64(payload, 0);
    let capacity = payload[8].max(1).min(BACKPACK_DEFAULT_CAPACITY);

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkBackpackError::InvalidPayer.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkBackpackError::InvalidSystemProgram,
    )?;
    require_key_eq(
        player_profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate_owner(&player_profile_data, payer.key)?;
    if PlayerProfileView::has_equipped_backpack(&player_profile_data)? {
        return Err(NicechunkBackpackError::PlayerBackpackAlreadyBound.into());
    }
    drop(player_profile_data);

    let bump = validate_backpack_pda(program_id, backpack.key, payer.key, backpack_id)?;
    if backpack.owner == program_id {
        return Err(NicechunkBackpackError::BackpackAlreadyInitialized.into());
    }
    if backpack.owner != &system_program::ID || backpack.data_len() != 0 {
        return Err(NicechunkBackpackError::InvalidSystemAccount.into());
    }

    create_backpack_pda(
        payer,
        backpack,
        system_program_account,
        program_id,
        backpack_id,
        bump,
    )?;

    let clock = Clock::get()?;
    let mut data = backpack.try_borrow_mut_data()?;
    BackpackAccount::pack_empty(
        &mut data,
        &BackpackInitArgs {
            bump,
            backpack_id,
            owner: payer.key,
            capacity,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn append_mined_resource(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    const PAYLOAD_LEN: usize = BackpackResourceRecord::LEN + 4 + 8;
    if accounts.len() != 5 || payload.len() != PAYLOAD_LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let record = BackpackResourceRecord::unpack(&payload[..BackpackResourceRecord::LEN])?;
    let volume_mm3 = u32::from_le_bytes(
        payload[BackpackResourceRecord::LEN..BackpackResourceRecord::LEN + 4]
            .try_into()
            .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
    );
    let action_id = read_u64(payload, BackpackResourceRecord::LEN + 4);
    let account_info_iter = &mut accounts.iter();
    let chunk_broken = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;

    let owner = validate_chunk_reward_authority(
        program_id,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        &record,
    )?;
    validate_material_physics_pda(program_id, material_physics)?;
    let physics_data = material_physics.try_borrow_data()?;
    let physics = MaterialPhysicsTableView::new(&physics_data)?;
    let slot = BackpackSlotRecord::from_block_resource_with_volume(record, volume_mm3);
    let mass_grams = physics.block_mass_grams(slot.block_id()?, volume_mm3)?;
    drop(physics_data);

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::record_mining_action(&mut backpack_data, &owner, action_id, clock.slot)?;
    BackpackAccount::append_resource_with_volume(
        &mut backpack_data,
        &owner,
        &record,
        volume_mm3,
        mass_grams,
        clock.slot,
    )
}

fn append_mined_resources_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 || payload.len() < 9 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let count = payload[0] as usize;
    let action_id = read_u64(payload, 1);
    let volume_end = BackpackResourceRecord::LEN + 4;
    let record_size = volume_end + 4;
    if payload.len() != 9 + count * record_size {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    if count == 0 || count > state::BACKPACK_MAX_CAPACITY as usize {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let mut records = Vec::with_capacity(count);
    let mut volumes_mm3 = Vec::with_capacity(count);
    let mut metadata = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 9 + index * record_size;
        records.push(BackpackResourceRecord::unpack(
            &payload[offset..offset + BackpackResourceRecord::LEN],
        )?);
        volumes_mm3.push(u32::from_le_bytes(
            payload[offset + BackpackResourceRecord::LEN..offset + volume_end]
                .try_into()
                .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
        ));
        metadata.push(u32::from_le_bytes(
            payload[offset + volume_end..offset + record_size]
                .try_into()
                .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
        ));
    }

    let account_info_iter = &mut accounts.iter();
    let chunk_broken = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;

    let owner = validate_chunk_reward_authority(
        program_id,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        records
            .first()
            .ok_or(NicechunkBackpackError::InvalidInstruction)?,
    )?;
    for record in records.iter().skip(1) {
        validate_chunk_broken_pda_for_record(chunk_broken, global_config, record)?;
    }
    validate_material_physics_pda(program_id, material_physics)?;
    let physics_data = material_physics.try_borrow_data()?;
    let physics = MaterialPhysicsTableView::new(&physics_data)?;
    let masses_grams = records
        .iter()
        .zip(volumes_mm3.iter())
        .map(|(record, volume_mm3)| {
            let slot = BackpackSlotRecord::from_block_resource_with_volume(*record, *volume_mm3);
            physics.block_mass_grams(slot.block_id()?, *volume_mm3)
        })
        .collect::<Result<Vec<_>, NicechunkBackpackError>>()?;
    drop(physics_data);

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::record_mining_action(&mut backpack_data, &owner, action_id, clock.slot)?;
    BackpackAccount::append_resources_lossy_with_volumes_and_metadata(
        &mut backpack_data,
        &owner,
        &records,
        &volumes_mm3,
        &metadata,
        &masses_grams,
        clock.slot,
    )
}

fn remove_resource(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if payload.len() != 1 || (accounts.len() != 2 && accounts.len() != 4) {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let index = payload[0];
    let account_info_iter = &mut accounts.iter();

    if accounts.len() == 2 {
        let owner = next_account_info(account_info_iter)?;
        let backpack = next_account_info(account_info_iter)?;

        if !owner.is_signer || !owner.is_writable {
            return Err(NicechunkBackpackError::InvalidPayer.into());
        }
        if !backpack.is_writable {
            return Err(NicechunkBackpackError::InvalidWritableAccount.into());
        }
        require_key_eq(
            backpack.owner,
            program_id,
            NicechunkBackpackError::InvalidBackpackOwner,
        )?;
        validate_existing_backpack_pda(program_id, backpack, owner.key)?;

        let clock = Clock::get()?;
        let mut backpack_data = backpack.try_borrow_mut_data()?;
        return BackpackAccount::remove_resource_at(
            &mut backpack_data,
            owner.key,
            index,
            clock.slot,
        );
    }

    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;

    if !session_authority.is_signer {
        return Err(NicechunkBackpackError::InvalidSessionAuthority.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    require_key_eq(
        player_profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        player_session.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;

    let clock = Clock::get()?;
    let player_session_data = player_session.try_borrow_data()?;
    let session = PlayerSessionView::validate(
        &player_session_data,
        session_authority.key,
        player_profile.key,
        SESSION_ACTION_BREAK_BLOCK,
        clock.unix_timestamp,
    )?;
    drop(player_session_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate_owner(&player_profile_data, &session.owner)?;
    drop(player_profile_data);
    validate_existing_backpack_pda(program_id, backpack, &session.owner)?;

    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::remove_resource_at(&mut backpack_data, &session.owner, index, clock.slot)
}

fn append_market_resource(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != BackpackSlotRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let record = BackpackSlotRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let market_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;

    if !market_authority.is_signer {
        return Err(NicechunkBackpackError::InvalidMarketAuthority.into());
    }
    let (expected_authority, _) =
        Pubkey::find_program_address(&[b"market-authority"], &NICECHUNK_MARKET_PROGRAM_ID);
    require_key_eq(
        market_authority.key,
        &expected_authority,
        NicechunkBackpackError::InvalidMarketAuthority,
    )?;
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;
    validate_material_physics_pda(program_id, material_physics)?;
    let physics_data = material_physics.try_borrow_data()?;
    MaterialPhysicsTableView::new(&physics_data)?.validate_mass(&record)?;
    drop(physics_data);

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_item(&mut backpack_data, owner.key, &record, clock.slot)
}

fn append_smelting_item(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != BackpackSlotRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let mut record = BackpackSlotRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let smelting_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;

    if !smelting_authority.is_signer {
        return Err(NicechunkBackpackError::InvalidSmeltingAuthority.into());
    }
    let (expected_authority, _) =
        Pubkey::find_program_address(&[b"smelting-authority"], &NICECHUNK_SMELTING_PROGRAM_ID);
    require_key_eq(
        smelting_authority.key,
        &expected_authority,
        NicechunkBackpackError::InvalidSmeltingAuthority,
    )?;
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;
    validate_material_physics_pda(program_id, material_physics)?;
    let physics_data = material_physics.try_borrow_data()?;
    MaterialPhysicsTableView::new(&physics_data)?.apply_mass(&mut record)?;
    drop(physics_data);

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_item(&mut backpack_data, owner.key, &record, clock.slot)
}

fn remove_resources(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if payload.is_empty() || (accounts.len() != 2 && accounts.len() != 4) {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let count = payload[0] as usize;
    if count == 0 || payload.len() != count + 1 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let indexes = &payload[1..];
    let account_info_iter = &mut accounts.iter();

    if accounts.len() == 2 {
        let owner = next_account_info(account_info_iter)?;
        let backpack = next_account_info(account_info_iter)?;

        if !owner.is_signer || !owner.is_writable {
            return Err(NicechunkBackpackError::InvalidPayer.into());
        }
        if !backpack.is_writable {
            return Err(NicechunkBackpackError::InvalidWritableAccount.into());
        }
        require_key_eq(
            backpack.owner,
            program_id,
            NicechunkBackpackError::InvalidBackpackOwner,
        )?;
        validate_existing_backpack_pda(program_id, backpack, owner.key)?;

        let clock = Clock::get()?;
        let mut backpack_data = backpack.try_borrow_mut_data()?;
        return BackpackAccount::remove_resources_at(
            &mut backpack_data,
            owner.key,
            indexes,
            clock.slot,
        );
    }

    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;

    if !session_authority.is_signer {
        return Err(NicechunkBackpackError::InvalidSessionAuthority.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;
    require_key_eq(
        player_profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        player_session.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;

    let clock = Clock::get()?;
    let player_session_data = player_session.try_borrow_data()?;
    let session = PlayerSessionView::validate(
        &player_session_data,
        session_authority.key,
        player_profile.key,
        SESSION_ACTION_BREAK_BLOCK,
        clock.unix_timestamp,
    )?;
    drop(player_session_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate_owner(&player_profile_data, &session.owner)?;
    drop(player_profile_data);
    validate_existing_backpack_pda(program_id, backpack, &session.owner)?;

    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::remove_resources_at(&mut backpack_data, &session.owner, indexes, clock.slot)
}

fn forge_equipment_with_material_verification(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    const HEADER_LEN: usize = 11;
    if accounts.len() != 7 || payload.len() < HEADER_LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let item_id = read_u64(payload, 0);
    let code_len = read_u16(payload, 8) as usize;
    let input_count = payload[10] as usize;
    if input_count == 0
        || input_count > state::MAX_FORGING_INPUTS
        || code_len == 0
        || code_len > MAX_VERIFIED_FORGE_CODE_BYTES
        || payload.len() != HEADER_LEN + code_len + input_count
    {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let code = &payload[HEADER_LEN..HEADER_LEN + code_len];
    let indexes = &payload[HEADER_LEN + code_len..];
    let (design_hash, requirements) = verified_forge_design(code)?;

    process_forge_equipment(
        program_id,
        accounts,
        item_id,
        design_hash,
        code,
        indexes,
        requirements,
    )
}

fn process_forge_equipment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    item_id: u64,
    design_hash: u32,
    code: &[u8],
    indexes: &[u8],
    requirements: ForgeMaterialRequirements,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let forged_item = next_account_info(account_info_iter)?;
    let player_program = next_account_info(account_info_iter)?;
    let player_skills = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkBackpackError::InvalidPayer.into());
    }
    if !player_profile.is_writable || !backpack.is_writable || !forged_item.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        player_program.key,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkBackpackError::InvalidSystemProgram,
    )?;
    require_key_eq(
        player_profile.owner,
        player_program.key,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;

    let global_config = {
        let player_profile_data = player_profile.try_borrow_data()?;
        PlayerProfileView::validate_owner(&player_profile_data, owner.key)?;
        PlayerProfileView::owner_and_global_config(&player_profile_data)?.1
    };
    let forging_level = player_skill_level(player_skills, &global_config, owner.key)?;
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;

    let clock = Clock::get()?;
    let item_id_bytes = item_id.to_le_bytes();
    let (expected_forged_item, forged_item_bump) = Pubkey::find_program_address(
        &[FORGED_ITEM_SEED, owner.key.as_ref(), &item_id_bytes],
        program_id,
    );
    require_key_eq(
        forged_item.key,
        &expected_forged_item,
        NicechunkBackpackError::InvalidForgedItemPda,
    )?;
    if forged_item.owner == program_id {
        return Err(NicechunkBackpackError::ForgedItemAlreadyInitialized.into());
    }
    if forged_item.owner != &system_program::ID || forged_item.data_len() != 0 {
        return Err(NicechunkBackpackError::InvalidSystemAccount.into());
    }
    create_forged_item_pda(
        owner,
        forged_item,
        system_program_account,
        program_id,
        item_id,
        forged_item_bump,
    )?;
    {
        let mut forged_item_data = forged_item.try_borrow_mut_data()?;
        ForgedItemAccount::pack(
            &mut forged_item_data,
            &ForgedItemInitArgs {
                bump: forged_item_bump,
                item_id,
                creator: owner.key,
                origin_backpack: backpack.key,
                design_hash,
                code,
                created_slot: clock.slot,
                created_at: clock.unix_timestamp,
            },
        )?;
    }
    let outcome = {
        let mut backpack_data = backpack.try_borrow_mut_data()?;
        BackpackAccount::forge_equipment_from_verified_materials(
            &mut backpack_data,
            owner.key,
            indexes,
            item_id,
            design_hash,
            forged_item.key,
            forging_level,
            clock.slot,
            requirements,
        )?
    };

    add_forging_xp_to_player(
        program_id,
        owner,
        backpack,
        player_profile,
        player_program,
        system_program_account,
        &outcome,
    )
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
        NicechunkBackpackError::InvalidPlayerSkillsPda,
    )?;
    if player_skills.owner == &system_program::ID && player_skills.data_len() == 0 {
        return Ok(0);
    }
    require_key_eq(
        player_skills.owner,
        &NICECHUNK_SKILLS_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerSkillsOwner,
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
        return Err(NicechunkBackpackError::InvalidPlayerSkillsData.into());
    }
    let level = data[PLAYER_SKILLS_LEVELS_OFFSET + FORGING_SKILL_INDEX];
    if level > MAX_SKILL_LEVEL {
        return Err(NicechunkBackpackError::InvalidPlayerSkillsData.into());
    }
    Ok(level)
}

fn create_forged_item_pda<'a>(
    payer: &AccountInfo<'a>,
    forged_item: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    item_id: u64,
    bump: u8,
) -> ProgramResult {
    let item_id_bytes = item_id.to_le_bytes();
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[
        FORGED_ITEM_SEED,
        payer.key.as_ref(),
        &item_id_bytes,
        &bump_seed,
    ];
    let rent = Rent::get()?;
    let create = system_instruction::create_account(
        payer.key,
        forged_item.key,
        rent.minimum_balance(ForgedItemAccount::LEN),
        ForgedItemAccount::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            payer.clone(),
            forged_item.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )
}

fn validate_backpack_pda(
    program_id: &Pubkey,
    backpack: &Pubkey,
    creator: &Pubkey,
    backpack_id: u64,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let backpack_id_bytes = backpack_id.to_le_bytes();
    let (expected_backpack, bump) = Pubkey::find_program_address(
        &[BACKPACK_SEED, creator.as_ref(), &backpack_id_bytes],
        program_id,
    );
    require_key_eq(
        backpack,
        &expected_backpack,
        NicechunkBackpackError::InvalidBackpackPda,
    )?;
    Ok(bump)
}

fn validate_global_config(global_config: &AccountInfo) -> ProgramResult {
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkBackpackError::InvalidGlobalConfig,
    )?;
    let (expected, _) =
        Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], &NICECHUNK_CORE_PROGRAM_ID);
    require_key_eq(
        global_config.key,
        &expected,
        NicechunkBackpackError::InvalidGlobalConfig,
    )?;
    let data = global_config.try_borrow_data()?;
    if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
        return Err(NicechunkBackpackError::InvalidGlobalConfig.into());
    }
    Ok(())
}

fn validate_material_physics_pda(
    program_id: &Pubkey,
    material_physics: &AccountInfo,
) -> ProgramResult {
    require_key_eq(
        material_physics.owner,
        program_id,
        NicechunkBackpackError::InvalidMaterialPhysicsPda,
    )?;
    let (global_config, _) =
        Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], &NICECHUNK_CORE_PROGRAM_ID);
    let (expected, bump) =
        Pubkey::find_program_address(&[MATERIAL_PHYSICS_SEED, global_config.as_ref()], program_id);
    require_key_eq(
        material_physics.key,
        &expected,
        NicechunkBackpackError::InvalidMaterialPhysicsPda,
    )?;
    let data = material_physics.try_borrow_data()?;
    MaterialPhysicsTableState::validate_header(&data)?;
    if data.get(9).copied() != Some(bump) {
        return Err(NicechunkBackpackError::InvalidMaterialPhysicsData.into());
    }
    Ok(())
}

fn validate_chunk_reward_authority(
    program_id: &Pubkey,
    chunk_broken: &AccountInfo,
    global_config: &AccountInfo,
    player_profile: &AccountInfo,
    backpack: &AccountInfo,
    record: &BackpackResourceRecord,
) -> Result<Pubkey, solana_program::program_error::ProgramError> {
    if !chunk_broken.is_signer {
        return Err(NicechunkBackpackError::InvalidChunkAuthority.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    require_key_eq(
        chunk_broken.owner,
        &NICECHUNK_CHUNK_PROGRAM_ID,
        NicechunkBackpackError::InvalidChunkAuthority,
    )?;
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkBackpackError::InvalidGlobalConfig,
    )?;
    require_key_eq(
        player_profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkBackpackError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        backpack.owner,
        program_id,
        NicechunkBackpackError::InvalidBackpackOwner,
    )?;

    let player_profile_data = player_profile.try_borrow_data()?;
    let (owner, profile_global_config) =
        PlayerProfileView::owner_and_global_config(&player_profile_data)?;
    drop(player_profile_data);
    require_key_eq(
        global_config.key,
        &profile_global_config,
        NicechunkBackpackError::InvalidGlobalConfig,
    )?;

    validate_chunk_broken_pda_for_record(chunk_broken, global_config, record)?;
    validate_existing_backpack_pda(program_id, backpack, &owner)?;
    Ok(owner)
}

fn validate_existing_backpack_pda(
    program_id: &Pubkey,
    backpack: &AccountInfo,
    owner: &Pubkey,
) -> ProgramResult {
    let data = backpack.try_borrow_data()?;
    BackpackAccount::validate_owner(&data, owner)?;
    let backpack_id = read_u64(&data, BackpackAccount::BACKPACK_ID_OFFSET);
    drop(data);
    validate_backpack_pda(program_id, backpack.key, owner, backpack_id)?;
    Ok(())
}

fn validate_chunk_broken_pda_for_record(
    chunk_broken: &AccountInfo,
    global_config: &AccountInfo,
    record: &BackpackResourceRecord,
) -> ProgramResult {
    let chunk_size = {
        let data = global_config.try_borrow_data()?;
        if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
            return Err(NicechunkBackpackError::InvalidGlobalConfig.into());
        }
        let chunk_size = read_u16(&data, GLOBAL_CONFIG_CHUNK_SIZE_OFFSET) as i32;
        if chunk_size <= 0 {
            return Err(NicechunkBackpackError::InvalidGlobalConfig.into());
        }
        chunk_size
    };
    let chunk_x = record.world_x.div_euclid(chunk_size);
    let chunk_z = record.world_z.div_euclid(chunk_size);
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let (expected, bump) = Pubkey::find_program_address(
        &[
            CHUNK_BROKEN_SEED,
            global_config.key.as_ref(),
            &chunk_x_bytes,
            &chunk_z_bytes,
        ],
        &NICECHUNK_CHUNK_PROGRAM_ID,
    );
    require_key_eq(
        chunk_broken.key,
        &expected,
        NicechunkBackpackError::InvalidChunkAuthority,
    )?;

    let data = chunk_broken.try_borrow_data()?;
    if data.len() < 6
        || data[0..4] != CHUNK_BROKEN_MAGIC
        || data[4] != CHUNK_BROKEN_VERSION
        || data[5] != bump
    {
        return Err(NicechunkBackpackError::InvalidChunkAuthority.into());
    }
    Ok(())
}

fn create_backpack_pda<'a>(
    payer: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    backpack_id: u64,
    bump: u8,
) -> ProgramResult {
    let backpack_id_bytes = backpack_id.to_le_bytes();
    let seeds = &[
        BACKPACK_SEED,
        payer.key.as_ref(),
        &backpack_id_bytes,
        &[bump],
    ];
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(BackpackAccount::LEN);
    let create = system_instruction::create_account(
        payer.key,
        backpack.key,
        lamports,
        BackpackAccount::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            payer.clone(),
            backpack.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )
}

fn create_material_physics_pda<'a>(
    authority: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    global_config: &Pubkey,
    bump: u8,
) -> ProgramResult {
    let seeds = &[MATERIAL_PHYSICS_SEED, global_config.as_ref(), &[bump]];
    let rent = Rent::get()?;
    let create = system_instruction::create_account(
        authority.key,
        material_physics.key,
        rent.minimum_balance(MaterialPhysicsTableState::LEN),
        MaterialPhysicsTableState::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            authority.clone(),
            material_physics.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )
}

fn create_blueprint_item_pda<'a>(
    issuer: &AccountInfo<'a>,
    blueprint_item: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    item_id: u64,
    bump: u8,
) -> ProgramResult {
    let item_id_bytes = item_id.to_le_bytes();
    let seeds = &[BLUEPRINT_ITEM_SEED, &item_id_bytes, &[bump]];
    let rent = Rent::get()?;
    let create = system_instruction::create_account(
        issuer.key,
        blueprint_item.key,
        rent.minimum_balance(BlueprintItemAccount::LEN),
        BlueprintItemAccount::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            issuer.clone(),
            blueprint_item.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )
}

fn add_forging_xp_to_player<'a>(
    program_id: &Pubkey,
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    player_program: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    outcome: &state::ForgeOutcome,
) -> ProgramResult {
    let (backpack_id, bump) = {
        let data = backpack.try_borrow_data()?;
        BackpackAccount::validate_owner(&data, owner.key)?;
        (
            read_u64(&data, BackpackAccount::BACKPACK_ID_OFFSET),
            data[10],
        )
    };
    validate_backpack_pda(program_id, backpack.key, owner.key, backpack_id)?;
    let backpack_id_bytes = backpack_id.to_le_bytes();
    let seeds = &[
        BACKPACK_SEED,
        owner.key.as_ref(),
        &backpack_id_bytes,
        &[bump],
    ];
    let mut data = Vec::with_capacity(11);
    data.push(6);
    data.push(outcome.grade);
    data.push(outcome.item_level);
    data.extend_from_slice(&outcome.gained_xp.to_le_bytes());
    let ix = Instruction {
        program_id: *player_program.key,
        accounts: vec![
            AccountMeta::new(*owner.key, true),
            AccountMeta::new_readonly(*backpack.key, true),
            AccountMeta::new(*player_profile.key, false),
            AccountMeta::new_readonly(*system_program_account.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[
            owner.clone(),
            backpack.clone(),
            player_profile.clone(),
            system_program_account.clone(),
            player_program.clone(),
        ],
        &[seeds],
    )
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
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

#[cfg(test)]
mod instruction_tests {
    use super::*;
    use solana_program::program_error::ProgramError;

    #[test]
    fn unverified_legacy_forge_instruction_is_disabled() {
        let error = process_instruction(&Pubkey::new_unique(), &[], &[7]).unwrap_err();
        assert!(matches!(
            error,
            ProgramError::Custom(code)
                if code == NicechunkBackpackError::UnverifiedForgeInstructionDisabled as u32
        ));
    }
}
