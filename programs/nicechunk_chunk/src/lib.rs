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
    NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkChunkError};
use state::{
    extra_drop_at, generated_block_id_at, generated_tree_fell_blocks, is_tree_leaf_block,
    is_tree_trunk_block, pack_backpack_resource_y, pack_broken_coord, unpack_resource_drop_rules,
    ChunkBrokenInitArgs, ChunkBrokenState, GlobalConfigView, MineBlockArgs, PlayerProfileView,
    PlayerProgressInitArgs, PlayerProgressState, PlayerSessionView, ResourceDropRule,
    ResourceDropTableState, TreeFellBlock, BLOCK_AIR, BLOCK_BEDROCK, BLOCK_WATER,
    CHUNK_BROKEN_GROW_BY, CHUNK_BROKEN_INITIAL_CAPACITY, CHUNK_BROKEN_MAX_CAPACITY,
    CHUNK_BROKEN_SEED, EXPLORATION_XP_PER_EXTRA_DROP, PLAYER_PROGRESS_LEN, PLAYER_PROGRESS_SEED,
    PRECISION_GATHERING_XP_PER_BLOCK, RESOURCE_DROP_RULE_LEN, RESOURCE_DROP_TABLE_SEED,
    TREE_FELL_MAX_CHUNKS,
};

declare_id!("7JD6kASAfQeiVLUi51mrfWSbeh96ntRJnRiFQKCqUVhn");

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
        5 => mine_block(program_id, accounts, payload),
        6 => initialize_chunk_broken(program_id, accounts, payload),
        7 => initialize_resource_drop_table(program_id, accounts, payload),
        8 => mine_block_with_rewards(program_id, accounts, payload),
        9 => fell_tree_with_rewards(program_id, accounts, payload),
        _ => Err(NicechunkChunkError::InvalidInstruction.into()),
    }
}

fn mine_block(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 6 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = MineBlockArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !session_authority.is_signer || !session_authority.is_writable {
        return Err(NicechunkChunkError::InvalidSessionAuthority.into());
    }
    if !chunk_broken.is_writable {
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
    let generated_args = args.generated_args(&global_config_view)?;
    generated_args.validate(&global_config_view)?;

    let clock = Clock::get()?;
    let player_session_data = player_session.try_borrow_data()?;
    let session = PlayerSessionView::validate(
        &player_session_data,
        session_authority.key,
        player_profile.key,
        global_config.key,
        1,
        clock.unix_timestamp,
    )?;
    drop(player_session_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate(&player_profile_data, &session.owner, global_config.key)?;
    drop(player_profile_data);

    let (chunk_x, chunk_z, local_x, local_z) = args.chunk_coords(&global_config_view)?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;

    let block_id = generated_block_id_at(&global_config_view, &generated_args);
    if args.expected_block_id != block_id {
        msg!(
            "NCKM mismatch x={} y={} z={} cx={} cz={} lx={} lz={} expected={} actual={}",
            args.world_x,
            args.world_y,
            args.world_z,
            chunk_x,
            chunk_z,
            local_x,
            local_z,
            args.expected_block_id,
            block_id
        );
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if matches!(block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    create_or_grow_chunk_broken_if_needed(
        session_authority,
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

    let already_mined = {
        let data = chunk_broken.try_borrow_data()?;
        ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
        ChunkBrokenState::contains_packed(&data, packed)?
    };
    if already_mined {
        return Err(NicechunkChunkError::BlockAlreadyMined.into());
    }

    {
        let data = chunk_broken.try_borrow_data()?;
        let (count, capacity) =
            ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
        if count >= capacity {
            drop(data);
            create_or_grow_chunk_broken_if_needed(
                session_authority,
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
    ChunkBrokenState::append_packed(&mut data, global_config_view.min_build_y, packed)
}

fn mine_block_with_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 10 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = MineBlockArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let player_progress = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let resource_drop_table = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !session_authority.is_signer || !session_authority.is_writable {
        return Err(NicechunkChunkError::InvalidSessionAuthority.into());
    }
    if !player_progress.is_writable || !chunk_broken.is_writable || !backpack.is_writable {
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
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkChunkError::InvalidBackpackProgram,
    )?;
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkChunkError::InvalidBackpackOwner,
    )?;

    let global_config_view = validate_global_config(global_config)?;
    let generated_args = args.generated_args(&global_config_view)?;
    generated_args.validate(&global_config_view)?;

    let clock = Clock::get()?;
    let player_session_data = player_session.try_borrow_data()?;
    let session = PlayerSessionView::validate(
        &player_session_data,
        session_authority.key,
        player_profile.key,
        global_config.key,
        1,
        clock.unix_timestamp,
    )?;
    drop(player_session_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate(&player_profile_data, &session.owner, global_config.key)?;
    drop(player_profile_data);

    let progress_bump = validate_player_progress_pda(
        program_id,
        player_progress.key,
        global_config.key,
        &session.owner,
    )?;
    create_player_progress_if_needed(
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        program_id,
        &session.owner,
        progress_bump,
        &clock,
    )?;
    let (gathered_volume_mm3, exploration_xp) = {
        let data = player_progress.try_borrow_data()?;
        let progress = PlayerProgressState::validate(&data, &session.owner, global_config.key)?;
        (
            PlayerProgressState::precision_gathering_volume_mm3_from_xp(
                progress.precision_gathering_xp,
            ),
            progress.exploration_xp,
        )
    };

    let (chunk_x, chunk_z, local_x, local_z) = args.chunk_coords(&global_config_view)?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;
    validate_resource_drop_table_pda(program_id, resource_drop_table.key, global_config.key)?;

    let block_id = generated_block_id_at(&global_config_view, &generated_args);
    if args.expected_block_id != block_id {
        msg!(
            "NCKM mismatch x={} y={} z={} cx={} cz={} lx={} lz={} expected={} actual={}",
            args.world_x,
            args.world_y,
            args.world_z,
            chunk_x,
            chunk_z,
            local_x,
            local_z,
            args.expected_block_id,
            block_id
        );
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if matches!(block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    let rules = {
        require_key_eq(
            resource_drop_table.owner,
            program_id,
            NicechunkChunkError::InvalidResourceDropTableData,
        )?;
        let data = resource_drop_table.try_borrow_data()?;
        unpack_resource_drop_rules(&data)?
    };

    create_or_grow_chunk_broken_if_needed(
        session_authority,
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

    let already_mined = {
        let data = chunk_broken.try_borrow_data()?;
        ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
        ChunkBrokenState::contains_packed(&data, packed)?
    };
    if already_mined {
        return Err(NicechunkChunkError::BlockAlreadyMined.into());
    }

    {
        let data = chunk_broken.try_borrow_data()?;
        let (count, capacity) =
            ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
        if count >= capacity {
            drop(data);
            create_or_grow_chunk_broken_if_needed(
                session_authority,
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

    {
        let mut data = chunk_broken.try_borrow_mut_data()?;
        ChunkBrokenState::append_packed(&mut data, global_config_view.min_build_y, packed)?;
    }

    append_backpack_block_resource(
        backpack_program,
        session_authority,
        player_profile,
        player_session,
        backpack,
        args.world_x,
        pack_backpack_resource_y(args.world_y, block_id, global_config_view.min_build_y),
        args.world_z,
        gathered_volume_mm3,
    )?;

    let extra_drop = extra_drop_at(
        &global_config_view,
        &rules,
        args.world_x,
        args.world_y,
        args.world_z,
        block_id,
        exploration_xp,
    );
    if let Some(drop) = extra_drop {
        append_backpack_block_resource(
            backpack_program,
            session_authority,
            player_profile,
            player_session,
            backpack,
            args.world_x,
            pack_backpack_resource_y(args.world_y, drop.block_id, global_config_view.min_build_y),
            args.world_z,
            drop.volume_mm3,
        )?;
    }

    {
        let mut data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_precision_gathering_xp(
            &mut data,
            &session.owner,
            global_config.key,
            PRECISION_GATHERING_XP_PER_BLOCK,
            clock.slot,
        )?;
        if extra_drop.is_some() {
            PlayerProgressState::add_exploration_xp(
                &mut data,
                &session.owner,
                global_config.key,
                EXPLORATION_XP_PER_EXTRA_DROP,
                clock.slot,
            )?;
        }
    }

    Ok(())
}

fn fell_tree_with_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() < 9 || accounts.len() > 8 + TREE_FELL_MAX_CHUNKS {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = MineBlockArgs::unpack(payload)?;
    let session_authority = &accounts[0];
    let player_profile = &accounts[1];
    let player_session = &accounts[2];
    let player_progress = &accounts[3];
    let global_config = &accounts[4];
    let backpack_program = &accounts[5];
    let backpack = &accounts[6];
    let system_program_account = &accounts[7];
    let chunk_accounts = &accounts[8..];

    if !session_authority.is_signer || !session_authority.is_writable {
        return Err(NicechunkChunkError::InvalidSessionAuthority.into());
    }
    if !player_progress.is_writable
        || !backpack.is_writable
        || chunk_accounts.iter().any(|account| !account.is_writable)
    {
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
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkChunkError::InvalidBackpackProgram,
    )?;
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkChunkError::InvalidBackpackOwner,
    )?;

    let global_config_view = validate_global_config(global_config)?;
    let generated_args = args.generated_args(&global_config_view)?;
    generated_args.validate(&global_config_view)?;

    let clock = Clock::get()?;
    let player_session_data = player_session.try_borrow_data()?;
    let session = PlayerSessionView::validate(
        &player_session_data,
        session_authority.key,
        player_profile.key,
        global_config.key,
        1,
        clock.unix_timestamp,
    )?;
    drop(player_session_data);

    let player_profile_data = player_profile.try_borrow_data()?;
    PlayerProfileView::validate(&player_profile_data, &session.owner, global_config.key)?;
    drop(player_profile_data);

    let progress_bump = validate_player_progress_pda(
        program_id,
        player_progress.key,
        global_config.key,
        &session.owner,
    )?;
    create_player_progress_if_needed(
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        program_id,
        &session.owner,
        progress_bump,
        &clock,
    )?;
    let gathered_volume_mm3 = {
        let data = player_progress.try_borrow_data()?;
        let progress = PlayerProgressState::validate(&data, &session.owner, global_config.key)?;
        PlayerProgressState::precision_gathering_volume_mm3_from_xp(progress.precision_gathering_xp)
    };

    let cut_block_id = generated_block_id_at(&global_config_view, &generated_args);
    if args.expected_block_id != cut_block_id {
        msg!(
            "NCKM tree mismatch x={} y={} z={} expected={} actual={}",
            args.world_x,
            args.world_y,
            args.world_z,
            args.expected_block_id,
            cut_block_id
        );
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if !is_tree_trunk_block(cut_block_id) {
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    let blocks = generated_tree_fell_blocks(
        &global_config_view,
        args.world_x,
        args.world_y,
        args.world_z,
    )?;
    let chunks = tree_fell_chunks(&global_config_view, &blocks)?;
    if chunks.len() > chunk_accounts.len() {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let mut used_accounts = [false; TREE_FELL_MAX_CHUNKS];
    let mut chunk_account_indexes = Vec::with_capacity(chunks.len());
    for (chunk_x, chunk_z) in &chunks {
        let mut matched: Option<usize> = None;
        for (index, account) in chunk_accounts.iter().enumerate() {
            if used_accounts[index] {
                continue;
            }
            if validate_chunk_broken_pda(
                program_id,
                account.key,
                global_config.key,
                *chunk_x,
                *chunk_z,
            )
            .is_ok()
            {
                matched = Some(index);
                used_accounts[index] = true;
                break;
            }
        }
        let index = matched.ok_or(NicechunkChunkError::InvalidChunkBrokenPda)?;
        chunk_account_indexes.push(index);
    }

    for ((chunk_x, chunk_z), account_index) in chunks.iter().zip(chunk_account_indexes.iter()) {
        let account = &chunk_accounts[*account_index];
        let bump = validate_chunk_broken_pda(
            program_id,
            account.key,
            global_config.key,
            *chunk_x,
            *chunk_z,
        )?;
        create_or_grow_chunk_broken_if_needed(
            session_authority,
            account,
            global_config,
            system_program_account,
            program_id,
            global_config_view.min_build_y,
            *chunk_x,
            *chunk_z,
            bump,
            false,
        )?;
    }

    let mut rewards = Vec::with_capacity(blocks.len());
    let mut fell_leaf_count: usize = 0;
    for block in &blocks {
        let (chunk_x, chunk_z, local_x, local_z) =
            tree_block_chunk_coords(&global_config_view, block)?;
        let chunk_index = chunks
            .iter()
            .position(|(x, z)| *x == chunk_x && *z == chunk_z)
            .ok_or(NicechunkChunkError::InvalidChunkBrokenPda)?;
        let account = &chunk_accounts[chunk_account_indexes[chunk_index]];
        let packed = pack_broken_coord(
            local_x,
            block.world_y,
            local_z,
            global_config_view.min_build_y,
        )?;

        if block.world_x == args.world_x
            && block.world_y == args.world_y
            && block.world_z == args.world_z
        {
            let data = account.try_borrow_data()?;
            ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
            if ChunkBrokenState::contains_packed(&data, packed)? {
                return Err(NicechunkChunkError::BlockAlreadyMined.into());
            }
        }

        {
            let data = account.try_borrow_data()?;
            let (count, capacity) =
                ChunkBrokenState::validate_header(&data, global_config_view.min_build_y)?;
            if count >= capacity {
                drop(data);
                let bump = validate_chunk_broken_pda(
                    program_id,
                    account.key,
                    global_config.key,
                    chunk_x,
                    chunk_z,
                )?;
                create_or_grow_chunk_broken_if_needed(
                    session_authority,
                    account,
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

        {
            let mut data = account.try_borrow_mut_data()?;
            ChunkBrokenState::append_packed(&mut data, global_config_view.min_build_y, packed)?;
        }

        if is_tree_trunk_block(block.block_id) {
            rewards.push(*block);
        } else if is_tree_leaf_block(block.block_id) {
            fell_leaf_count = fell_leaf_count.saturating_add(1);
            if fell_leaf_count % 5 == 0 {
                rewards.push(*block);
            }
        }
    }

    append_backpack_block_resources_batch(
        backpack_program,
        session_authority,
        player_profile,
        player_session,
        backpack,
        &rewards,
        global_config_view.min_build_y,
        gathered_volume_mm3,
    )?;

    if !rewards.is_empty() {
        let mut data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_precision_gathering_xp(
            &mut data,
            &session.owner,
            global_config.key,
            PRECISION_GATHERING_XP_PER_BLOCK.saturating_mul(rewards.len() as u64),
            clock.slot,
        )?;
    }

    Ok(())
}

fn initialize_resource_drop_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.is_empty() {
        return Err(NicechunkChunkError::InvalidInstruction.into());
    }

    let rule_count = payload[0] as usize;
    if rule_count == 0 || payload.len() != 1 + rule_count * RESOURCE_DROP_RULE_LEN {
        return Err(NicechunkChunkError::InvalidResourceDropTableData.into());
    }

    let mut rules = Vec::with_capacity(rule_count);
    for index in 0..rule_count {
        let offset = 1 + index * RESOURCE_DROP_RULE_LEN;
        rules.push(ResourceDropRule::unpack(
            &payload[offset..offset + RESOURCE_DROP_RULE_LEN],
        )?);
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let resource_drop_table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkChunkError::InvalidPayer.into());
    }
    if !resource_drop_table.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;
    validate_global_config(global_config)?;
    let bump =
        validate_resource_drop_table_pda(program_id, resource_drop_table.key, global_config.key)?;
    if resource_drop_table.owner == program_id {
        return Err(NicechunkChunkError::InvalidResourceDropTableData.into());
    }
    if resource_drop_table.owner != &system_program::ID || resource_drop_table.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }

    let len = ResourceDropTableState::len_for_rules(rules.len());
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(len);
    let seeds = &[
        RESOURCE_DROP_TABLE_SEED,
        global_config.key.as_ref(),
        &[bump],
    ];
    let create = system_instruction::create_account(
        payer.key,
        resource_drop_table.key,
        lamports,
        len as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            payer.clone(),
            resource_drop_table.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )?;

    let mut data = resource_drop_table.try_borrow_mut_data()?;
    ResourceDropTableState::pack(&mut data, bump, &rules)
}

fn initialize_chunk_broken(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }
    if payload.len() != 8 {
        return Err(NicechunkChunkError::InvalidInstruction.into());
    }

    let chunk_x = i32::from_le_bytes(
        payload[0..4]
            .try_into()
            .map_err(|_| NicechunkChunkError::InvalidInstruction)?,
    );
    let chunk_z = i32::from_le_bytes(
        payload[4..8]
            .try_into()
            .map_err(|_| NicechunkChunkError::InvalidInstruction)?,
    );

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkChunkError::InvalidSessionAuthority.into());
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
    )
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

fn validate_resource_drop_table_pda(
    program_id: &Pubkey,
    resource_drop_table: &Pubkey,
    global_config: &Pubkey,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let (expected_table, bump) = Pubkey::find_program_address(
        &[RESOURCE_DROP_TABLE_SEED, global_config.as_ref()],
        program_id,
    );
    require_key_eq(
        resource_drop_table,
        &expected_table,
        NicechunkChunkError::InvalidResourceDropTablePda,
    )?;
    Ok(bump)
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
        NicechunkChunkError::InvalidPlayerProgress,
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
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
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

fn append_backpack_block_resource<'a>(
    backpack_program: &AccountInfo<'a>,
    session_authority: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    player_session: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    world_x: i32,
    packed_y: i16,
    world_z: i32,
    volume_mm3: u32,
) -> ProgramResult {
    let mut data = [0_u8; 15];
    data[0] = 1;
    data[1..5].copy_from_slice(&world_x.to_le_bytes());
    data[5..7].copy_from_slice(&packed_y.to_le_bytes());
    data[7..11].copy_from_slice(&world_z.to_le_bytes());
    data[11..15].copy_from_slice(&volume_mm3.to_le_bytes());
    let data = backpack_cpi_data(&data);
    let ix = Instruction {
        program_id: *backpack_program.key,
        accounts: vec![
            AccountMeta::new_readonly(*session_authority.key, true),
            AccountMeta::new_readonly(*player_profile.key, false),
            AccountMeta::new_readonly(*player_session.key, false),
            AccountMeta::new(*backpack.key, false),
        ],
        data,
    };
    invoke(
        &ix,
        &[
            session_authority.clone(),
            player_profile.clone(),
            player_session.clone(),
            backpack.clone(),
        ],
    )
}

fn append_backpack_block_resources_batch<'a>(
    backpack_program: &AccountInfo<'a>,
    session_authority: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    player_session: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    blocks: &[TreeFellBlock],
    min_y: i16,
    volume_mm3: u32,
) -> ProgramResult {
    if blocks.is_empty() {
        return Ok(());
    }

    let mut data = Vec::with_capacity(2 + blocks.len() * 14);
    data.push(6);
    data.push(blocks.len() as u8);
    for block in blocks {
        data.extend_from_slice(&block.world_x.to_le_bytes());
        data.extend_from_slice(
            &pack_backpack_resource_y(block.world_y, block.block_id, min_y).to_le_bytes(),
        );
        data.extend_from_slice(&block.world_z.to_le_bytes());
        data.extend_from_slice(&volume_mm3.to_le_bytes());
    }
    let data = backpack_cpi_data(&data);
    let ix = Instruction {
        program_id: *backpack_program.key,
        accounts: vec![
            AccountMeta::new_readonly(*session_authority.key, true),
            AccountMeta::new_readonly(*player_profile.key, false),
            AccountMeta::new_readonly(*player_session.key, false),
            AccountMeta::new(*backpack.key, false),
        ],
        data,
    };
    invoke(
        &ix,
        &[
            session_authority.clone(),
            player_profile.clone(),
            player_session.clone(),
            backpack.clone(),
        ],
    )
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

fn tree_block_chunk_coords(
    global_config: &GlobalConfigView,
    block: &TreeFellBlock,
) -> Result<(i32, i32, u8, u8), NicechunkChunkError> {
    if global_config.chunk_size != 16 {
        return Err(NicechunkChunkError::InvalidGlobalConfigData);
    }
    let chunk_size = global_config.chunk_size as i32;
    Ok((
        block.world_x.div_euclid(chunk_size),
        block.world_z.div_euclid(chunk_size),
        block.world_x.rem_euclid(chunk_size) as u8,
        block.world_z.rem_euclid(chunk_size) as u8,
    ))
}

fn tree_fell_chunks(
    global_config: &GlobalConfigView,
    blocks: &[TreeFellBlock],
) -> Result<Vec<(i32, i32)>, NicechunkChunkError> {
    let mut chunks = Vec::with_capacity(TREE_FELL_MAX_CHUNKS);
    for block in blocks {
        let (chunk_x, chunk_z, _, _) = tree_block_chunk_coords(global_config, block)?;
        if chunks.iter().any(|(x, z)| *x == chunk_x && *z == chunk_z) {
            continue;
        }
        if chunks.len() >= TREE_FELL_MAX_CHUNKS {
            return Err(NicechunkChunkError::InvalidAccountCount);
        }
        chunks.push((chunk_x, chunk_z));
    }
    Ok(chunks)
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
        let mut data = chunk_broken.try_borrow_mut_data()?;
        data[8..10].copy_from_slice(&next_capacity.to_le_bytes());
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
