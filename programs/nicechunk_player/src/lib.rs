#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
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

use cluster_config::{
    NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_GAME_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkPlayerError};
use state::{
    BackpackAccountView, GlobalConfigView, PlayerProfile, PlayerSession, PlayerSessionInitArgs,
    PLAYER_PROFILE_SEED, PLAYER_SESSION_SEED, SESSION_ACTION_BREAK_BLOCK,
    SESSION_ACTION_PLACE_BLOCK,
};

declare_id!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkPlayerError::InvalidInstruction)?;

    match tag {
        0 => initialize_player(program_id, accounts),
        1 => update_position(program_id, accounts, payload),
        2 => set_equipment_slot(program_id, accounts, payload),
        3 => set_backpack_style(program_id, accounts, payload),
        4 => create_or_refresh_player_session(program_id, accounts, payload),
        5 => set_equipped_backpack(program_id, accounts),
        _ => Err(NicechunkPlayerError::InvalidInstruction.into()),
    }
}

fn initialize_player(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkPlayerError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkPlayerError::InvalidPayer.into());
    }
    if !player_profile.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkPlayerError::InvalidSystemProgram,
    )?;
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkPlayerError::InvalidGlobalConfigOwner,
    )?;

    let (expected_player_profile, bump) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, payer.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_player_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;

    if player_profile.owner == program_id {
        return Err(NicechunkPlayerError::PlayerProfileAlreadyInitialized.into());
    }
    if player_profile.owner != &system_program::ID || player_profile.data_len() != 0 {
        return Err(NicechunkPlayerError::InvalidSystemAccount.into());
    }

    let global_config_data = global_config.try_borrow_data()?;
    let global_config_view = GlobalConfigView::unpack(&global_config_data)?;
    drop(global_config_data);

    create_or_allocate_player_profile_pda(
        payer,
        player_profile,
        system_program_account,
        program_id,
        payer.key,
        bump,
    )?;

    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::pack_default(
        &mut data,
        bump,
        payer.key,
        global_config.key,
        global_config_view.world_id,
        clock.slot,
        clock.unix_timestamp,
    )?;

    Ok(())
}

fn update_position(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 12 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    let x = read_i32(payload, 0);
    let y = read_i32(payload, 4);
    let z = read_i32(payload, 8);

    let global_config_data = global_config.try_borrow_data()?;
    let global_config_view = GlobalConfigView::unpack(&global_config_data)?;
    if y < global_config_view.min_build_y as i32 || y > global_config_view.max_build_y as i32 {
        return Err(NicechunkPlayerError::InvalidWorldBounds.into());
    }
    drop(global_config_data);

    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_position(&mut data, x, y, z, clock.slot)
}

fn set_equipment_slot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 33 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    let slot = payload[0];
    let item = Pubkey::new_from_array(payload[1..33].try_into().unwrap());
    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipment_slot(&mut data, slot, &item, clock.slot)
}

fn set_backpack_style(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 1 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_backpack_style(&mut data, payload[0], clock.slot)
}

fn create_or_refresh_player_session(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 6 || payload.len() != 14 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkPlayerError::InvalidPayer.into());
    }
    if !session_authority.is_signer {
        return Err(NicechunkPlayerError::InvalidSessionAuthority.into());
    }
    if !player_session.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkPlayerError::InvalidSystemProgram,
    )?;
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkPlayerError::InvalidGlobalConfigOwner,
    )?;
    require_key_eq(
        player_profile.owner,
        program_id,
        NicechunkPlayerError::InvalidPlayerProfileOwner,
    )?;

    let expires_at = read_i64(payload, 0);
    let allowed_actions = read_u16(payload, 8);
    let max_actions = read_u32(payload, 10);
    let allowed_mask = SESSION_ACTION_BREAK_BLOCK | SESSION_ACTION_PLACE_BLOCK;
    if allowed_actions == 0 || allowed_actions & !allowed_mask != 0 || max_actions == 0 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let clock = Clock::get()?;
    if expires_at <= clock.unix_timestamp {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let (expected_player_profile, _) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_player_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;
    let (expected_player_session, bump) = Pubkey::find_program_address(
        &[
            PLAYER_SESSION_SEED,
            owner.key.as_ref(),
            session_authority.key.as_ref(),
        ],
        program_id,
    );
    require_key_eq(
        player_session.key,
        &expected_player_session,
        NicechunkPlayerError::InvalidPlayerSessionPda,
    )?;

    let global_config_data = global_config.try_borrow_data()?;
    let global_config_view = GlobalConfigView::unpack(&global_config_data)?;
    drop(global_config_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfile::validate_owner_and_config(&player_profile_data, owner.key, global_config.key)?;
    drop(player_profile_data);

    if player_session.owner == program_id {
        let mut data = player_session.try_borrow_mut_data()?;
        return PlayerSession::refresh(
            &mut data,
            owner.key,
            session_authority.key,
            player_profile.key,
            global_config.key,
            allowed_actions,
            expires_at,
            max_actions,
            clock.slot,
        );
    }
    if player_session.owner != &system_program::ID || player_session.data_len() != 0 {
        return Err(NicechunkPlayerError::InvalidSystemAccount.into());
    }

    create_or_allocate_player_session_pda(
        owner,
        player_session,
        system_program_account,
        program_id,
        session_authority.key,
        bump,
    )?;

    let mut data = player_session.try_borrow_mut_data()?;
    PlayerSession::pack(
        &mut data,
        &PlayerSessionInitArgs {
            bump,
            owner: owner.key,
            session_authority: session_authority.key,
            player_profile: player_profile.key,
            global_config: global_config.key,
            world_id: global_config_view.world_id,
            allowed_actions,
            expires_at,
            max_actions,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn set_equipped_backpack(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkPlayerError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !authority.is_signer || !authority.is_writable {
        return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
    }
    if !player_profile.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkPlayerError::InvalidSystemProgram,
    )?;
    require_key_eq(
        player_profile.owner,
        program_id,
        NicechunkPlayerError::InvalidPlayerProfileOwner,
    )?;
    if backpack.owner != &NICECHUNK_BACKPACK_PROGRAM_ID
        && backpack.owner != &NICECHUNK_GAME_PROGRAM_ID
    {
        return Err(NicechunkPlayerError::InvalidBackpackProgram.into());
    }

    let (expected_player_profile, _) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, authority.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_player_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;

    {
        let data = player_profile.try_borrow_data()?;
        PlayerProfile::validate_owner(&data, authority.key)?;
    }

    {
        let backpack_data = backpack.try_borrow_data()?;
        BackpackAccountView::validate_owner(&backpack_data, authority.key)?;
    }

    extend_player_profile_if_needed(authority, player_profile, system_program_account)?;

    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipped_backpack(&mut data, backpack.key, clock.slot)
}

fn validate_player_write_accounts(
    program_id: &Pubkey,
    authority: &AccountInfo,
    player_profile: &AccountInfo,
    global_config: &AccountInfo,
) -> ProgramResult {
    if !authority.is_signer {
        return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
    }
    if !player_profile.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkPlayerError::InvalidGlobalConfigOwner,
    )?;
    require_key_eq(
        player_profile.owner,
        program_id,
        NicechunkPlayerError::InvalidPlayerProfileOwner,
    )?;

    let (expected_player_profile, _) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, authority.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_player_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;

    let data = player_profile.try_borrow_data()?;
    PlayerProfile::validate_owner_and_config(&data, authority.key, global_config.key)
}

fn create_or_allocate_player_profile_pda<'a>(
    payer: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    owner: &Pubkey,
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PlayerProfile::LEN);

    if player_profile.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            player_profile.key,
            lamports,
            PlayerProfile::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                player_profile.clone(),
                system_program_account.clone(),
            ],
            &[&[PLAYER_PROFILE_SEED, owner.as_ref(), &[bump]]],
        )?;
        return Ok(());
    }

    if player_profile.lamports() < lamports {
        let delta = lamports - player_profile.lamports();
        let transfer = system_instruction::transfer(payer.key, player_profile.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                player_profile.clone(),
                system_program_account.clone(),
            ],
        )?;
    }

    let allocate = system_instruction::allocate(player_profile.key, PlayerProfile::LEN as u64);
    invoke_signed(
        &allocate,
        &[player_profile.clone(), system_program_account.clone()],
        &[&[PLAYER_PROFILE_SEED, owner.as_ref(), &[bump]]],
    )?;

    let assign = system_instruction::assign(player_profile.key, program_id);
    invoke_signed(
        &assign,
        &[player_profile.clone(), system_program_account.clone()],
        &[&[PLAYER_PROFILE_SEED, owner.as_ref(), &[bump]]],
    )?;

    Ok(())
}

fn create_or_allocate_player_session_pda<'a>(
    payer: &AccountInfo<'a>,
    player_session: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    session_authority: &Pubkey,
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PlayerSession::LEN);

    if player_session.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            player_session.key,
            lamports,
            PlayerSession::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                player_session.clone(),
                system_program_account.clone(),
            ],
            &[&[
                PLAYER_SESSION_SEED,
                payer.key.as_ref(),
                session_authority.as_ref(),
                &[bump],
            ]],
        )?;
        return Ok(());
    }

    if player_session.lamports() < lamports {
        let delta = lamports - player_session.lamports();
        let transfer = system_instruction::transfer(payer.key, player_session.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                player_session.clone(),
                system_program_account.clone(),
            ],
        )?;
    }

    let allocate = system_instruction::allocate(player_session.key, PlayerSession::LEN as u64);
    invoke_signed(
        &allocate,
        &[player_session.clone(), system_program_account.clone()],
        &[&[
            PLAYER_SESSION_SEED,
            payer.key.as_ref(),
            session_authority.as_ref(),
            &[bump],
        ]],
    )?;

    let assign = system_instruction::assign(player_session.key, program_id);
    invoke_signed(
        &assign,
        &[player_session.clone(), system_program_account.clone()],
        &[&[
            PLAYER_SESSION_SEED,
            payer.key.as_ref(),
            session_authority.as_ref(),
            &[bump],
        ]],
    )?;

    Ok(())
}

fn extend_player_profile_if_needed<'a>(
    payer: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
) -> ProgramResult {
    if player_profile.data_len() == PlayerProfile::LEN {
        return Ok(());
    }
    if !PlayerProfile::is_supported_len(player_profile.data_len()) {
        return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PlayerProfile::LEN);
    if player_profile.lamports() < lamports {
        let delta = lamports - player_profile.lamports();
        let transfer = system_instruction::transfer(payer.key, player_profile.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                player_profile.clone(),
                system_program_account.clone(),
            ],
        )?;
    }
    player_profile.realloc(PlayerProfile::LEN, true)?;

    let mut data = player_profile.try_borrow_mut_data()?;
    let created_slot: [u8; 8] = data
        [PlayerProfile::LEGACY_CREATED_SLOT_OFFSET..PlayerProfile::LEGACY_CREATED_SLOT_OFFSET + 8]
        .try_into()
        .map_err(|_| NicechunkPlayerError::InvalidPlayerProfileData)?;
    let updated_slot: [u8; 8] = data
        [PlayerProfile::LEGACY_UPDATED_SLOT_OFFSET..PlayerProfile::LEGACY_UPDATED_SLOT_OFFSET + 8]
        .try_into()
        .map_err(|_| NicechunkPlayerError::InvalidPlayerProfileData)?;
    let created_at: [u8; 8] = data
        [PlayerProfile::LEGACY_CREATED_AT_OFFSET..PlayerProfile::LEGACY_CREATED_AT_OFFSET + 8]
        .try_into()
        .map_err(|_| NicechunkPlayerError::InvalidPlayerProfileData)?;

    data[PlayerProfile::EQUIPPED_BACKPACK_OFFSET..PlayerProfile::EQUIPPED_BACKPACK_OFFSET + 32]
        .fill(0);
    data[PlayerProfile::CREATED_SLOT_OFFSET..PlayerProfile::CREATED_SLOT_OFFSET + 8]
        .copy_from_slice(&created_slot);
    data[PlayerProfile::UPDATED_SLOT_OFFSET..PlayerProfile::UPDATED_SLOT_OFFSET + 8]
        .copy_from_slice(&updated_slot);
    data[PlayerProfile::CREATED_AT_OFFSET..PlayerProfile::CREATED_AT_OFFSET + 8]
        .copy_from_slice(&created_at);
    Ok(())
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_i64(bytes: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
