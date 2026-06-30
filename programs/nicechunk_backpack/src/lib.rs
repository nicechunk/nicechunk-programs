#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
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
    NICECHUNK_MARKET_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID, NICECHUNK_SMELTING_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkBackpackError};
use state::{
    BackpackAccount, BackpackInitArgs, BackpackResourceRecord, BackpackSlotRecord,
    PlayerProfileView, PlayerSessionView, BACKPACK_DEFAULT_CAPACITY, BACKPACK_SEED,
    SESSION_ACTION_BREAK_BLOCK,
};

declare_id!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");

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
        _ => Err(NicechunkBackpackError::InvalidInstruction.into()),
    }
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
    if accounts.len() != 4 || payload.len() != BackpackResourceRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let record = BackpackResourceRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
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

    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_resource(&mut backpack_data, &session.owner, &record, clock.slot)
}

fn append_mined_resources_batch(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.is_empty() {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }
    let count = payload[0] as usize;
    if count == 0
        || count > state::BACKPACK_MAX_CAPACITY as usize
        || payload.len() != 1 + count * BackpackResourceRecord::LEN
    {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 1 + index * BackpackResourceRecord::LEN;
        records.push(BackpackResourceRecord::unpack(
            &payload[offset..offset + BackpackResourceRecord::LEN],
        )?);
    }

    let account_info_iter = &mut accounts.iter();
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

    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_resources_lossy(
        &mut backpack_data,
        &session.owner,
        &records,
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

    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::remove_resource_at(&mut backpack_data, &session.owner, index, clock.slot)
}

fn append_market_resource(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != BackpackResourceRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let record = BackpackResourceRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let market_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;

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

    let clock = Clock::get()?;
    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::append_resource(&mut backpack_data, owner.key, &record, clock.slot)
}

fn append_smelting_item(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != BackpackSlotRecord::LEN {
        return Err(NicechunkBackpackError::InvalidInstruction.into());
    }

    let record = BackpackSlotRecord::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let smelting_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;

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

    let mut backpack_data = backpack.try_borrow_mut_data()?;
    BackpackAccount::remove_resources_at(&mut backpack_data, &session.owner, indexes, clock.slot)
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
