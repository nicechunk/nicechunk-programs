#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
    hash::hashv,
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

use cluster_config::{
    NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_GAME_PROGRAM_ID,
    NICECHUNK_TREASURY_AUTHORITY,
};
use errors::{require_key_eq, NicechunkPlayerError};
use state::{
    BackpackAccountView, GlobalConfigView, InviteIndex, InviteIndexInitArgs, PlayerAppearance,
    PlayerAppearanceInitArgs, PlayerEquipment, PlayerEquipmentInitArgs, PlayerProfile,
    PlayerSession, PlayerSessionInitArgs, UsernameIndex, UsernameIndexInitArgs,
    EQUIPMENT_TRANSFER_AUTHORITY_SEED, INVITE_INDEX_SEED, PLAYER_APPEARANCE_SEED,
    PLAYER_EQUIPMENT_SEED, PLAYER_PROFILE_SEED, PLAYER_SESSION_SEED, SESSION_ACTION_BREAK_BLOCK,
    SESSION_ACTION_PLACE_BLOCK, USERNAME_INDEX_SEED,
};

declare_id!("CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB");

const CLEAR_EQUIPMENT_BACKPACK_INDEX: u8 = u8::MAX;

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
        0 => initialize_player(program_id, accounts, payload),
        1 => update_position(program_id, accounts, payload),
        2 => set_equipment_slot(program_id, accounts, payload),
        3 => set_backpack_style(program_id, accounts, payload),
        4 => create_or_refresh_player_session(program_id, accounts, payload),
        5 => set_equipped_backpack(program_id, accounts),
        6 => add_forging_xp(program_id, accounts, payload),
        7 => set_player_name(program_id, accounts, payload),
        8 => upsert_player_appearance(program_id, accounts, payload),
        9 => close_player_appearance(program_id, accounts),
        10 => initialize_invite_index_page(program_id, accounts, payload),
        11 => append_invite_registration(program_id, accounts, payload),
        12 => set_equipment_slot_v2(program_id, accounts, payload),
        13 => transfer_equipment_slot(program_id, accounts, payload),
        14 => swap_equipment_slots(program_id, accounts, payload),
        15 => consume_equipment_durability(program_id, accounts, payload),
        _ => Err(NicechunkPlayerError::InvalidInstruction.into()),
    }
}

fn consume_equipment_durability(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 6 || payload.len() != 5 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let slot = payload[0];
    let amount = read_u32(payload, 1);
    if amount == 0 {
        return Err(NicechunkPlayerError::InvalidDurabilityAmount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    if !session_authority.is_signer {
        return Err(NicechunkPlayerError::InvalidSessionAuthority.into());
    }
    if !player_equipment.is_writable {
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
    require_key_eq(
        player_session.owner,
        program_id,
        NicechunkPlayerError::InvalidPlayerSessionData,
    )?;
    require_key_eq(
        player_equipment.owner,
        program_id,
        NicechunkPlayerError::InvalidPlayerEquipmentOwner,
    )?;

    let (expected_profile, _) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;
    let (expected_session, _) = Pubkey::find_program_address(
        &[
            PLAYER_SESSION_SEED,
            owner.key.as_ref(),
            session_authority.key.as_ref(),
        ],
        program_id,
    );
    require_key_eq(
        player_session.key,
        &expected_session,
        NicechunkPlayerError::InvalidPlayerSessionPda,
    )?;
    let (expected_equipment, _) =
        Pubkey::find_program_address(&[PLAYER_EQUIPMENT_SEED, owner.key.as_ref()], program_id);
    require_key_eq(
        player_equipment.key,
        &expected_equipment,
        NicechunkPlayerError::InvalidPlayerEquipmentPda,
    )?;

    let clock = Clock::get()?;
    {
        let session_data = player_session.try_borrow_data()?;
        PlayerSession::validate_action(
            &session_data,
            owner.key,
            session_authority.key,
            player_profile.key,
            global_config.key,
            SESSION_ACTION_BREAK_BLOCK,
            clock.unix_timestamp,
        )?;
    }
    let mut equipment_data = player_equipment.try_borrow_mut_data()?;
    PlayerEquipment::validate_owner_and_config(
        &equipment_data,
        owner.key,
        player_profile.key,
        global_config.key,
    )?;
    PlayerEquipment::consume_forged_durability(&mut equipment_data, slot, amount, clock.slot)
}

fn initialize_player(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let player_name = PlayerProfile::validate_name(payload)
        .map_err(|_| NicechunkPlayerError::InvalidPlayerName)?;
    let has_player_name = !player_name.is_empty();
    if accounts.len() != if has_player_name { 5 } else { 4 } {
        return Err(NicechunkPlayerError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let username_index = if has_player_name {
        Some(next_account_info(account_info_iter)?)
    } else {
        None
    };

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
        player_name,
        clock.slot,
        clock.unix_timestamp,
    )?;
    drop(data);
    if let Some(username_index) = username_index {
        ensure_username_index_current(
            payer,
            username_index,
            system_program_account,
            program_id,
            payer.key,
            player_profile.key,
            global_config.key,
            player_name,
            &clock,
        )?;
    }

    Ok(())
}

fn set_player_name(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    let player_name = PlayerProfile::validate_name(payload)
        .map_err(|_| NicechunkPlayerError::InvalidPlayerName)?;
    let has_player_name = !player_name.is_empty();
    if accounts.len() != if has_player_name { 5 } else { 3 } {
        return Err(NicechunkPlayerError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let (system_program_account, username_index) = if has_player_name {
        (
            Some(next_account_info(account_info_iter)?),
            Some(next_account_info(account_info_iter)?),
        )
    } else {
        (None, None)
    };

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_name(&mut data, player_name, clock.slot)?;
    drop(data);
    if let (Some(system_program_account), Some(username_index)) =
        (system_program_account, username_index)
    {
        ensure_username_index_current(
            authority,
            username_index,
            system_program_account,
            program_id,
            authority.key,
            player_profile.key,
            global_config.key,
            player_name,
            &clock,
        )?;
    }
    Ok(())
}

fn upsert_player_appearance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 6 || payload.len() < 7 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let model_kind = payload[0];
    PlayerAppearance::validate_model_kind(model_kind)
        .map_err(|_| NicechunkPlayerError::InvalidCharacterModelKind)?;
    let player_name_len = read_u16(payload, 1) as usize;
    let title_len = read_u16(payload, 3) as usize;
    let code_len = read_u16(payload, 5) as usize;
    let expected_len = 7usize
        .checked_add(player_name_len)
        .and_then(|len| len.checked_add(title_len))
        .and_then(|len| len.checked_add(code_len))
        .ok_or(NicechunkPlayerError::InvalidInstruction)?;
    if payload.len() != expected_len {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let player_name_bytes = &payload[7..7 + player_name_len];
    let title = &payload[7 + player_name_len..7 + player_name_len + title_len];
    let code = &payload[7 + player_name_len + title_len..];
    let player_name = PlayerProfile::validate_name(player_name_bytes)
        .map_err(|_| NicechunkPlayerError::InvalidPlayerName)?;
    PlayerAppearance::validate_title(title)
        .map_err(|_| NicechunkPlayerError::InvalidAppearanceTitle)?;
    PlayerAppearance::validate_model_code(code)
        .map_err(|_| NicechunkPlayerError::InvalidCharacterCode)?;

    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let appearance = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let username_index = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkPlayerError::InvalidPayer.into());
    }
    if !player_profile.is_writable || !appearance.is_writable || !username_index.is_writable {
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

    let (expected_player_profile, profile_bump) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_player_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;
    let (expected_appearance, character_bump) =
        Pubkey::find_program_address(&[PLAYER_APPEARANCE_SEED, owner.key.as_ref()], program_id);
    require_key_eq(
        appearance.key,
        &expected_appearance,
        NicechunkPlayerError::InvalidAppearancePda,
    )?;

    let global_config_data = global_config.try_borrow_data()?;
    let global_config_view = GlobalConfigView::unpack(&global_config_data)?;
    drop(global_config_data);

    let clock = Clock::get()?;
    ensure_player_profile_current(
        owner,
        player_profile,
        global_config,
        system_program_account,
        program_id,
        owner.key,
        profile_bump,
        &global_config_view,
        &clock,
    )?;
    {
        let mut data = player_profile.try_borrow_mut_data()?;
        PlayerProfile::write_name(&mut data, player_name, clock.slot)?;
    }
    ensure_username_index_current(
        owner,
        username_index,
        system_program_account,
        program_id,
        owner.key,
        player_profile.key,
        global_config.key,
        player_name,
        &clock,
    )?;

    let mut created_slot = clock.slot;
    let mut created_at = clock.unix_timestamp;
    if appearance.owner == program_id {
        require_key_eq(
            appearance.owner,
            program_id,
            NicechunkPlayerError::InvalidAppearanceOwner,
        )?;
        {
            let data = appearance.try_borrow_data()?;
            PlayerAppearance::validate_owner_and_config(
                &data,
                owner.key,
                player_profile.key,
                global_config.key,
            )?;
            if let Some((stored_slot, stored_at)) = PlayerAppearance::created_metadata(&data) {
                created_slot = stored_slot;
                created_at = stored_at;
            }
        }
        ensure_appearance_rent(owner, appearance, system_program_account, program_id)?;
    } else {
        if appearance.owner != &system_program::ID || appearance.data_len() != 0 {
            return Err(NicechunkPlayerError::InvalidSystemAccount.into());
        }
        create_or_allocate_appearance_pda(
            owner,
            appearance,
            system_program_account,
            program_id,
            owner.key,
            character_bump,
        )?;
    }

    let mut data = appearance.try_borrow_mut_data()?;
    PlayerAppearance::pack(
        &mut data,
        &PlayerAppearanceInitArgs {
            bump: character_bump,
            owner: owner.key,
            player_profile: player_profile.key,
            global_config: global_config.key,
            treasury_authority: &NICECHUNK_TREASURY_AUTHORITY,
            model_kind,
            display_name: player_name,
            title,
            model_code: code,
            created_slot,
            updated_slot: clock.slot,
            created_at,
            updated_at: clock.unix_timestamp,
        },
    )
}

fn close_player_appearance(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 3 {
        return Err(NicechunkPlayerError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let treasury_authority = next_account_info(account_info_iter)?;
    let appearance = next_account_info(account_info_iter)?;
    let recipient = next_account_info(account_info_iter)?;

    if !treasury_authority.is_signer || !treasury_authority.is_writable || !recipient.is_writable {
        return Err(NicechunkPlayerError::InvalidTreasuryAuthority.into());
    }
    require_key_eq(
        treasury_authority.key,
        &NICECHUNK_TREASURY_AUTHORITY,
        NicechunkPlayerError::InvalidTreasuryAuthority,
    )?;
    require_key_eq(
        recipient.key,
        treasury_authority.key,
        NicechunkPlayerError::InvalidTreasuryAuthority,
    )?;
    require_key_eq(
        appearance.owner,
        program_id,
        NicechunkPlayerError::InvalidAppearanceOwner,
    )?;

    {
        let data = appearance.try_borrow_data()?;
        PlayerAppearance::validate_treasury_authority(&data, treasury_authority.key)?;
        let owner = PlayerAppearance::owner(&data)?;
        let (expected_appearance, _) =
            Pubkey::find_program_address(&[PLAYER_APPEARANCE_SEED, owner.as_ref()], program_id);
        require_key_eq(
            appearance.key,
            &expected_appearance,
            NicechunkPlayerError::InvalidAppearancePda,
        )?;
    }
    {
        let mut data = appearance.try_borrow_mut_data()?;
        data.fill(0);
    }
    let reclaimed_lamports = appearance.lamports();
    **recipient.try_borrow_mut_lamports()? = recipient
        .lamports()
        .checked_add(reclaimed_lamports)
        .ok_or(NicechunkPlayerError::InvalidAppearanceData)?;
    **appearance.try_borrow_mut_lamports()? = 0;
    Ok(())
}

fn initialize_invite_index_page(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 || payload.len() != 4 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let page_index = read_u32(payload, 0);
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let inviter = next_account_info(account_info_iter)?;
    let invite_index = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkPlayerError::InvalidPayer.into());
    }
    if !invite_index.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    if page_index == 0 && payer.key != inviter.key {
        return Err(NicechunkPlayerError::InviteFirstPageRequiresInviter.into());
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

    let (expected_invite_index, bump) = invite_index_pda(program_id, inviter.key, page_index);
    require_key_eq(
        invite_index.key,
        &expected_invite_index,
        NicechunkPlayerError::InvalidInviteIndexPda,
    )?;
    if invite_index.owner == program_id {
        let data = invite_index.try_borrow_data()?;
        return InviteIndex::validate(&data, inviter.key, global_config.key, page_index);
    }
    if invite_index.owner != &system_program::ID || invite_index.data_len() != 0 {
        return Err(NicechunkPlayerError::InvalidSystemAccount.into());
    }

    create_or_allocate_invite_index_pda(
        payer,
        invite_index,
        system_program_account,
        program_id,
        inviter.key,
        page_index,
        bump,
    )?;
    let clock = Clock::get()?;
    let mut data = invite_index.try_borrow_mut_data()?;
    InviteIndex::pack_empty(
        &mut data,
        &InviteIndexInitArgs {
            bump,
            inviter: inviter.key,
            global_config: global_config.key,
            page_index,
            created_slot: clock.slot,
        },
    )
}

fn append_invite_registration(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if payload.len() != 4 || (accounts.len() != 5 && accounts.len() != 6) {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let page_index = read_u32(payload, 0);
    let account_info_iter = &mut accounts.iter();
    let invited = next_account_info(account_info_iter)?;
    let inviter = next_account_info(account_info_iter)?;
    let invite_index = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !invited.is_signer || !invited.is_writable {
        return Err(NicechunkPlayerError::InvalidPayer.into());
    }
    if invited.key == inviter.key {
        return Err(NicechunkPlayerError::InvalidInviteSelf.into());
    }
    if !invite_index.is_writable {
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

    let (expected_invite_index, bump) = invite_index_pda(program_id, inviter.key, page_index);
    require_key_eq(
        invite_index.key,
        &expected_invite_index,
        NicechunkPlayerError::InvalidInviteIndexPda,
    )?;

    if invite_index.owner != program_id {
        if page_index == 0 {
            return Err(NicechunkPlayerError::InviteFirstPageRequiresInviter.into());
        }
        if accounts.len() != 6 {
            return Err(NicechunkPlayerError::InvitePreviousPageRequired.into());
        }
        let previous_invite_index = next_account_info(account_info_iter)?;
        let previous_page_index = page_index.saturating_sub(1);
        let (expected_previous, _) = invite_index_pda(program_id, inviter.key, previous_page_index);
        require_key_eq(
            previous_invite_index.key,
            &expected_previous,
            NicechunkPlayerError::InvalidInviteIndexPda,
        )?;
        require_key_eq(
            previous_invite_index.owner,
            program_id,
            NicechunkPlayerError::InvalidInviteIndexData,
        )?;
        let previous_data = previous_invite_index.try_borrow_data()?;
        if !InviteIndex::is_full(
            &previous_data,
            inviter.key,
            global_config.key,
            previous_page_index,
        )
        .map_err(|_| NicechunkPlayerError::InvalidInviteIndexData)?
        {
            return Err(NicechunkPlayerError::InvitePreviousPageNotFull.into());
        }
        drop(previous_data);
        if invite_index.owner != &system_program::ID || invite_index.data_len() != 0 {
            return Err(NicechunkPlayerError::InvalidSystemAccount.into());
        }
        create_or_allocate_invite_index_pda(
            invited,
            invite_index,
            system_program_account,
            program_id,
            inviter.key,
            page_index,
            bump,
        )?;
        let clock = Clock::get()?;
        {
            let mut data = invite_index.try_borrow_mut_data()?;
            InviteIndex::pack_empty(
                &mut data,
                &InviteIndexInitArgs {
                    bump,
                    inviter: inviter.key,
                    global_config: global_config.key,
                    page_index,
                    created_slot: clock.slot,
                },
            )?;
        }
    }

    let clock = Clock::get()?;
    let mut data = invite_index.try_borrow_mut_data()?;
    InviteIndex::validate(&data, inviter.key, global_config.key, page_index)?;
    InviteIndex::append(&mut data, invited.key, clock.slot)
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
    if payload.len() != 2 || (accounts.len() != 3 && accounts.len() != 4) {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    let slot = payload[0];
    let backpack_index = payload[1];
    let item = if backpack_index == CLEAR_EQUIPMENT_BACKPACK_INDEX {
        if accounts.len() != 3 {
            return Err(NicechunkPlayerError::InvalidAccountCount.into());
        }
        Pubkey::default()
    } else {
        if accounts.len() != 4 {
            return Err(NicechunkPlayerError::InvalidAccountCount.into());
        }
        let backpack = next_account_info(account_info_iter)?;
        if backpack.owner != &NICECHUNK_BACKPACK_PROGRAM_ID
            && backpack.owner != &NICECHUNK_GAME_PROGRAM_ID
        {
            return Err(NicechunkPlayerError::InvalidBackpackProgram.into());
        }
        let backpack_data = backpack.try_borrow_data()?;
        BackpackAccountView::validate_pda_and_owner(
            &backpack_data,
            backpack.key,
            backpack.owner,
            authority.key,
        )?;
        BackpackAccountView::equippable_item_pda_at(&backpack_data, authority.key, backpack_index)?
    };
    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipment_slot(&mut data, slot, &item, clock.slot)
}

fn set_equipment_slot_v2(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if payload.len() < 4 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let slot = payload[0];
    let backpack_index = payload[1];
    let model_length = read_u16(payload, 2) as usize;
    if payload.len() != 4 + model_length {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let clears_slot = backpack_index == CLEAR_EQUIPMENT_BACKPACK_INDEX;
    if accounts.len() != if clears_slot { 5 } else { 6 } || (clears_slot && model_length != 0) {
        return Err(NicechunkPlayerError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let backpack = if clears_slot {
        None
    } else {
        Some(next_account_info(account_info_iter)?)
    };

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    if !authority.is_writable {
        return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
    }
    if !player_equipment.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkPlayerError::InvalidSystemProgram,
    )?;
    let (expected_player_equipment, bump) =
        Pubkey::find_program_address(&[PLAYER_EQUIPMENT_SEED, authority.key.as_ref()], program_id);
    require_key_eq(
        player_equipment.key,
        &expected_player_equipment,
        NicechunkPlayerError::InvalidPlayerEquipmentPda,
    )?;

    let equipment_exists =
        player_equipment.owner == program_id && player_equipment.data_len() == PlayerEquipment::LEN;
    let clock = Clock::get()?;
    if clears_slot && !equipment_exists {
        if player_equipment.owner != &system_program::ID || player_equipment.data_len() != 0 {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentOwner.into());
        }
        let mut profile_data = player_profile.try_borrow_mut_data()?;
        return PlayerProfile::write_equipment_slot(
            &mut profile_data,
            slot,
            &Pubkey::default(),
            clock.slot,
        );
    }

    ensure_player_equipment_current(
        authority,
        player_profile,
        player_equipment,
        global_config,
        system_program_account,
        program_id,
        bump,
        &clock,
    )?;

    let legacy_identity = if let Some(backpack) = backpack {
        if backpack.owner != &NICECHUNK_BACKPACK_PROGRAM_ID
            && backpack.owner != &NICECHUNK_GAME_PROGRAM_ID
        {
            return Err(NicechunkPlayerError::InvalidBackpackProgram.into());
        }
        let backpack_data = backpack.try_borrow_data()?;
        BackpackAccountView::validate_pda_and_owner(
            &backpack_data,
            backpack.key,
            backpack.owner,
            authority.key,
        )?;
        let backpack_record = BackpackAccountView::equipment_record_at(
            &backpack_data,
            authority.key,
            backpack_index,
        )?;
        drop(backpack_data);
        {
            let mut equipment_data = player_equipment.try_borrow_mut_data()?;
            PlayerEquipment::validate_owner_and_config(
                &equipment_data,
                authority.key,
                player_profile.key,
                global_config.key,
            )?;
            PlayerEquipment::write_slot(
                &mut equipment_data,
                slot,
                backpack_index,
                backpack.key,
                &backpack_record,
                &payload[4..],
                clock.slot,
            )?;
        }
        Pubkey::new_from_array(
            hashv(&[b"equipment-v1", backpack.key.as_ref(), &backpack_record]).to_bytes(),
        )
    } else {
        let mut equipment_data = player_equipment.try_borrow_mut_data()?;
        PlayerEquipment::validate_owner_and_config(
            &equipment_data,
            authority.key,
            player_profile.key,
            global_config.key,
        )?;
        PlayerEquipment::clear_slot(&mut equipment_data, slot, clock.slot)?;
        Pubkey::default()
    };

    let mut profile_data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipment_slot(&mut profile_data, slot, &legacy_identity, clock.slot)
}

fn transfer_equipment_slot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 9 || payload.len() < 4 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let slot = payload[0];
    let backpack_index = payload[1];
    let model_length = read_u16(payload, 2) as usize;
    if payload.len() != 4 + model_length
        || (backpack_index == CLEAR_EQUIPMENT_BACKPACK_INDEX && model_length != 0)
    {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let game_program = next_account_info(account_info_iter)?;
    let transfer_authority = next_account_info(account_info_iter)?;

    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    if !authority.is_writable || !player_equipment.is_writable || !backpack.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkPlayerError::InvalidSystemProgram,
    )?;
    require_key_eq(
        game_program.key,
        &NICECHUNK_GAME_PROGRAM_ID,
        NicechunkPlayerError::InvalidGameProgram,
    )?;
    if !game_program.executable {
        return Err(NicechunkPlayerError::InvalidGameProgram.into());
    }
    require_key_eq(
        backpack.owner,
        game_program.key,
        NicechunkPlayerError::InvalidBackpackProgram,
    )?;
    let (expected_equipment, equipment_bump) =
        Pubkey::find_program_address(&[PLAYER_EQUIPMENT_SEED, authority.key.as_ref()], program_id);
    require_key_eq(
        player_equipment.key,
        &expected_equipment,
        NicechunkPlayerError::InvalidPlayerEquipmentPda,
    )?;
    let (expected_transfer_authority, transfer_bump) =
        Pubkey::find_program_address(&[EQUIPMENT_TRANSFER_AUTHORITY_SEED], program_id);
    require_key_eq(
        transfer_authority.key,
        &expected_transfer_authority,
        NicechunkPlayerError::InvalidEquipmentTransferAuthority,
    )?;

    let clock = Clock::get()?;
    ensure_player_equipment_current(
        authority,
        player_profile,
        player_equipment,
        global_config,
        system_program_account,
        program_id,
        equipment_bump,
        &clock,
    )?;

    let clears_slot = backpack_index == CLEAR_EQUIPMENT_BACKPACK_INDEX;
    let (was_equipped, was_custodied) = {
        let equipment_data = player_equipment.try_borrow_data()?;
        PlayerEquipment::validate_owner_and_config(
            &equipment_data,
            authority.key,
            player_profile.key,
            global_config.key,
        )?;
        (
            PlayerEquipment::slot_is_equipped(&equipment_data, slot)?,
            PlayerEquipment::slot_is_custodied(&equipment_data, slot)?,
        )
    };

    let mut next_identity = Pubkey::default();
    if clears_slot {
        if was_custodied {
            invoke_backpack_equipment_transfer(
                game_program,
                transfer_authority,
                authority,
                backpack,
                player_equipment,
                material_physics,
                &[1, 11, slot],
                transfer_bump,
            )?;
        }
        if was_equipped {
            let mut equipment_data = player_equipment.try_borrow_mut_data()?;
            PlayerEquipment::clear_slot(&mut equipment_data, slot, clock.slot)?;
        }
    } else {
        let backpack_record = {
            let backpack_data = backpack.try_borrow_data()?;
            BackpackAccountView::validate_pda_and_owner(
                &backpack_data,
                backpack.key,
                backpack.owner,
                authority.key,
            )?;
            BackpackAccountView::equipment_record_at(&backpack_data, authority.key, backpack_index)?
        };
        invoke_backpack_equipment_transfer(
            game_program,
            transfer_authority,
            authority,
            backpack,
            player_equipment,
            material_physics,
            &[1, 10, slot, backpack_index],
            transfer_bump,
        )?;
        {
            let mut equipment_data = player_equipment.try_borrow_mut_data()?;
            PlayerEquipment::write_custodied_slot(
                &mut equipment_data,
                slot,
                backpack_index,
                backpack.key,
                &backpack_record,
                &payload[4..],
                clock.slot,
            )?;
            next_identity = PlayerEquipment::slot_identity(&equipment_data, slot)?;
        }
    }

    let mut profile_data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipment_slot(&mut profile_data, slot, &next_identity, clock.slot)
}

fn swap_equipment_slots(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != 2 || payload[0] == payload[1] {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let from_slot = payload[0];
    let to_slot = payload[1];
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_equipment = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    validate_player_write_accounts(program_id, authority, player_profile, global_config)?;
    if !player_equipment.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    let (expected_equipment, _) =
        Pubkey::find_program_address(&[PLAYER_EQUIPMENT_SEED, authority.key.as_ref()], program_id);
    require_key_eq(
        player_equipment.key,
        &expected_equipment,
        NicechunkPlayerError::InvalidPlayerEquipmentPda,
    )?;
    require_key_eq(
        player_equipment.owner,
        program_id,
        NicechunkPlayerError::InvalidPlayerEquipmentOwner,
    )?;
    let clock = Clock::get()?;
    let (from_identity, to_identity) = {
        let mut equipment_data = player_equipment.try_borrow_mut_data()?;
        PlayerEquipment::validate_owner_and_config(
            &equipment_data,
            authority.key,
            player_profile.key,
            global_config.key,
        )?;
        for slot in [from_slot, to_slot] {
            if PlayerEquipment::slot_is_equipped(&equipment_data, slot)?
                && !PlayerEquipment::slot_is_custodied(&equipment_data, slot)?
            {
                return Err(NicechunkPlayerError::EquipmentNotCustodied.into());
            }
        }
        PlayerEquipment::swap_slots(&mut equipment_data, from_slot, to_slot, clock.slot)?;
        (
            PlayerEquipment::slot_identity(&equipment_data, from_slot)?,
            PlayerEquipment::slot_identity(&equipment_data, to_slot)?,
        )
    };
    let mut profile_data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipment_slot(&mut profile_data, from_slot, &from_identity, clock.slot)?;
    PlayerProfile::write_equipment_slot(&mut profile_data, to_slot, &to_identity, clock.slot)
}

fn invoke_backpack_equipment_transfer<'a>(
    game_program: &AccountInfo<'a>,
    transfer_authority: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    player_equipment: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    data: &[u8],
    transfer_bump: u8,
) -> ProgramResult {
    let instruction = Instruction {
        program_id: *game_program.key,
        accounts: vec![
            AccountMeta::new_readonly(*transfer_authority.key, true),
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new(*backpack.key, false),
            AccountMeta::new_readonly(*player_equipment.key, false),
            AccountMeta::new_readonly(*material_physics.key, false),
        ],
        data: data.to_vec(),
    };
    let bump_seed = [transfer_bump];
    let signer_seeds: &[&[u8]] = &[EQUIPMENT_TRANSFER_AUTHORITY_SEED, &bump_seed];
    invoke_signed(
        &instruction,
        &[
            transfer_authority.clone(),
            authority.clone(),
            backpack.clone(),
            player_equipment.clone(),
            material_physics.clone(),
            game_program.clone(),
        ],
        &[signer_seeds],
    )
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

    let (expected_player_profile, profile_bump) =
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

    ensure_player_profile_current(
        owner,
        player_profile,
        global_config,
        system_program_account,
        program_id,
        owner.key,
        profile_bump,
        &global_config_view,
        &clock,
    )?;

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
        BackpackAccountView::validate_pda_and_owner(
            &backpack_data,
            backpack.key,
            backpack.owner,
            authority.key,
        )?;
    }

    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::write_equipped_backpack(&mut data, backpack.key, clock.slot)
}

fn add_forging_xp(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != 10 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }
    let grade = payload[0].max(1).min(10);
    let item_level = payload[1].max(1).min(100);
    let gained_xp = read_u64(payload, 2);
    if gained_xp == 0 {
        return Err(NicechunkPlayerError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let forging_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
    }
    if !forging_authority.is_signer {
        return Err(NicechunkPlayerError::InvalidForgingAuthority.into());
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
    let (expected_player_profile, _) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.key.as_ref()], program_id);
    require_key_eq(
        player_profile.key,
        &expected_player_profile,
        NicechunkPlayerError::InvalidPlayerProfilePda,
    )?;
    {
        let data = player_profile.try_borrow_data()?;
        PlayerProfile::validate_owner(&data, owner.key)?;
    }
    {
        if forging_authority.owner != &NICECHUNK_BACKPACK_PROGRAM_ID
            && forging_authority.owner != &NICECHUNK_GAME_PROGRAM_ID
        {
            return Err(NicechunkPlayerError::InvalidForgingAuthority.into());
        }
        let data = forging_authority.try_borrow_data()?;
        BackpackAccountView::validate_pda_and_owner(
            &data,
            forging_authority.key,
            forging_authority.owner,
            owner.key,
        )?;
    }
    let clock = Clock::get()?;
    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::add_forging_result(
        &mut data, owner.key, gained_xp, grade, item_level, clock.slot,
    )
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

#[allow(clippy::too_many_arguments)]
fn ensure_player_equipment_current<'a>(
    payer: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    player_equipment: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    bump: u8,
    clock: &Clock,
) -> ProgramResult {
    if player_equipment.owner == program_id && player_equipment.data_len() == PlayerEquipment::LEN {
        let data = player_equipment.try_borrow_data()?;
        PlayerEquipment::validate_owner_and_config(
            &data,
            payer.key,
            player_profile.key,
            global_config.key,
        )?;
        return Ok(());
    }

    if player_equipment.owner == program_id {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(PlayerEquipment::LEN);
        if player_equipment.lamports() < lamports {
            let transfer = system_instruction::transfer(
                payer.key,
                player_equipment.key,
                lamports - player_equipment.lamports(),
            );
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    player_equipment.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        player_equipment.realloc(PlayerEquipment::LEN, false)?;
    } else {
        if player_equipment.owner != &system_program::ID || player_equipment.data_len() != 0 {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentOwner.into());
        }
        create_or_allocate_player_equipment_pda(
            payer,
            player_equipment,
            system_program_account,
            program_id,
            bump,
        )?;
    }

    let mut data = player_equipment.try_borrow_mut_data()?;
    PlayerEquipment::pack_empty(
        &mut data,
        &PlayerEquipmentInitArgs {
            bump,
            owner: payer.key,
            player_profile: player_profile.key,
            global_config: global_config.key,
            created_slot: clock.slot,
        },
    )
}

fn create_or_allocate_player_equipment_pda<'a>(
    payer: &AccountInfo<'a>,
    player_equipment: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PlayerEquipment::LEN);
    let signer_seeds = &[PLAYER_EQUIPMENT_SEED, payer.key.as_ref(), &[bump]];

    if player_equipment.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            player_equipment.key,
            lamports,
            PlayerEquipment::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                player_equipment.clone(),
                system_program_account.clone(),
            ],
            &[signer_seeds],
        )?;
        return Ok(());
    }

    if player_equipment.lamports() < lamports {
        let transfer = system_instruction::transfer(
            payer.key,
            player_equipment.key,
            lamports - player_equipment.lamports(),
        );
        invoke(
            &transfer,
            &[
                payer.clone(),
                player_equipment.clone(),
                system_program_account.clone(),
            ],
        )?;
    }
    let allocate = system_instruction::allocate(player_equipment.key, PlayerEquipment::LEN as u64);
    invoke_signed(
        &allocate,
        &[player_equipment.clone(), system_program_account.clone()],
        &[signer_seeds],
    )?;
    let assign = system_instruction::assign(player_equipment.key, program_id);
    invoke_signed(
        &assign,
        &[player_equipment.clone(), system_program_account.clone()],
        &[signer_seeds],
    )?;
    Ok(())
}

fn ensure_player_profile_current<'a>(
    payer: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    owner: &Pubkey,
    bump: u8,
    global_config_view: &GlobalConfigView,
    clock: &Clock,
) -> ProgramResult {
    {
        let data = player_profile.try_borrow_data()?;
        if PlayerProfile::validate_owner_and_config(&data, owner, global_config.key).is_ok() {
            return Ok(());
        }
    }

    if player_profile.owner == program_id {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(PlayerProfile::LEN);
        if player_profile.lamports() < lamports {
            let transfer = system_instruction::transfer(
                payer.key,
                player_profile.key,
                lamports - player_profile.lamports(),
            );
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    player_profile.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        if player_profile.data_len() != PlayerProfile::LEN {
            player_profile.realloc(PlayerProfile::LEN, false)?;
        }
    } else {
        if player_profile.owner != &system_program::ID || player_profile.data_len() != 0 {
            return Err(NicechunkPlayerError::InvalidSystemAccount.into());
        }
        create_or_allocate_player_profile_pda(
            payer,
            player_profile,
            system_program_account,
            program_id,
            owner,
            bump,
        )?;
    }

    let mut data = player_profile.try_borrow_mut_data()?;
    PlayerProfile::pack_default(
        &mut data,
        bump,
        owner,
        global_config.key,
        global_config_view.world_id,
        "",
        clock.slot,
        clock.unix_timestamp,
    )
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

fn ensure_username_index_current<'a>(
    payer: &AccountInfo<'a>,
    username_index: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    owner: &Pubkey,
    player_profile: &Pubkey,
    global_config: &Pubkey,
    player_name: &str,
    clock: &Clock,
) -> ProgramResult {
    if player_name.is_empty() {
        return Ok(());
    }
    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkPlayerError::InvalidPayer.into());
    }
    if !username_index.is_writable {
        return Err(NicechunkPlayerError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkPlayerError::InvalidSystemProgram,
    )?;
    let name_hash = canonical_player_name_hash(player_name);
    let (expected_username_index, bump) = username_index_pda(program_id, &name_hash);
    require_key_eq(
        username_index.key,
        &expected_username_index,
        NicechunkPlayerError::InvalidUsernameIndexPda,
    )?;
    if username_index.owner == program_id {
        let data = username_index.try_borrow_data()?;
        return UsernameIndex::validate_owner_or_available(
            &data,
            owner,
            player_profile,
            global_config,
            &name_hash,
        );
    }
    if username_index.owner != &system_program::ID || username_index.data_len() != 0 {
        return Err(NicechunkPlayerError::UsernameAlreadyTaken.into());
    }
    create_or_allocate_username_index_pda(
        payer,
        username_index,
        system_program_account,
        program_id,
        &name_hash,
        bump,
    )?;
    let mut data = username_index.try_borrow_mut_data()?;
    UsernameIndex::pack(
        &mut data,
        &UsernameIndexInitArgs {
            bump,
            owner,
            player_profile,
            global_config,
            name_hash: &name_hash,
            display_name: player_name,
            created_slot: clock.slot,
        },
    )
}

fn username_index_pda(program_id: &Pubkey, name_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USERNAME_INDEX_SEED, name_hash], program_id)
}

fn canonical_player_name_hash(player_name: &str) -> [u8; 32] {
    let mut canonical = Vec::with_capacity(player_name.len());
    for ch in player_name.chars() {
        if ch.is_ascii_uppercase() {
            canonical.push(ch.to_ascii_lowercase() as u8);
        } else {
            let mut encoded = [0_u8; 4];
            canonical.extend_from_slice(ch.encode_utf8(&mut encoded).as_bytes());
        }
    }
    hashv(&[canonical.as_slice()]).to_bytes()
}

fn create_or_allocate_username_index_pda<'a>(
    payer: &AccountInfo<'a>,
    username_index: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    name_hash: &[u8; 32],
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(UsernameIndex::LEN);

    if username_index.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            username_index.key,
            lamports,
            UsernameIndex::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                username_index.clone(),
                system_program_account.clone(),
            ],
            &[&[USERNAME_INDEX_SEED, name_hash, &[bump]]],
        )?;
        return Ok(());
    }

    if username_index.lamports() < lamports {
        let delta = lamports - username_index.lamports();
        let transfer = system_instruction::transfer(payer.key, username_index.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                username_index.clone(),
                system_program_account.clone(),
            ],
        )?;
    }

    let allocate = system_instruction::allocate(username_index.key, UsernameIndex::LEN as u64);
    invoke_signed(
        &allocate,
        &[username_index.clone(), system_program_account.clone()],
        &[&[USERNAME_INDEX_SEED, name_hash, &[bump]]],
    )?;

    let assign = system_instruction::assign(username_index.key, program_id);
    invoke_signed(
        &assign,
        &[username_index.clone(), system_program_account.clone()],
        &[&[USERNAME_INDEX_SEED, name_hash, &[bump]]],
    )?;

    Ok(())
}

fn invite_index_pda(program_id: &Pubkey, inviter: &Pubkey, page_index: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            INVITE_INDEX_SEED,
            inviter.as_ref(),
            &page_index.to_le_bytes(),
        ],
        program_id,
    )
}

fn create_or_allocate_invite_index_pda<'a>(
    payer: &AccountInfo<'a>,
    invite_index: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    inviter: &Pubkey,
    page_index: u32,
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(InviteIndex::LEN);

    if invite_index.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            invite_index.key,
            lamports,
            InviteIndex::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                invite_index.clone(),
                system_program_account.clone(),
            ],
            &[&[
                INVITE_INDEX_SEED,
                inviter.as_ref(),
                &page_index.to_le_bytes(),
                &[bump],
            ]],
        )?;
        return Ok(());
    }

    if invite_index.lamports() < lamports {
        let delta = lamports - invite_index.lamports();
        let transfer = system_instruction::transfer(payer.key, invite_index.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                invite_index.clone(),
                system_program_account.clone(),
            ],
        )?;
    }

    let allocate = system_instruction::allocate(invite_index.key, InviteIndex::LEN as u64);
    invoke_signed(
        &allocate,
        &[invite_index.clone(), system_program_account.clone()],
        &[&[
            INVITE_INDEX_SEED,
            inviter.as_ref(),
            &page_index.to_le_bytes(),
            &[bump],
        ]],
    )?;

    let assign = system_instruction::assign(invite_index.key, program_id);
    invoke_signed(
        &assign,
        &[invite_index.clone(), system_program_account.clone()],
        &[&[
            INVITE_INDEX_SEED,
            inviter.as_ref(),
            &page_index.to_le_bytes(),
            &[bump],
        ]],
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

fn ensure_appearance_rent<'a>(
    payer: &AccountInfo<'a>,
    appearance: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PlayerAppearance::LEN);
    if appearance.lamports() < lamports {
        let transfer = system_instruction::transfer(
            payer.key,
            appearance.key,
            lamports - appearance.lamports(),
        );
        invoke(
            &transfer,
            &[
                payer.clone(),
                appearance.clone(),
                system_program_account.clone(),
            ],
        )?;
    }
    if appearance.data_len() != PlayerAppearance::LEN {
        appearance.realloc(PlayerAppearance::LEN, false)?;
    }
    require_key_eq(
        appearance.owner,
        program_id,
        NicechunkPlayerError::InvalidAppearanceOwner,
    )
}

fn create_or_allocate_appearance_pda<'a>(
    payer: &AccountInfo<'a>,
    appearance: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    owner: &Pubkey,
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(PlayerAppearance::LEN);

    if appearance.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            appearance.key,
            lamports,
            PlayerAppearance::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                appearance.clone(),
                system_program_account.clone(),
            ],
            &[&[PLAYER_APPEARANCE_SEED, owner.as_ref(), &[bump]]],
        )?;
        return Ok(());
    }

    if appearance.lamports() < lamports {
        let delta = lamports - appearance.lamports();
        let transfer = system_instruction::transfer(payer.key, appearance.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                appearance.clone(),
                system_program_account.clone(),
            ],
        )?;
    }

    let allocate = system_instruction::allocate(appearance.key, PlayerAppearance::LEN as u64);
    invoke_signed(
        &allocate,
        &[appearance.clone(), system_program_account.clone()],
        &[&[PLAYER_APPEARANCE_SEED, owner.as_ref(), &[bump]]],
    )?;

    let assign = system_instruction::assign(appearance.key, program_id);
    invoke_signed(
        &assign,
        &[appearance.clone(), system_program_account.clone()],
        &[&[PLAYER_APPEARANCE_SEED, owner.as_ref(), &[bump]]],
    )?;

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

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
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
