#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
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
    MAGICBLOCK_DELEGATION_PROGRAM_ID, NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkChunkError};
use state::{
    generated_block_id_at, generated_surface_height, pack_broken_coord, BlockChangeArgs,
    ChunkBrokenInitArgs, ChunkBrokenState, ChunkInitArgs, ChunkState, GeneratedBlockArgs,
    GlobalConfigView, MineBlockArgs, PlayerProfileView, PlayerSessionView, BLOCK_AIR,
    BLOCK_BEDROCK, BLOCK_WATER, CHUNK_BROKEN_GROW_BY, CHUNK_BROKEN_INITIAL_CAPACITY,
    CHUNK_BROKEN_MAX_CAPACITY, CHUNK_BROKEN_SEED, CHUNK_SEED,
};

declare_id!("12rCvz9PZ64Uix1TCiHEGU4AN4ZS1h4jv5u7CkqTRdk5");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkChunkError::InvalidInstruction)?;

    match tag {
        0 => initialize_chunk(program_id, accounts, payload),
        1 => record_block_change(program_id, accounts, payload),
        2 => delegate_chunk(program_id, accounts, payload),
        3 => record_block_change_with_session(program_id, accounts, payload),
        4 => verify_generated_block(accounts, payload),
        5 => mine_block(program_id, accounts, payload),
        _ => Err(NicechunkChunkError::InvalidInstruction.into()),
    }
}

fn initialize_chunk(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != 8 {
        return Err(NicechunkChunkError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkChunkError::InvalidPayer.into());
    }
    if !chunk.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }

    let chunk_x = read_i32(payload, 0);
    let chunk_z = read_i32(payload, 4);
    let global_config_view = validate_global_config(global_config)?;
    let bump = validate_chunk_pda(program_id, chunk.key, global_config.key, chunk_x, chunk_z)?;
    create_chunk_if_needed(
        payer,
        chunk,
        global_config,
        system_program_account,
        program_id,
        &global_config_view,
        chunk_x,
        chunk_z,
        bump,
    )
}

fn record_block_change(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = BlockChangeArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !authority.is_signer || !authority.is_writable {
        return Err(NicechunkChunkError::InvalidPayer.into());
    }
    if !chunk.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;
    require_key_eq(
        player_profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkChunkError::InvalidPlayerProgram,
    )?;

    let global_config_view = validate_global_config(global_config)?;
    args.validate(&global_config_view)?;

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate(&player_profile_data, authority.key, global_config.key)?;
    drop(player_profile_data);

    let bump = validate_chunk_pda(
        program_id,
        chunk.key,
        global_config.key,
        args.chunk_x,
        args.chunk_z,
    )?;

    create_chunk_if_needed(
        authority,
        chunk,
        global_config,
        system_program_account,
        program_id,
        &global_config_view,
        args.chunk_x,
        args.chunk_z,
        bump,
    )?;

    let mut data = chunk.try_borrow_mut_data()?;
    ChunkState::validate_header(
        &data,
        global_config.key,
        global_config_view.world_id,
        args.chunk_x,
        args.chunk_z,
    )?;
    validate_previous_block_id(&data, &global_config_view, &args)?;
    let clock = Clock::get()?;
    ChunkState::append_delta(
        &mut data,
        &args,
        authority.key,
        clock.slot,
        clock.unix_timestamp,
    )
}

fn record_block_change_with_session(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 6 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = BlockChangeArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !session_authority.is_signer || !session_authority.is_writable {
        return Err(NicechunkChunkError::InvalidSessionAuthority.into());
    }
    if !chunk.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;
    require_key_eq(
        player_profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkChunkError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        player_session.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkChunkError::InvalidPlayerProgram,
    )?;

    let global_config_view = validate_global_config(global_config)?;
    args.validate(&global_config_view)?;

    let clock = Clock::get()?;
    let player_session_data = player_session.try_borrow_data()?;
    let session = PlayerSessionView::validate(
        &player_session_data,
        session_authority.key,
        player_profile.key,
        global_config.key,
        args.action,
        clock.unix_timestamp,
    )?;
    drop(player_session_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate(&player_profile_data, &session.owner, global_config.key)?;
    drop(player_profile_data);

    let bump = validate_chunk_pda(
        program_id,
        chunk.key,
        global_config.key,
        args.chunk_x,
        args.chunk_z,
    )?;

    create_chunk_if_needed(
        session_authority,
        chunk,
        global_config,
        system_program_account,
        program_id,
        &global_config_view,
        args.chunk_x,
        args.chunk_z,
        bump,
    )?;

    let mut data = chunk.try_borrow_mut_data()?;
    ChunkState::validate_header(
        &data,
        global_config.key,
        global_config_view.world_id,
        args.chunk_x,
        args.chunk_z,
    )?;
    validate_previous_block_id(&data, &global_config_view, &args)?;
    ChunkState::append_delta(
        &mut data,
        &args,
        &session.owner,
        clock.slot,
        clock.unix_timestamp,
    )
}

fn delegate_chunk(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 9 || payload.len() != 12 {
        return Err(NicechunkChunkError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let owner_program = next_account_info(account_info_iter)?;
    let delegate_buffer = next_account_info(account_info_iter)?;
    let delegation_record = next_account_info(account_info_iter)?;
    let delegation_metadata = next_account_info(account_info_iter)?;
    let delegation_program = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkChunkError::InvalidPayer.into());
    }
    if !chunk.is_writable || !delegate_buffer.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    require_key_eq(
        owner_program.key,
        program_id,
        NicechunkChunkError::InvalidOwnerProgram,
    )?;
    require_key_eq(
        delegation_program.key,
        &MAGICBLOCK_DELEGATION_PROGRAM_ID,
        NicechunkChunkError::InvalidDelegationProgram,
    )?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;

    let chunk_x = read_i32(payload, 0);
    let chunk_z = read_i32(payload, 4);
    let commit_frequency_ms = read_u32(payload, 8);
    let global_config_view = validate_global_config(global_config)?;
    let bump = validate_chunk_pda(program_id, chunk.key, global_config.key, chunk_x, chunk_z)?;

    create_chunk_if_needed(
        payer,
        chunk,
        global_config,
        system_program_account,
        program_id,
        &global_config_view,
        chunk_x,
        chunk_z,
        bump,
    )?;

    if chunk.owner != program_id {
        return Err(NicechunkChunkError::InvalidChunkOwner.into());
    }

    validate_delegation_pdas(
        program_id,
        chunk.key,
        delegate_buffer.key,
        delegation_record.key,
        delegation_metadata.key,
    )?;

    delegate_chunk_to_magicblock(
        payer,
        chunk,
        owner_program,
        delegate_buffer,
        delegation_record,
        delegation_metadata,
        delegation_program,
        system_program_account,
        global_config.key,
        chunk_x,
        chunk_z,
        bump,
        commit_frequency_ms,
    )
}

fn verify_generated_block(accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 1 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = GeneratedBlockArgs::unpack(payload)?;
    let global_config = &accounts[0];
    let global_config_view = validate_global_config(global_config)?;
    args.validate(&global_config_view)?;

    let block_id = generated_block_id_at(&global_config_view, &args);
    let world_x = args.world_x(&global_config_view);
    let world_z = args.world_z(&global_config_view);
    let surface_y = generated_surface_height(&global_config_view, world_x, world_z);
    msg!(
        "NCK generated block verify: chunk=({},{}), local=({},{},{}), world=({},{},{}), surface_y={}, block_id={}",
        args.chunk_x,
        args.chunk_z,
        args.local_x,
        args.y,
        args.local_z,
        world_x,
        args.y,
        world_z,
        surface_y,
        block_id
    );

    if args.expected_block_id != GeneratedBlockArgs::INSPECT_ONLY_EXPECTED_BLOCK_ID
        && args.expected_block_id != block_id
    {
        msg!(
            "NCK generated block mismatch: expected={}, actual={}",
            args.expected_block_id,
            block_id
        );
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }

    Ok(())
}

fn mine_block(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 4 && accounts.len() != 6 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = MineBlockArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let (player_profile, player_session, chunk_broken, global_config, system_program_account) =
        if accounts.len() == 6 {
            (
                Some(next_account_info(account_info_iter)?),
                Some(next_account_info(account_info_iter)?),
                next_account_info(account_info_iter)?,
                next_account_info(account_info_iter)?,
                next_account_info(account_info_iter)?,
            )
        } else {
            (
                None,
                None,
                next_account_info(account_info_iter)?,
                next_account_info(account_info_iter)?,
                next_account_info(account_info_iter)?,
            )
        };

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkChunkError::InvalidPayer.into());
    }
    if !chunk_broken.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;

    let global_config_view = validate_global_config(global_config)?;
    let generated_args = args.generated_args(&global_config_view)?;
    generated_args.validate(&global_config_view)?;

    if let (Some(player_profile), Some(player_session)) = (player_profile, player_session) {
        require_key_eq(
            player_profile.owner,
            &NICECHUNK_PLAYER_PROGRAM_ID,
            NicechunkChunkError::InvalidPlayerProgram,
        )?;
        require_key_eq(
            player_session.owner,
            &NICECHUNK_PLAYER_PROGRAM_ID,
            NicechunkChunkError::InvalidPlayerProgram,
        )?;

        let clock = Clock::get()?;
        let player_session_data = player_session.try_borrow_data()?;
        let session = PlayerSessionView::validate(
            &player_session_data,
            payer.key,
            player_profile.key,
            global_config.key,
            1,
            clock.unix_timestamp,
        )?;
        drop(player_session_data);

        let player_profile_data = player_profile.try_borrow_data()?;
        PlayerProfileView::validate(&player_profile_data, &session.owner, global_config.key)?;
        drop(player_profile_data);
    }

    let (chunk_x, chunk_z, local_x, local_z) = args.chunk_coords(&global_config_view)?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;

    create_or_grow_chunk_broken_if_needed(
        payer,
        chunk_broken,
        global_config,
        system_program_account,
        program_id,
        global_config_view.min_build_y,
        chunk_x,
        chunk_z,
        bump,
        false,
    )?;

    let packed = pack_broken_coord(
        local_x,
        args.world_y,
        local_z,
        global_config_view.min_build_y,
    )?;
    {
        let data = chunk_broken.try_borrow_data()?;
        ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
        if ChunkBrokenState::contains_packed(&data, packed)? {
            msg!(
                "NCKM already_mined {} {} {}",
                args.world_x,
                args.world_y,
                args.world_z
            );
            return Err(NicechunkChunkError::BlockAlreadyMined.into());
        }
    }

    let block_id = generated_block_id_at(&global_config_view, &generated_args);
    let inspect_only = args.expected_block_id == MineBlockArgs::INSPECT_ONLY_EXPECTED_BLOCK_ID;
    if !inspect_only && args.expected_block_id != block_id {
        msg!(
            "NCKM mismatch expected={} actual={}",
            args.expected_block_id,
            block_id
        );
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if !inspect_only && matches!(block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
        msg!("NCKM unmineable block_id={}", block_id);
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    {
        let data = chunk_broken.try_borrow_data()?;
        let (count, capacity) =
            ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
        if count >= capacity {
            drop(data);
            create_or_grow_chunk_broken_if_needed(
                payer,
                chunk_broken,
                global_config,
                system_program_account,
                program_id,
                global_config_view.min_build_y,
                chunk_x,
                chunk_z,
                bump,
                true,
            )?;
        }
    }

    let mut data = chunk_broken.try_borrow_mut_data()?;
    if ChunkBrokenState::contains_packed(&data, packed)? {
        msg!(
            "NCKM already_mined {} {} {}",
            args.world_x,
            args.world_y,
            args.world_z
        );
        return Err(NicechunkChunkError::BlockAlreadyMined.into());
    }
    ChunkBrokenState::append_packed(&mut data, global_config_view.min_build_y, packed)?;
    msg!(
        "NCKM mined {} {} {} {} {} {}",
        chunk_x,
        chunk_z,
        args.world_x,
        args.world_y,
        args.world_z,
        block_id
    );
    Ok(())
}

fn validate_previous_block_id(
    chunk_data: &[u8],
    global_config: &GlobalConfigView,
    args: &BlockChangeArgs,
) -> ProgramResult {
    let current_block_id = ChunkState::current_block_id_at(chunk_data, global_config, args)?;
    if args.previous_block_id != current_block_id {
        msg!(
            "NCK block change mismatch: expected_previous={}, actual_current={}",
            args.previous_block_id,
            current_block_id
        );
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    Ok(())
}

fn validate_global_config(
    global_config: &AccountInfo,
) -> Result<GlobalConfigView, solana_program::program_error::ProgramError> {
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkChunkError::InvalidGlobalConfigOwner,
    )?;
    let data = global_config.try_borrow_data()?;
    GlobalConfigView::unpack(&data).map_err(Into::into)
}

fn validate_chunk_pda(
    program_id: &Pubkey,
    chunk: &Pubkey,
    global_config: &Pubkey,
    chunk_x: i32,
    chunk_z: i32,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let (expected_chunk, bump) = Pubkey::find_program_address(
        &[
            CHUNK_SEED,
            global_config.as_ref(),
            &chunk_x_bytes,
            &chunk_z_bytes,
        ],
        program_id,
    );
    require_key_eq(chunk, &expected_chunk, NicechunkChunkError::InvalidChunkPda)?;
    Ok(bump)
}

fn validate_chunk_broken_pda(
    program_id: &Pubkey,
    chunk_broken: &Pubkey,
    global_config: &Pubkey,
    chunk_x: i32,
    chunk_z: i32,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let (expected_chunk, bump) = Pubkey::find_program_address(
        &[
            CHUNK_BROKEN_SEED,
            global_config.as_ref(),
            &chunk_x_bytes,
            &chunk_z_bytes,
        ],
        program_id,
    );
    require_key_eq(
        chunk_broken,
        &expected_chunk,
        NicechunkChunkError::InvalidChunkBrokenPda,
    )?;
    Ok(bump)
}

#[allow(clippy::too_many_arguments)]
fn create_or_grow_chunk_broken_if_needed<'a>(
    payer: &AccountInfo<'a>,
    chunk_broken: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    min_build_y: i16,
    chunk_x: i32,
    chunk_z: i32,
    bump: u8,
    force_grow: bool,
) -> ProgramResult {
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let seeds = &[
        CHUNK_BROKEN_SEED,
        global_config.key.as_ref(),
        &chunk_x_bytes,
        &chunk_z_bytes,
        &[bump],
    ];

    if chunk_broken.owner == program_id {
        let capacity = {
            let data = chunk_broken.try_borrow_data()?;
            let (_count, capacity) = ChunkBrokenState::validate_header(&data, min_build_y)?;
            capacity
        };
        if !force_grow {
            return Ok(());
        }
        if capacity >= CHUNK_BROKEN_MAX_CAPACITY {
            return Err(NicechunkChunkError::ChunkBrokenCapacityExceeded.into());
        }
        let next_capacity = capacity
            .saturating_add(CHUNK_BROKEN_GROW_BY)
            .min(CHUNK_BROKEN_MAX_CAPACITY);
        fund_account_to_rent_exempt(
            payer,
            chunk_broken,
            system_program_account,
            ChunkBrokenState::len_for_capacity(next_capacity),
        )?;
        chunk_broken.realloc(ChunkBrokenState::len_for_capacity(next_capacity), false)?;
        {
            let mut data = chunk_broken.try_borrow_mut_data()?;
            data[8..10].copy_from_slice(&next_capacity.to_le_bytes());
        }
        return Ok(());
    }

    if chunk_broken.owner != &system_program::ID || chunk_broken.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }

    let initial_len = ChunkBrokenState::len_for_capacity(CHUNK_BROKEN_INITIAL_CAPACITY);
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(initial_len);

    if chunk_broken.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            chunk_broken.key,
            lamports,
            initial_len as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                chunk_broken.clone(),
                system_program_account.clone(),
            ],
            &[seeds],
        )?;
    } else {
        if chunk_broken.lamports() < lamports {
            let delta = lamports - chunk_broken.lamports();
            let transfer = system_instruction::transfer(payer.key, chunk_broken.key, delta);
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    chunk_broken.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }

        let allocate = system_instruction::allocate(chunk_broken.key, initial_len as u64);
        invoke_signed(
            &allocate,
            &[chunk_broken.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_instruction::assign(chunk_broken.key, program_id);
        invoke_signed(
            &assign,
            &[chunk_broken.clone(), system_program_account.clone()],
            &[seeds],
        )?;
    }

    let mut data = chunk_broken.try_borrow_mut_data()?;
    ChunkBrokenState::pack_empty(
        &mut data,
        &ChunkBrokenInitArgs {
            bump,
            min_y: min_build_y,
            capacity: CHUNK_BROKEN_INITIAL_CAPACITY,
        },
    )
}

fn fund_account_to_rent_exempt<'a>(
    payer: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    len: usize,
) -> ProgramResult {
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(len);
    if target.lamports() >= required_lamports {
        return Ok(());
    }
    let delta = required_lamports - target.lamports();
    let transfer = system_instruction::transfer(payer.key, target.key, delta);
    invoke(
        &transfer,
        &[
            payer.clone(),
            target.clone(),
            system_program_account.clone(),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
fn create_chunk_if_needed<'a>(
    payer: &AccountInfo<'a>,
    chunk: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    global_config_view: &GlobalConfigView,
    chunk_x: i32,
    chunk_z: i32,
    bump: u8,
) -> ProgramResult {
    if chunk.owner == program_id || chunk.owner == &MAGICBLOCK_DELEGATION_PROGRAM_ID {
        let data = chunk.try_borrow_data()?;
        ChunkState::validate_header(
            &data,
            global_config.key,
            global_config_view.world_id,
            chunk_x,
            chunk_z,
        )?;
        return Ok(());
    }
    if chunk.owner != &system_program::ID || chunk.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(ChunkState::LEN);
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let seeds = &[
        CHUNK_SEED,
        global_config.key.as_ref(),
        &chunk_x_bytes,
        &chunk_z_bytes,
        &[bump],
    ];

    if chunk.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            chunk.key,
            lamports,
            ChunkState::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[payer.clone(), chunk.clone(), system_program_account.clone()],
            &[seeds],
        )?;
    } else {
        if chunk.lamports() < lamports {
            let delta = lamports - chunk.lamports();
            let transfer = system_instruction::transfer(payer.key, chunk.key, delta);
            invoke(
                &transfer,
                &[payer.clone(), chunk.clone(), system_program_account.clone()],
            )?;
        }

        let allocate = system_instruction::allocate(chunk.key, ChunkState::LEN as u64);
        invoke_signed(
            &allocate,
            &[chunk.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_instruction::assign(chunk.key, program_id);
        invoke_signed(
            &assign,
            &[chunk.clone(), system_program_account.clone()],
            &[seeds],
        )?;
    }

    let clock = Clock::get()?;
    let mut data = chunk.try_borrow_mut_data()?;
    ChunkState::pack_empty(
        &mut data,
        &ChunkInitArgs {
            bump,
            global_config: global_config.key,
            world_id: global_config_view.world_id,
            chunk_x,
            chunk_z,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn validate_delegation_pdas(
    program_id: &Pubkey,
    chunk: &Pubkey,
    delegate_buffer: &Pubkey,
    delegation_record: &Pubkey,
    delegation_metadata: &Pubkey,
) -> ProgramResult {
    let (expected_buffer, _) =
        Pubkey::find_program_address(&[b"buffer", chunk.as_ref()], program_id);
    let (expected_record, _) = Pubkey::find_program_address(
        &[b"delegation", chunk.as_ref()],
        &MAGICBLOCK_DELEGATION_PROGRAM_ID,
    );
    let (expected_metadata, _) = Pubkey::find_program_address(
        &[b"delegation-metadata", chunk.as_ref()],
        &MAGICBLOCK_DELEGATION_PROGRAM_ID,
    );
    require_key_eq(
        delegate_buffer,
        &expected_buffer,
        NicechunkChunkError::InvalidDelegationAccount,
    )?;
    require_key_eq(
        delegation_record,
        &expected_record,
        NicechunkChunkError::InvalidDelegationAccount,
    )?;
    require_key_eq(
        delegation_metadata,
        &expected_metadata,
        NicechunkChunkError::InvalidDelegationAccount,
    )
}

#[allow(clippy::too_many_arguments)]
fn delegate_chunk_to_magicblock<'a>(
    payer: &AccountInfo<'a>,
    chunk: &AccountInfo<'a>,
    owner_program: &AccountInfo<'a>,
    delegate_buffer: &AccountInfo<'a>,
    delegation_record: &AccountInfo<'a>,
    delegation_metadata: &AccountInfo<'a>,
    delegation_program: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    global_config: &Pubkey,
    chunk_x: i32,
    chunk_z: i32,
    bump: u8,
    commit_frequency_ms: u32,
) -> ProgramResult {
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let chunk_seeds = &[
        CHUNK_SEED,
        global_config.as_ref(),
        &chunk_x_bytes,
        &chunk_z_bytes,
        &[bump],
    ];

    create_or_reuse_delegate_buffer(
        payer,
        delegate_buffer,
        system_program_account,
        owner_program.key,
        chunk.key,
    )?;

    {
        let chunk_data = chunk.try_borrow_data()?;
        let mut buffer_data = delegate_buffer.try_borrow_mut_data()?;
        buffer_data.copy_from_slice(&chunk_data);
    }
    chunk.try_borrow_mut_data()?.fill(0);

    chunk.assign(&system_program::ID);
    let assign = system_instruction::assign(chunk.key, delegation_program.key);
    invoke_signed(
        &assign,
        &[chunk.clone(), system_program_account.clone()],
        &[chunk_seeds],
    )?;

    let delegate_ix = Instruction {
        program_id: *delegation_program.key,
        accounts: vec![
            AccountMeta::new(*payer.key, true),
            AccountMeta::new(*chunk.key, true),
            AccountMeta::new_readonly(*owner_program.key, false),
            AccountMeta::new(*delegate_buffer.key, false),
            AccountMeta::new(*delegation_record.key, false),
            AccountMeta::new(*delegation_metadata.key, false),
            AccountMeta::new_readonly(*system_program_account.key, false),
        ],
        data: serialize_delegate_with_any_validator_data(
            commit_frequency_ms,
            global_config,
            &chunk_x_bytes,
            &chunk_z_bytes,
        ),
    };
    invoke_signed(
        &delegate_ix,
        &[
            payer.clone(),
            chunk.clone(),
            owner_program.clone(),
            delegate_buffer.clone(),
            delegation_record.clone(),
            delegation_metadata.clone(),
            system_program_account.clone(),
        ],
        &[chunk_seeds],
    )?;

    close_program_owned_buffer(delegate_buffer, payer)
}

fn create_or_reuse_delegate_buffer<'a>(
    payer: &AccountInfo<'a>,
    delegate_buffer: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    chunk: &Pubkey,
) -> ProgramResult {
    let (expected_buffer, bump) =
        Pubkey::find_program_address(&[b"buffer", chunk.as_ref()], program_id);
    require_key_eq(
        delegate_buffer.key,
        &expected_buffer,
        NicechunkChunkError::InvalidDelegationAccount,
    )?;

    if delegate_buffer.owner == program_id && delegate_buffer.data_len() == ChunkState::LEN {
        return Ok(());
    }
    if delegate_buffer.owner != &system_program::ID || delegate_buffer.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(ChunkState::LEN);
    let seeds = &[b"buffer", chunk.as_ref(), &[bump]];
    if delegate_buffer.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            delegate_buffer.key,
            lamports,
            ChunkState::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                delegate_buffer.clone(),
                system_program_account.clone(),
            ],
            &[seeds],
        )?;
    } else {
        if delegate_buffer.lamports() < lamports {
            let delta = lamports - delegate_buffer.lamports();
            let transfer = system_instruction::transfer(payer.key, delegate_buffer.key, delta);
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    delegate_buffer.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        let allocate = system_instruction::allocate(delegate_buffer.key, ChunkState::LEN as u64);
        invoke_signed(
            &allocate,
            &[delegate_buffer.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_instruction::assign(delegate_buffer.key, program_id);
        invoke_signed(
            &assign,
            &[delegate_buffer.clone(), system_program_account.clone()],
            &[seeds],
        )?;
    }
    Ok(())
}

fn close_program_owned_buffer(buffer: &AccountInfo, payer: &AccountInfo) -> ProgramResult {
    let lamports = buffer.lamports();
    **payer.try_borrow_mut_lamports()? = payer
        .lamports()
        .checked_add(lamports)
        .ok_or(NicechunkChunkError::InvalidDelegationAccount)?;
    **buffer.try_borrow_mut_lamports()? = 0;
    buffer.try_borrow_mut_data()?.fill(0);
    Ok(())
}

fn serialize_delegate_with_any_validator_data(
    commit_frequency_ms: u32,
    global_config: &Pubkey,
    chunk_x_bytes: &[u8; 4],
    chunk_z_bytes: &[u8; 4],
) -> Vec<u8> {
    let seeds = [
        CHUNK_SEED,
        global_config.as_ref(),
        chunk_x_bytes.as_ref(),
        chunk_z_bytes.as_ref(),
    ];
    let mut data = Vec::with_capacity(96);
    data.extend_from_slice(&19_u64.to_le_bytes());
    data.extend_from_slice(&commit_frequency_ms.to_le_bytes());
    data.extend_from_slice(&(seeds.len() as u32).to_le_bytes());
    for seed in seeds {
        data.extend_from_slice(&(seed.len() as u32).to_le_bytes());
        data.extend_from_slice(seed);
    }
    data.push(0);
    data
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
