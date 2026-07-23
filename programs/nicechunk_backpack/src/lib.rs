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
    NICECHUNK_SMELTING_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkBackpackError};
use state::{
    verified_forge_design, BackpackAccount, BackpackInitArgs, BackpackResourceRecord,
    BackpackSlotRecord, BlueprintItemAccount, ForgeMaterialRequirements, MaterialPhysicsRecord,
    MaterialPhysicsState, PlayerEquipmentView, PlayerProfileView, PlayerSessionView,
    BACKPACK_BLUEPRINT_ITEM_CODE, BACKPACK_DEFAULT_CAPACITY, BACKPACK_ITEM_CATEGORY_BLUEPRINT,
    BACKPACK_ITEM_FLAG_UNIQUE, BACKPACK_PACKED_Y_BITS, BACKPACK_SEED, BACKPACK_SLOT_KIND_ITEM,
    BLUEPRINT_ITEM_SEED, EQUIPMENT_TRANSFER_AUTHORITY_SEED, MATERIAL_PHYSICS_MAX_RECORDS,
    MATERIAL_PHYSICS_SEED, MAX_VERIFIED_FORGE_CODE_BYTES, SESSION_ACTION_BREAK_BLOCK,
};

declare_id!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");

const CHUNK_BROKEN_MAGIC: [u8; 4] = *b"NCBK";
const CHUNK_BROKEN_VERSION: u8 = 1;
const CHUNK_BROKEN_SEED: &[u8] = b"chunk-broken";
const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
const GLOBAL_CONFIG_TREASURY_OFFSET: usize = 53;
const GLOBAL_CONFIG_CHUNK_SIZE_OFFSET: usize = 259;
const GLOBAL_CONFIG_LEN: usize = 293;

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
        12 => initialize_material_physics(program_id, accounts, payload),
        13 => replace_material_physics(program_id, accounts, payload),
        14 => migrate_backpack_mass(program_id, accounts, payload),
        _ => Err(NicechunkBackpackError::InvalidInstruction.into()),
    }
}

fn initialize_material_physics(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || !payload.is_empty() {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
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
    let expected_treasury = validate_global_config_treasury(global_config)?;
    validate_material_physics_authority(authority.key, &expected_treasury, true)?;
    let (expected_physics, bump) = Pubkey::find_program_address(
        &[MATERIAL_PHYSICS_SEED, global_config.key.as_ref()],
        program_id,
    );
    require_key_eq(
        material_physics.key,
        &expected_physics,
        NicechunkBackpackError::InvalidMaterialPhysicsData,
    )?;
    if material_physics.owner == program_id {
        return Err(NicechunkBackpackError::InvalidMaterialPhysicsData.into());
    }
    create_material_physics_pda(
        authority,
        material_physics,
        global_config,
        system_program_account,
        program_id,
        bump,
    )?;
    let clock = Clock::get()?;
    let mut data = material_physics.try_borrow_mut_data()?;
    MaterialPhysicsState::pack_empty(
        &mut data,
        bump,
        &expected_treasury,
        global_config.key,
        clock.slot,
        clock.unix_timestamp,
    )
}

fn replace_material_physics(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.is_empty() {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let record_count = payload[0] as usize;
    if record_count == 0
        || record_count > MATERIAL_PHYSICS_MAX_RECORDS
        || payload.len() != 1 + record_count * 4
    {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    if !authority.is_signer {
        return Err(NicechunkBackpackError::InvalidMaterialPhysicsAuthority.into());
    }
    if !material_physics.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    let expected_treasury = validate_global_config_treasury(global_config)?;
    validate_material_physics_pda(program_id, material_physics, global_config)?;
    let bootstrap_allowed = {
        let data = material_physics.try_borrow_data()?;
        let state = MaterialPhysicsState::validate(&data, global_config.key)?;
        state.revision == 0 && state.record_count == 0
    };
    validate_material_physics_authority(authority.key, &expected_treasury, bootstrap_allowed)?;

    let mut records = Vec::with_capacity(record_count);
    for index in 0..record_count {
        let offset = 1 + index * 4;
        records.push(MaterialPhysicsRecord {
            material_id: read_u16(payload, offset),
            density_kg_m3: read_u16(payload, offset + 2),
        });
    }
    let clock = Clock::get()?;
    let mut data = material_physics.try_borrow_mut_data()?;
    MaterialPhysicsState::replace_records(
        &mut data,
        global_config.key,
        &expected_treasury,
        &records,
        clock.slot,
    )
}

fn migrate_backpack_mass(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || !payload.is_empty() {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    if !owner.is_signer {
        return Err(NicechunkBackpackError::InvalidPayer.into());
    }
    if !backpack.is_writable {
        return Err(NicechunkBackpackError::InvalidWritableAccount.into());
    }
    validate_global_config_treasury(global_config)?;
    validate_material_physics_pda(program_id, material_physics, global_config)?;
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;
    let physics_data = material_physics.try_borrow_data()?;
    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::migrate_mass(
        &mut backpack_data,
        owner.key,
        &physics_data,
        global_config.key,
        clock.slot,
    )
}

fn transfer_backpack_item_to_equipment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 6 || payload.len() != 2 {
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
    let global_config = next_account_info(account_info_iter)?;

    validate_equipment_transfer_accounts(
        program_id,
        transfer_authority,
        owner,
        backpack,
        player_equipment,
    )?;
    let mut previous_equipment = {
        let equipment_data = player_equipment.try_borrow_data()?;
        PlayerEquipmentView::custodied_slot(&equipment_data, equipment_slot)?
    };
    if let Some(record) = previous_equipment.as_mut() {
        ensure_record_mass(program_id, material_physics, global_config, record, true)?;
    }
    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    if let Some(previous) = previous_equipment {
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
    if accounts.len() != 6 || payload.len() != 1 {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let equipment_slot = payload[0];
    let account_info_iter = &mut accounts.iter();
    let transfer_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    validate_equipment_transfer_accounts(
        program_id,
        transfer_authority,
        owner,
        backpack,
        player_equipment,
    )?;
    let mut equipment_record = {
        let equipment_data = player_equipment.try_borrow_data()?;
        PlayerEquipmentView::custodied_slot(&equipment_data, equipment_slot)?
            .ok_or(NicechunkBackpackError::EquipmentSlotEmpty)?
    };
    ensure_record_mass(
        program_id,
        material_physics,
        global_config,
        &mut equipment_record,
        true,
    )?;
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
    record.set_mass_grams(0);
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
    validate_material_physics_pda(program_id, material_physics, global_config)?;
    let physics_data = material_physics.try_borrow_data()?;
    let mass_grams = mined_resource_mass(&physics_data, global_config.key, &record, volume_mm3)?;

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::record_mining_action(&mut backpack_data, &owner, action_id, clock.slot)?;
    BackpackAccount::append_resource_with_volume_metadata_and_mass(
        &mut backpack_data,
        &owner,
        &record,
        volume_mm3,
        0,
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
    let legacy_record_size = BackpackResourceRecord::LEN + 4;
    let metadata_record_size = legacy_record_size + 4;
    let record_size = match payload.len() {
        len if len == 9 + count * metadata_record_size => metadata_record_size,
        len if len == 9 + count * legacy_record_size => legacy_record_size,
        _ => return Err(NicechunkBackpackError::InvalidInstruction.into()),
    };
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
            payload[offset + BackpackResourceRecord::LEN..offset + legacy_record_size]
                .try_into()
                .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
        ));
        metadata.push(if record_size == metadata_record_size {
            u32::from_le_bytes(
                payload[offset + legacy_record_size..offset + metadata_record_size]
                    .try_into()
                    .map_err(|_| NicechunkBackpackError::InvalidInstruction)?,
            )
        } else {
            0
        });
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
    validate_material_physics_pda(program_id, material_physics, global_config)?;
    let physics_data = material_physics.try_borrow_data()?;
    let masses_grams = records
        .iter()
        .zip(volumes_mm3.iter())
        .map(|(record, volume)| {
            mined_resource_mass(&physics_data, global_config.key, record, *volume)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::record_mining_action(&mut backpack_data, &owner, action_id, clock.slot)?;
    BackpackAccount::append_resources_lossy_with_physics(
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
    if accounts.len() != 5 || payload.len() != BackpackSlotRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let mut record = BackpackSlotRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let market_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

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
    ensure_record_mass(
        program_id,
        material_physics,
        global_config,
        &mut record,
        true,
    )?;

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_item(&mut backpack_data, owner.key, &record, clock.slot)
}

fn append_smelting_item(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 || payload.len() != BackpackSlotRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let mut record = BackpackSlotRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let smelting_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

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
    ensure_record_mass(
        program_id,
        material_physics,
        global_config,
        &mut record,
        false,
    )?;

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
    if accounts.len() != 5 || payload.len() < HEADER_LEN {
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
        indexes,
        requirements,
    )
}

fn process_forge_equipment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    item_id: u64,
    design_hash: u32,
    indexes: &[u8],
    requirements: ForgeMaterialRequirements,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let player_program = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkBackpackError::InvalidPayer.into());
    }
    if !player_profile.is_writable || !backpack.is_writable {
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

    let forging_level = {
        let player_profile_data = player_profile.try_borrow_data()?;
        PlayerProfileView::validate_owner(&player_profile_data, owner.key)?;
        PlayerProfileView::forging_level(&player_profile_data)?
    };
    validate_existing_backpack_pda(program_id, backpack, owner.key)?;

    let clock = Clock::get()?;
    let outcome = {
        let mut backpack_data = backpack.try_borrow_mut_data()?;
        BackpackAccount::forge_equipment_from_verified_materials(
            &mut backpack_data,
            owner.key,
            indexes,
            item_id,
            design_hash,
            player_profile.key,
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

fn validate_global_config_treasury(
    global_config: &AccountInfo,
) -> Result<Pubkey, solana_program::program_error::ProgramError> {
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkBackpackError::InvalidGlobalConfig,
    )?;
    let data = global_config.try_borrow_data()?;
    if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
        return Err(NicechunkBackpackError::InvalidGlobalConfig.into());
    }
    let bytes: [u8; 32] = data[GLOBAL_CONFIG_TREASURY_OFFSET..GLOBAL_CONFIG_TREASURY_OFFSET + 32]
        .try_into()
        .map_err(|_| NicechunkBackpackError::InvalidGlobalConfig)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn validate_material_physics_pda(
    program_id: &Pubkey,
    material_physics: &AccountInfo,
    global_config: &AccountInfo,
) -> ProgramResult {
    require_key_eq(
        material_physics.owner,
        program_id,
        NicechunkBackpackError::InvalidMaterialPhysicsData,
    )?;
    let (expected, _) = Pubkey::find_program_address(
        &[MATERIAL_PHYSICS_SEED, global_config.key.as_ref()],
        program_id,
    );
    require_key_eq(
        material_physics.key,
        &expected,
        NicechunkBackpackError::InvalidMaterialPhysicsData,
    )?;
    let data = material_physics.try_borrow_data()?;
    MaterialPhysicsState::validate(&data, global_config.key)?;
    Ok(())
}

fn validate_material_physics_authority(
    authority: &Pubkey,
    treasury: &Pubkey,
    bootstrap_allowed: bool,
) -> ProgramResult {
    if authority == treasury || (bootstrap_allowed && authority == &NICECHUNK_BOOTSTRAP_AUTHORITY) {
        return Ok(());
    }
    Err(NicechunkBackpackError::InvalidMaterialPhysicsAuthority.into())
}

fn mined_resource_mass(
    physics_data: &[u8],
    global_config: &Pubkey,
    record: &BackpackResourceRecord,
    volume_mm3: u32,
) -> Result<u32, solana_program::program_error::ProgramError> {
    if record.world_y < 0 {
        return Err(NicechunkBackpackError::MissingMaterialPhysicsRecord.into());
    }
    let material_id = (record.world_y as u16) >> BACKPACK_PACKED_Y_BITS;
    MaterialPhysicsState::mass_grams(physics_data, global_config, material_id, volume_mm3)
        .map_err(Into::into)
}

fn ensure_record_mass(
    program_id: &Pubkey,
    material_physics: &AccountInfo,
    global_config: &AccountInfo,
    record: &mut BackpackSlotRecord,
    preserve_valid_mass: bool,
) -> ProgramResult {
    validate_global_config_treasury(global_config)?;
    validate_material_physics_pda(program_id, material_physics, global_config)?;
    if preserve_valid_mass && record.has_valid_mass() {
        return Ok(());
    }
    let physics_data = material_physics.try_borrow_data()?;
    let mass_grams = record.derived_mass_grams(&physics_data, global_config.key)?;
    record.set_mass_grams(mass_grams);
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
    payer: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    bump: u8,
) -> ProgramResult {
    if material_physics.owner != &system_program::ID || material_physics.data_len() != 0 {
        return Err(NicechunkBackpackError::InvalidSystemAccount.into());
    }
    let seeds = &[MATERIAL_PHYSICS_SEED, global_config.key.as_ref(), &[bump]];
    let rent = Rent::get()?;
    let create = system_instruction::create_account(
        payer.key,
        material_physics.key,
        rent.minimum_balance(MaterialPhysicsState::LEN),
        MaterialPhysicsState::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            payer.clone(),
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
        ],
        &[seeds],
    )
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
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

    #[test]
    fn material_physics_bootstrap_is_one_time_only() {
        let treasury = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();

        assert!(validate_material_physics_authority(&treasury, &treasury, false).is_ok());
        assert!(validate_material_physics_authority(
            &NICECHUNK_BOOTSTRAP_AUTHORITY,
            &treasury,
            true,
        )
        .is_ok());
        assert!(validate_material_physics_authority(
            &NICECHUNK_BOOTSTRAP_AUTHORITY,
            &treasury,
            false,
        )
        .is_err());
        assert!(validate_material_physics_authority(&stranger, &treasury, true).is_err());
    }
}
