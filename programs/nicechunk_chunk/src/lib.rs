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
    system_program,
    sysvar::Sysvar,
};

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

pub mod civilization_adapter;
pub mod cluster_config;
pub mod errors;
pub mod state;

use cluster_config::{
    NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_BUILDING_PROGRAM_ID,
    NICECHUNK_CIVILIZATION_PROGRAM_ID, NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkChunkError};
use state::{
    batch_mine_reward_passes, extra_drop_from_table, generated_block_id_at,
    generated_tree_fell_blocks, is_tree_leaf_block, is_tree_trunk_block, pack_backpack_resource_y,
    pack_broken_coord, range_mine_reward_passes, surface_decoration_from_table, BatchMineArgs,
    ChunkBrokenInitArgs, ChunkBrokenState, FoundationChunkState, FoundationChunkV2State,
    FoundationRecordV2, GlobalConfigView, MineBlockArgs, PlayerProfileView, PlayerProgressInitArgs,
    PlayerProgressState, PlayerSessionView, RangeMineArgs, ResourceDropTableState,
    SurfaceDecorationTableState, TreeFellBlock, BATCH_MINE_BASE_DROP_CHANCE_BPS,
    BATCH_MINE_DECORATION_DROP_CHANCE_BPS, BATCH_MINE_EXTRA_DROP_CHANCE_BPS, BLOCK_AIR,
    BLOCK_BEDROCK, BLOCK_WATER, CHUNK_BROKEN_GROW_BY, CHUNK_BROKEN_INITIAL_CAPACITY,
    CHUNK_BROKEN_MAX_CAPACITY, CHUNK_BROKEN_SEED, EXPLORATION_XP_PER_EXTRA_DROP,
    FOUNDATION_CHUNK_MAGIC, FOUNDATION_CHUNK_SEED, FOUNDATION_CHUNK_V2_GROWTH,
    FOUNDATION_CHUNK_V2_INITIAL_CAPACITY, FOUNDATION_CHUNK_V2_MAGIC,
    FOUNDATION_CHUNK_V2_MAX_CAPACITY, PLAYER_PROGRESS_LEN, PLAYER_PROGRESS_SEED,
    PRECISION_GATHERING_XP_PER_BLOCK, RANGE_MINE_BASE_DROP_CHANCE_BPS, RANGE_MINE_MAX_REWARDS,
    RANGE_MINE_SECONDARY_CANDIDATE_CHANCE_BPS, RANGE_MINE_SECONDARY_PROOF_LIMIT,
    RESOURCE_DROP_TABLE_SEED, SURFACE_DECORATION_FLAG_MINEABLE, SURFACE_DECORATION_TABLE_LEN,
    SURFACE_DECORATION_TABLE_SEED, TREE_FELL_MAX_CHUNKS,
};

declare_id!("GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj");

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
        10 => apply_civilization_resource_drop_receipt(program_id, accounts, payload),
        11 => initialize_surface_decoration_table(program_id, accounts, payload),
        12 => verify_surface_decoration(program_id, accounts, payload),
        13 => apply_civilization_surface_decoration_receipt(program_id, accounts, payload),
        15 => register_build_site_chunk(program_id, accounts, payload),
        20 => batch_mine_with_rewards(program_id, accounts, payload),
        21 => range_mine_with_rewards(program_id, accounts, payload),
        _ => Err(NicechunkChunkError::InvalidInstruction.into()),
    }
}

struct PlayerActionContext {
    config: GlobalConfigView,
    owner: Pubkey,
    clock: Clock,
}

fn mining_action_id(context: &PlayerActionContext, action_kind: u8, anchor: &MineBlockArgs) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in context
        .owner
        .as_ref()
        .iter()
        .copied()
        .chain(context.clock.slot.to_le_bytes())
        .chain([action_kind])
        .chain(anchor.world_x.to_le_bytes())
        .chain(anchor.world_y.to_le_bytes())
        .chain(anchor.world_z.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash.max(1)
}

fn validate_player_action(
    session_authority: &AccountInfo,
    player_profile: &AccountInfo,
    player_session: &AccountInfo,
    global_config: &AccountInfo,
    system_program_account: &AccountInfo,
) -> Result<PlayerActionContext, solana_program::program_error::ProgramError> {
    if !session_authority.is_signer || !session_authority.is_writable {
        return Err(NicechunkChunkError::InvalidSessionAuthority.into());
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
    let config = validate_global_config(global_config)?;
    let clock = Clock::get()?;
    let owner = {
        let data = player_session.try_borrow_data()?;
        PlayerSessionView::validate(
            &data,
            session_authority.key,
            player_profile.key,
            global_config.key,
            1,
            clock.unix_timestamp,
        )?
        .owner
    };
    {
        let data = player_profile.try_borrow_data()?;
        PlayerProfileView::validate(&data, &owner, global_config.key)?;
    }
    Ok(PlayerActionContext {
        config,
        owner,
        clock,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_player_progress<'a>(
    program_id: &Pubkey,
    session_authority: &AccountInfo<'a>,
    player_progress: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    context: &PlayerActionContext,
) -> Result<(u32, u64), solana_program::program_error::ProgramError> {
    if !player_progress.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    let bump = validate_player_progress_pda(
        program_id,
        player_progress.key,
        global_config.key,
        &context.owner,
    )?;
    create_player_progress_if_needed(
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        program_id,
        &context.owner,
        bump,
        &context.clock,
    )?;
    let data = player_progress.try_borrow_data()?;
    let progress = PlayerProgressState::validate(&data, &context.owner, global_config.key)?;
    Ok((
        PlayerProgressState::precision_gathering_volume_mm3_from_xp(
            progress.precision_gathering_xp,
        ),
        progress.exploration_xp,
    ))
}

fn validate_backpack(backpack_program: &AccountInfo, backpack: &AccountInfo) -> ProgramResult {
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkChunkError::InvalidBackpackProgram,
    )?;
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkChunkError::InvalidBackpackOwner,
    )
}

#[allow(clippy::too_many_arguments)]
fn record_mined_block<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    chunk_broken: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    min_build_y: i16,
    chunk_x: i32,
    chunk_z: i32,
    bump: u8,
    packed: [u8; 3],
) -> Result<bool, solana_program::program_error::ProgramError> {
    let created = create_or_grow_chunk_broken_if_needed(
        payer,
        chunk_broken,
        global_config,
        system_program_account,
        program_id,
        min_build_y,
        chunk_x,
        chunk_z,
        bump,
        false,
    )?;
    let (already_mined, count, capacity) = {
        let data = chunk_broken.try_borrow_data()?;
        let (count, capacity) = ChunkBrokenState::validate_header(&data, min_build_y)?;
        (
            ChunkBrokenState::contains_packed(&data, packed)?,
            count,
            capacity,
        )
    };
    if already_mined {
        return Err(NicechunkChunkError::BlockAlreadyMined.into());
    }
    if count >= capacity {
        create_or_grow_chunk_broken_if_needed(
            payer,
            chunk_broken,
            global_config,
            system_program_account,
            program_id,
            min_build_y,
            chunk_x,
            chunk_z,
            bump,
            true,
        )?;
    }
    let mut data = chunk_broken.try_borrow_mut_data()?;
    ChunkBrokenState::append_packed(&mut data, min_build_y, packed)?;
    Ok(created)
}

fn mine_block(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 7 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = MineBlockArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let foundation_chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !chunk_broken.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    let context = validate_player_action(
        session_authority,
        player_profile,
        player_session,
        global_config,
        system_program_account,
    )?;
    let global_config_view = &context.config;
    let generated_args = args.generated_args(global_config_view)?;
    generated_args.validate(global_config_view)?;

    let (chunk_x, chunk_z, local_x, local_z) = args.chunk_coords(global_config_view)?;
    reject_foundation_protected_block(
        program_id,
        foundation_chunk,
        global_config,
        chunk_x,
        chunk_z,
        &args,
    )?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;

    let block_id = generated_block_id_at(global_config_view, &generated_args);
    if args.expected_block_id != block_id {
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if matches!(block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    let packed = pack_broken_coord(
        local_x,
        args.world_y,
        local_z,
        global_config_view.min_build_y,
    )?;
    record_mined_block(
        program_id,
        session_authority,
        chunk_broken,
        global_config,
        system_program_account,
        global_config_view.min_build_y,
        chunk_x,
        chunk_z,
        bump,
        packed,
    )?;
    Ok(())
}

fn mine_block_with_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 13 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = MineBlockArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let player_progress = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let foundation_chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let resource_drop_table = next_account_info(account_info_iter)?;
    let surface_decoration_table = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !player_progress.is_writable || !chunk_broken.is_writable || !backpack.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    validate_backpack(backpack_program, backpack)?;

    let context = validate_player_action(
        session_authority,
        player_profile,
        player_session,
        global_config,
        system_program_account,
    )?;
    let global_config_view = &context.config;
    let action_id = mining_action_id(&context, 1, &args);
    let generated_args = args.generated_args(global_config_view)?;
    generated_args.validate(global_config_view)?;
    let (gathered_volume_mm3, exploration_xp) = load_player_progress(
        program_id,
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        &context,
    )?;

    let (chunk_x, chunk_z, local_x, local_z) = args.chunk_coords(global_config_view)?;
    reject_foundation_protected_block(
        program_id,
        foundation_chunk,
        global_config,
        chunk_x,
        chunk_z,
        &args,
    )?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;
    validate_rule_table_pda(
        program_id,
        resource_drop_table.key,
        global_config.key,
        RESOURCE_DROP_TABLE_SEED,
        NicechunkChunkError::InvalidResourceDropTablePda,
    )?;
    validate_rule_table_pda(
        program_id,
        surface_decoration_table.key,
        global_config.key,
        SURFACE_DECORATION_TABLE_SEED,
        NicechunkChunkError::InvalidSurfaceDecorationTablePda,
    )?;

    let block_id = generated_block_id_at(global_config_view, &generated_args);
    if args.expected_block_id != block_id {
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if matches!(block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    let extra_drop = {
        require_key_eq(
            resource_drop_table.owner,
            program_id,
            NicechunkChunkError::InvalidResourceDropTableData,
        )?;
        let data = resource_drop_table.try_borrow_data()?;
        extra_drop_from_table(
            global_config_view,
            &data,
            args.world_x,
            args.world_y,
            args.world_z,
            block_id,
            exploration_xp,
        )?
    };
    let surface_decoration_drop = {
        require_key_eq(
            surface_decoration_table.owner,
            program_id,
            NicechunkChunkError::InvalidSurfaceDecorationTableData,
        )?;
        let data = surface_decoration_table.try_borrow_data()?;
        surface_decoration_from_table(global_config_view, &data, args.world_x, args.world_z)?
            .filter(|entry| {
                entry.surface_y == args.world_y
                    && entry.surface_block_id == block_id
                    && entry.flags & SURFACE_DECORATION_FLAG_MINEABLE != 0
            })
    };

    let packed = pack_broken_coord(
        local_x,
        args.world_y,
        local_z,
        global_config_view.min_build_y,
    )?;
    let explored_chunk_count_delta = u32::from(record_mined_block(
        program_id,
        session_authority,
        chunk_broken,
        global_config,
        system_program_account,
        global_config_view.min_build_y,
        chunk_x,
        chunk_z,
        bump,
        packed,
    )?);

    append_backpack_block_resource(
        program_id,
        backpack_program,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        material_physics,
        chunk_x,
        chunk_z,
        bump,
        args.world_x,
        pack_backpack_resource_y(args.world_y, block_id, global_config_view.min_build_y),
        args.world_z,
        gathered_volume_mm3,
        action_id,
    )?;

    let mut secondary_rewards = Vec::with_capacity(2);
    // Visible surface decoration has priority over an invisible exploration
    // bonus when only one slot remains after the mined block is stored.
    if let Some(decoration) = surface_decoration_drop {
        secondary_rewards.push(BackpackBlockReward {
            world_x: args.world_x,
            packed_y: pack_backpack_resource_y(
                decoration.surface_y.saturating_add(1),
                decoration.drop_block_id,
                global_config_view.min_build_y,
            ),
            world_z: args.world_z,
            volume_mm3: gathered_volume_mm3,
            metadata: pack_surface_decoration_metadata(
                decoration.rule_id,
                decoration.decoration_id,
            ),
        });
    }
    if let Some(drop) = extra_drop {
        secondary_rewards.push(BackpackBlockReward {
            world_x: args.world_x,
            packed_y: pack_backpack_resource_y(
                args.world_y,
                drop.block_id,
                global_config_view.min_build_y,
            ),
            world_z: args.world_z,
            volume_mm3: drop.volume_mm3,
            metadata: 0,
        });
    }
    append_backpack_block_resources_lossy(
        program_id,
        backpack_program,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        material_physics,
        chunk_x,
        chunk_z,
        bump,
        &secondary_rewards,
        action_id,
    )?;

    {
        let mut data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_precision_gathering_xp(
            &mut data,
            &context.owner,
            global_config.key,
            PRECISION_GATHERING_XP_PER_BLOCK,
            context.clock.slot,
        )?;
        if extra_drop.is_some() {
            PlayerProgressState::add_exploration_xp(
                &mut data,
                &context.owner,
                global_config.key,
                EXPLORATION_XP_PER_EXTRA_DROP,
                context.clock.slot,
            )?;
        }
        PlayerProgressState::add_explored_chunk_count(
            &mut data,
            &context.owner,
            global_config.key,
            explored_chunk_count_delta,
            context.clock.slot,
        )?;
    }

    Ok(())
}

fn batch_mine_with_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 13 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = BatchMineArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let player_progress = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let foundation_chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let resource_drop_table = next_account_info(account_info_iter)?;
    let surface_decoration_table = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !player_progress.is_writable || !chunk_broken.is_writable || !backpack.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    validate_backpack(backpack_program, backpack)?;

    let context = validate_player_action(
        session_authority,
        player_profile,
        player_session,
        global_config,
        system_program_account,
    )?;
    let global_config_view = &context.config;
    let first = args
        .blocks
        .first()
        .ok_or(NicechunkChunkError::InvalidBatchMine)?;
    let action_id = mining_action_id(&context, 2, first);
    let (chunk_x, chunk_z, _, _) = first.chunk_coords(global_config_view)?;
    for block in &args.blocks {
        let (block_chunk_x, block_chunk_z, _, _) = block.chunk_coords(global_config_view)?;
        if block_chunk_x != chunk_x || block_chunk_z != chunk_z {
            return Err(NicechunkChunkError::BatchMineCrossChunk.into());
        }
    }

    let (gathered_volume_mm3, exploration_xp) = load_player_progress(
        program_id,
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        &context,
    )?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;
    validate_rule_table_pda(
        program_id,
        resource_drop_table.key,
        global_config.key,
        RESOURCE_DROP_TABLE_SEED,
        NicechunkChunkError::InvalidResourceDropTablePda,
    )?;
    validate_rule_table_pda(
        program_id,
        surface_decoration_table.key,
        global_config.key,
        SURFACE_DECORATION_TABLE_SEED,
        NicechunkChunkError::InvalidSurfaceDecorationTablePda,
    )?;
    require_key_eq(
        resource_drop_table.owner,
        program_id,
        NicechunkChunkError::InvalidResourceDropTableData,
    )?;
    require_key_eq(
        surface_decoration_table.owner,
        program_id,
        NicechunkChunkError::InvalidSurfaceDecorationTableData,
    )?;

    let resource_drop_data = resource_drop_table.try_borrow_data()?;
    let surface_decoration_data = surface_decoration_table.try_borrow_data()?;
    let mut primary_rewards = Vec::with_capacity(args.blocks.len());
    let mut secondary_rewards = Vec::with_capacity(args.blocks.len().saturating_mul(2));
    let mut explored_chunk_count_delta = 0_u32;
    let mut extra_drop_count = 0_u64;

    for block in &args.blocks {
        let generated_args = block.generated_args(global_config_view)?;
        generated_args.validate(global_config_view)?;
        let (_, _, local_x, local_z) = block.chunk_coords(global_config_view)?;
        reject_foundation_protected_block(
            program_id,
            foundation_chunk,
            global_config,
            chunk_x,
            chunk_z,
            block,
        )?;

        let block_id = generated_block_id_at(global_config_view, &generated_args);
        if block.expected_block_id != block_id {
            return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
        }
        if matches!(block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
            return Err(NicechunkChunkError::UnmineableBlock.into());
        }

        let extra_drop = extra_drop_from_table(
            global_config_view,
            &resource_drop_data,
            block.world_x,
            block.world_y,
            block.world_z,
            block_id,
            exploration_xp,
        )?;
        let surface_decoration = surface_decoration_from_table(
            global_config_view,
            &surface_decoration_data,
            block.world_x,
            block.world_z,
        )?
        .filter(|entry| {
            entry.surface_y == block.world_y
                && entry.surface_block_id == block_id
                && entry.flags & SURFACE_DECORATION_FLAG_MINEABLE != 0
        });

        let packed = pack_broken_coord(
            local_x,
            block.world_y,
            local_z,
            global_config_view.min_build_y,
        )?;
        explored_chunk_count_delta =
            explored_chunk_count_delta.saturating_add(u32::from(record_mined_block(
                program_id,
                session_authority,
                chunk_broken,
                global_config,
                system_program_account,
                global_config_view.min_build_y,
                chunk_x,
                chunk_z,
                bump,
                packed,
            )?));

        if batch_mine_reward_passes(
            global_config_view,
            block,
            u32::from(block_id),
            BATCH_MINE_BASE_DROP_CHANCE_BPS,
        ) {
            primary_rewards.push(BackpackBlockReward {
                world_x: block.world_x,
                packed_y: pack_backpack_resource_y(
                    block.world_y,
                    block_id,
                    global_config_view.min_build_y,
                ),
                world_z: block.world_z,
                volume_mm3: gathered_volume_mm3,
                metadata: 0,
            });
        }

        if let Some(decoration) = surface_decoration {
            if batch_mine_reward_passes(
                global_config_view,
                block,
                0x1_000_u32.saturating_add(u32::from(decoration.rule_id)),
                BATCH_MINE_DECORATION_DROP_CHANCE_BPS,
            ) {
                secondary_rewards.push(BackpackBlockReward {
                    world_x: block.world_x,
                    packed_y: pack_backpack_resource_y(
                        decoration.surface_y.saturating_add(1),
                        decoration.drop_block_id,
                        global_config_view.min_build_y,
                    ),
                    world_z: block.world_z,
                    volume_mm3: gathered_volume_mm3,
                    metadata: pack_surface_decoration_metadata(
                        decoration.rule_id,
                        decoration.decoration_id,
                    ),
                });
            }
        }
        if let Some(drop) = extra_drop {
            if batch_mine_reward_passes(
                global_config_view,
                block,
                0x2_000_u32.saturating_add(u32::from(drop.block_id)),
                BATCH_MINE_EXTRA_DROP_CHANCE_BPS,
            ) {
                secondary_rewards.push(BackpackBlockReward {
                    world_x: block.world_x,
                    packed_y: pack_backpack_resource_y(
                        block.world_y,
                        drop.block_id,
                        global_config_view.min_build_y,
                    ),
                    world_z: block.world_z,
                    volume_mm3: drop.volume_mm3,
                    metadata: 0,
                });
                extra_drop_count = extra_drop_count.saturating_add(1);
            }
        }
    }

    drop(resource_drop_data);
    drop(surface_decoration_data);
    primary_rewards.extend(secondary_rewards);
    append_backpack_block_resources_lossy(
        program_id,
        backpack_program,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        material_physics,
        chunk_x,
        chunk_z,
        bump,
        &primary_rewards,
        action_id,
    )?;

    {
        let mut data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_precision_gathering_xp(
            &mut data,
            &context.owner,
            global_config.key,
            PRECISION_GATHERING_XP_PER_BLOCK.saturating_mul(args.blocks.len() as u64),
            context.clock.slot,
        )?;
        PlayerProgressState::add_exploration_xp(
            &mut data,
            &context.owner,
            global_config.key,
            EXPLORATION_XP_PER_EXTRA_DROP.saturating_mul(extra_drop_count),
            context.clock.slot,
        )?;
        PlayerProgressState::add_explored_chunk_count(
            &mut data,
            &context.owner,
            global_config.key,
            explored_chunk_count_delta,
            context.clock.slot,
        )?;
    }

    Ok(())
}

fn range_mine_with_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 13 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }

    let args = RangeMineArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let session_authority = next_account_info(account_info_iter)?;
    let player_profile = next_account_info(account_info_iter)?;
    let player_session = next_account_info(account_info_iter)?;
    let player_progress = next_account_info(account_info_iter)?;
    let chunk_broken = next_account_info(account_info_iter)?;
    let foundation_chunk = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let resource_drop_table = next_account_info(account_info_iter)?;
    let surface_decoration_table = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let backpack = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !player_progress.is_writable || !chunk_broken.is_writable || !backpack.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    validate_backpack(backpack_program, backpack)?;

    let context = validate_player_action(
        session_authority,
        player_profile,
        player_session,
        global_config,
        system_program_account,
    )?;
    let global_config_view = &context.config;
    let action_anchor = args
        .blocks
        .first()
        .ok_or(NicechunkChunkError::InvalidRangeMine)?;
    let action_id = mining_action_id(&context, 3, action_anchor);
    let chunk_size = i32::from(global_config_view.chunk_size);
    let max_x = args
        .min_x
        .checked_add(i32::from(args.size_x).saturating_sub(1))
        .ok_or(NicechunkChunkError::InvalidRangeMine)?;
    let max_z = args
        .min_z
        .checked_add(i32::from(args.size_z).saturating_sub(1))
        .ok_or(NicechunkChunkError::InvalidRangeMine)?;
    let chunk_x = args.min_x.div_euclid(chunk_size);
    let chunk_z = args.min_z.div_euclid(chunk_size);
    if max_x.div_euclid(chunk_size) != chunk_x || max_z.div_euclid(chunk_size) != chunk_z {
        return Err(NicechunkChunkError::BatchMineCrossChunk.into());
    }
    reject_foundation_protected_blocks(
        program_id,
        foundation_chunk,
        global_config,
        chunk_x,
        chunk_z,
        &args.blocks,
    )?;

    let (gathered_volume_mm3, exploration_xp) = load_player_progress(
        program_id,
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        &context,
    )?;
    let bump = validate_chunk_broken_pda(
        program_id,
        chunk_broken.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;
    let mut existing = if chunk_broken.owner == program_id {
        let data = chunk_broken.try_borrow_data()?;
        ChunkBrokenState::packed_values(&data, global_config_view.min_build_y)?
    } else if chunk_broken.owner == &system_program::ID && chunk_broken.data_len() == 0 {
        Vec::new()
    } else {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    };
    existing.sort_unstable();
    let mut accepted = Vec::with_capacity(args.blocks.len());
    let chunk_origin_x = chunk_x.saturating_mul(chunk_size);
    let chunk_origin_z = chunk_z.saturating_mul(chunk_size);
    for block in args.blocks {
        let local_x = u8::try_from(block.world_x.saturating_sub(chunk_origin_x))
            .map_err(|_| NicechunkChunkError::InvalidBlockCoordinate)?;
        let local_z = u8::try_from(block.world_z.saturating_sub(chunk_origin_z))
            .map_err(|_| NicechunkChunkError::InvalidBlockCoordinate)?;
        if u16::from(local_x) >= global_config_view.chunk_size
            || u16::from(local_z) >= global_config_view.chunk_size
        {
            return Err(NicechunkChunkError::InvalidBlockCoordinate.into());
        }
        let packed = pack_broken_coord(
            local_x,
            block.world_y,
            local_z,
            global_config_view.min_build_y,
        )?;
        let packed_value = u32::from_le_bytes([packed[0], packed[1], packed[2], 0]);
        if existing.binary_search(&packed_value).is_ok() {
            continue;
        }
        accepted.push((block, packed));
    }
    if accepted.is_empty() {
        return Err(NicechunkChunkError::BlockAlreadyMined.into());
    }

    let required_count = existing
        .len()
        .checked_add(accepted.len())
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(NicechunkChunkError::ChunkBrokenCapacityExceeded)?;
    let created = create_or_grow_range_chunk_broken(
        session_authority,
        chunk_broken,
        global_config,
        system_program_account,
        program_id,
        global_config_view.min_build_y,
        chunk_x,
        chunk_z,
        bump,
        required_count,
    )?;
    {
        let packed = accepted
            .iter()
            .map(|(_, packed)| *packed)
            .collect::<Vec<_>>();
        let mut data = chunk_broken.try_borrow_mut_data()?;
        ChunkBrokenState::append_many_packed(&mut data, global_config_view.min_build_y, &packed)?;
    }

    validate_rule_table_pda(
        program_id,
        resource_drop_table.key,
        global_config.key,
        RESOURCE_DROP_TABLE_SEED,
        NicechunkChunkError::InvalidResourceDropTablePda,
    )?;
    validate_rule_table_pda(
        program_id,
        surface_decoration_table.key,
        global_config.key,
        SURFACE_DECORATION_TABLE_SEED,
        NicechunkChunkError::InvalidSurfaceDecorationTablePda,
    )?;
    require_key_eq(
        resource_drop_table.owner,
        program_id,
        NicechunkChunkError::InvalidResourceDropTableData,
    )?;
    require_key_eq(
        surface_decoration_table.owner,
        program_id,
        NicechunkChunkError::InvalidSurfaceDecorationTableData,
    )?;

    let resource_drop_data = resource_drop_table.try_borrow_data()?;
    let surface_decoration_data = surface_decoration_table.try_borrow_data()?;
    let mut primary_rewards = Vec::with_capacity(RANGE_MINE_MAX_REWARDS);
    let mut secondary_rewards = Vec::with_capacity(RANGE_MINE_SECONDARY_PROOF_LIMIT * 2);
    let mut secondary_proofs = 0_usize;
    let mut extra_drop_count = 0_u64;
    for (block, _) in &accepted {
        if primary_rewards.len() < RANGE_MINE_MAX_REWARDS
            && range_mine_reward_passes(
                global_config_view,
                block,
                u32::from(block.expected_block_id),
                RANGE_MINE_BASE_DROP_CHANCE_BPS,
            )
        {
            primary_rewards.push(BackpackBlockReward {
                world_x: block.world_x,
                packed_y: pack_backpack_resource_y(
                    block.world_y,
                    block.expected_block_id,
                    global_config_view.min_build_y,
                ),
                world_z: block.world_z,
                volume_mm3: gathered_volume_mm3,
                metadata: 0,
            });
        }

        if secondary_proofs >= RANGE_MINE_SECONDARY_PROOF_LIMIT
            || primary_rewards.len() + secondary_rewards.len() >= RANGE_MINE_MAX_REWARDS
            || !range_mine_reward_passes(
                global_config_view,
                block,
                0x3_000_u32.saturating_add(u32::from(block.expected_block_id)),
                RANGE_MINE_SECONDARY_CANDIDATE_CHANCE_BPS,
            )
        {
            continue;
        }
        secondary_proofs += 1;

        if let Some(drop) = extra_drop_from_table(
            global_config_view,
            &resource_drop_data,
            block.world_x,
            block.world_y,
            block.world_z,
            block.expected_block_id,
            exploration_xp,
        )? {
            if range_mine_reward_passes(
                global_config_view,
                block,
                0x2_000_u32.saturating_add(u32::from(drop.block_id)),
                BATCH_MINE_EXTRA_DROP_CHANCE_BPS,
            ) {
                secondary_rewards.push(BackpackBlockReward {
                    world_x: block.world_x,
                    packed_y: pack_backpack_resource_y(
                        block.world_y,
                        drop.block_id,
                        global_config_view.min_build_y,
                    ),
                    world_z: block.world_z,
                    volume_mm3: drop.volume_mm3,
                    metadata: 0,
                });
                extra_drop_count = extra_drop_count.saturating_add(1);
            }
        }

        if let Some(decoration) = surface_decoration_from_table(
            global_config_view,
            &surface_decoration_data,
            block.world_x,
            block.world_z,
        )?
        .filter(|entry| {
            entry.surface_y == block.world_y
                && entry.surface_block_id == block.expected_block_id
                && entry.flags & SURFACE_DECORATION_FLAG_MINEABLE != 0
        }) {
            if range_mine_reward_passes(
                global_config_view,
                block,
                0x1_000_u32.saturating_add(u32::from(decoration.rule_id)),
                BATCH_MINE_DECORATION_DROP_CHANCE_BPS,
            ) {
                secondary_rewards.push(BackpackBlockReward {
                    world_x: block.world_x,
                    packed_y: pack_backpack_resource_y(
                        decoration.surface_y.saturating_add(1),
                        decoration.drop_block_id,
                        global_config_view.min_build_y,
                    ),
                    world_z: block.world_z,
                    volume_mm3: gathered_volume_mm3,
                    metadata: pack_surface_decoration_metadata(
                        decoration.rule_id,
                        decoration.decoration_id,
                    ),
                });
            }
        }
    }

    drop(resource_drop_data);
    drop(surface_decoration_data);
    primary_rewards.extend(secondary_rewards);
    primary_rewards.truncate(RANGE_MINE_MAX_REWARDS);
    append_backpack_block_resources_lossy(
        program_id,
        backpack_program,
        chunk_broken,
        global_config,
        player_profile,
        backpack,
        material_physics,
        chunk_x,
        chunk_z,
        bump,
        &primary_rewards,
        action_id,
    )?;

    {
        let mut data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_precision_gathering_xp(
            &mut data,
            &context.owner,
            global_config.key,
            PRECISION_GATHERING_XP_PER_BLOCK.saturating_mul(accepted.len() as u64),
            context.clock.slot,
        )?;
        PlayerProgressState::add_exploration_xp(
            &mut data,
            &context.owner,
            global_config.key,
            EXPLORATION_XP_PER_EXTRA_DROP.saturating_mul(extra_drop_count),
            context.clock.slot,
        )?;
        PlayerProgressState::add_explored_chunk_count(
            &mut data,
            &context.owner,
            global_config.key,
            u32::from(created),
            context.clock.slot,
        )?;
    }

    Ok(())
}

fn fell_tree_with_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() < 10 || accounts.len() > 9 + TREE_FELL_MAX_CHUNKS {
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
    let material_physics = &accounts[7];
    let system_program_account = &accounts[8];
    let chunk_accounts = &accounts[9..];

    if !player_progress.is_writable
        || !backpack.is_writable
        || chunk_accounts.iter().any(|account| !account.is_writable)
    {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    validate_backpack(backpack_program, backpack)?;

    let context = validate_player_action(
        session_authority,
        player_profile,
        player_session,
        global_config,
        system_program_account,
    )?;
    let global_config_view = &context.config;
    let action_id = mining_action_id(&context, 4, &args);
    let generated_args = args.generated_args(global_config_view)?;
    generated_args.validate(global_config_view)?;
    let (gathered_volume_mm3, _) = load_player_progress(
        program_id,
        session_authority,
        player_progress,
        global_config,
        system_program_account,
        &context,
    )?;

    let cut_block_id = generated_block_id_at(global_config_view, &generated_args);
    if args.expected_block_id != cut_block_id {
        return Err(NicechunkChunkError::GeneratedBlockMismatch.into());
    }
    if !is_tree_trunk_block(cut_block_id) {
        return Err(NicechunkChunkError::UnmineableBlock.into());
    }

    let blocks =
        generated_tree_fell_blocks(global_config_view, args.world_x, args.world_y, args.world_z)?;
    let chunks = tree_fell_chunks(global_config_view, &blocks)?;
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

    let mut explored_chunk_count_delta = 0_u32;
    for ((chunk_x, chunk_z), account_index) in chunks.iter().zip(chunk_account_indexes.iter()) {
        let account = &chunk_accounts[*account_index];
        let bump = validate_chunk_broken_pda(
            program_id,
            account.key,
            global_config.key,
            *chunk_x,
            *chunk_z,
        )?;
        if create_or_grow_chunk_broken_if_needed(
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
        )? {
            explored_chunk_count_delta = explored_chunk_count_delta.saturating_add(1);
        }
    }

    let mut rewards = Vec::with_capacity(blocks.len());
    let mut fell_leaf_count: usize = 0;
    for block in &blocks {
        let (chunk_x, chunk_z, local_x, local_z) =
            tree_block_chunk_coords(global_config_view, block)?;
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
            let bump = validate_chunk_broken_pda(
                program_id,
                account.key,
                global_config.key,
                chunk_x,
                chunk_z,
            )?;
            rewards.push((
                *block,
                chunk_account_indexes[chunk_index],
                chunk_x,
                chunk_z,
                bump,
            ));
        } else if is_tree_leaf_block(block.block_id) {
            fell_leaf_count = fell_leaf_count.saturating_add(1);
            if fell_leaf_count % 5 == 0 {
                let bump = validate_chunk_broken_pda(
                    program_id,
                    account.key,
                    global_config.key,
                    chunk_x,
                    chunk_z,
                )?;
                rewards.push((
                    *block,
                    chunk_account_indexes[chunk_index],
                    chunk_x,
                    chunk_z,
                    bump,
                ));
            }
        }
    }

    for ((chunk_x, chunk_z), account_index) in chunks.iter().zip(chunk_account_indexes.iter()) {
        let bump = validate_chunk_broken_pda(
            program_id,
            chunk_accounts[*account_index].key,
            global_config.key,
            *chunk_x,
            *chunk_z,
        )?;
        let chunk_rewards = rewards
            .iter()
            .filter(|(_, reward_account_index, _, _, _)| reward_account_index == account_index)
            .map(|(block, _, _, _, _)| BackpackBlockReward {
                world_x: block.world_x,
                packed_y: pack_backpack_resource_y(
                    block.world_y,
                    block.block_id,
                    global_config_view.min_build_y,
                ),
                world_z: block.world_z,
                volume_mm3: gathered_volume_mm3,
                metadata: 0,
            })
            .collect::<Vec<_>>();
        append_backpack_block_resources_lossy(
            program_id,
            backpack_program,
            &chunk_accounts[*account_index],
            global_config,
            player_profile,
            backpack,
            material_physics,
            *chunk_x,
            *chunk_z,
            bump,
            &chunk_rewards,
            action_id,
        )?;
    }

    if !rewards.is_empty() {
        let mut data = player_progress.try_borrow_mut_data()?;
        PlayerProgressState::add_precision_gathering_xp(
            &mut data,
            &context.owner,
            global_config.key,
            PRECISION_GATHERING_XP_PER_BLOCK.saturating_mul(rewards.len() as u64),
            context.clock.slot,
        )?;
        PlayerProgressState::add_explored_chunk_count(
            &mut data,
            &context.owner,
            global_config.key,
            explored_chunk_count_delta,
            context.clock.slot,
        )?;
    }

    Ok(())
}

fn reject_foundation_protected_block(
    program_id: &Pubkey,
    foundation_chunk: &AccountInfo,
    global_config: &AccountInfo,
    chunk_x: i32,
    chunk_z: i32,
    args: &MineBlockArgs,
) -> ProgramResult {
    reject_foundation_protected_blocks(
        program_id,
        foundation_chunk,
        global_config,
        chunk_x,
        chunk_z,
        core::slice::from_ref(args),
    )
}

fn reject_foundation_protected_blocks(
    program_id: &Pubkey,
    foundation_chunk: &AccountInfo,
    global_config: &AccountInfo,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &[MineBlockArgs],
) -> ProgramResult {
    validate_foundation_chunk_pda(
        program_id,
        foundation_chunk.key,
        global_config.key,
        chunk_x,
        chunk_z,
    )?;
    if foundation_chunk.owner == &system_program::ID && foundation_chunk.data_len() == 0 {
        return Ok(());
    }
    require_key_eq(
        foundation_chunk.owner,
        program_id,
        NicechunkChunkError::InvalidFoundationChunkData,
    )?;
    let data = foundation_chunk.try_borrow_data()?;
    if FoundationChunkState::protects_any(&data, global_config.key, chunk_x, chunk_z, blocks)? {
        return Err(NicechunkChunkError::FoundationProtected.into());
    }
    Ok(())
}

const BUILDING_CHUNK_AUTHORITY_SEED: &[u8] = b"chunk-authority-v1";
const FOUNDATION_REGISTRATION_LEN: usize = 67;
const FOUNDATION_OPERATION_UPSERT: u8 = 0;
const FOUNDATION_OPERATION_REMOVE: u8 = 1;

#[derive(Clone, Copy)]
struct FoundationRegistrationArgs {
    record: FoundationRecordV2,
    chunk_x: i32,
    chunk_z: i32,
    operation: u8,
}

impl FoundationRegistrationArgs {
    fn unpack(payload: &[u8]) -> Result<Self, NicechunkChunkError> {
        if payload.len() != FOUNDATION_REGISTRATION_LEN {
            return Err(NicechunkChunkError::InvalidFoundationRegistration);
        }
        let record = FoundationRecordV2 {
            owner: Pubkey::new_from_array(
                payload[0..32]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
            foundation_id: u64::from_le_bytes(
                payload[32..40]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
            min_x: i32::from_le_bytes(
                payload[40..44]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
            min_z: i32::from_le_bytes(
                payload[44..48]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
            surface_y: i16::from_le_bytes(
                payload[48..50]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
            width: u32::from_le_bytes(
                payload[50..54]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
            depth: u32::from_le_bytes(
                payload[54..58]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
            ),
        };
        let chunk_x = i32::from_le_bytes(
            payload[58..62]
                .try_into()
                .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
        );
        let chunk_z = i32::from_le_bytes(
            payload[62..66]
                .try_into()
                .map_err(|_| NicechunkChunkError::InvalidFoundationRegistration)?,
        );
        let operation = payload[66];
        if record.owner == Pubkey::default()
            || record.foundation_id == 0
            || record.width < 2
            || record.depth < 2
            || record.max_x().is_none()
            || record.max_z().is_none()
            || operation != FOUNDATION_OPERATION_UPSERT && operation != FOUNDATION_OPERATION_REMOVE
        {
            return Err(NicechunkChunkError::InvalidFoundationRegistration);
        }
        Ok(Self {
            record,
            chunk_x,
            chunk_z,
            operation,
        })
    }

    fn validate(&self, config: &GlobalConfigView) -> Result<(), NicechunkChunkError> {
        if self.record.surface_y <= config.min_build_y || self.record.surface_y > config.max_build_y
        {
            return Err(NicechunkChunkError::InvalidFoundationRegistration);
        }
        let chunk_size = i32::from(config.chunk_size);
        let max_x = self
            .record
            .max_x()
            .ok_or(NicechunkChunkError::InvalidFoundationRegistration)?;
        let max_z = self
            .record
            .max_z()
            .ok_or(NicechunkChunkError::InvalidFoundationRegistration)?;
        if self.chunk_x < self.record.min_x.div_euclid(chunk_size)
            || self.chunk_x > max_x.div_euclid(chunk_size)
            || self.chunk_z < self.record.min_z.div_euclid(chunk_size)
            || self.chunk_z > max_z.div_euclid(chunk_size)
        {
            return Err(NicechunkChunkError::InvalidFoundationRegistration);
        }
        Ok(())
    }
}

fn register_build_site_chunk(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 5 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }
    let args = FoundationRegistrationArgs::unpack(payload)?;
    let building_authority = &accounts[0];
    let payer = &accounts[1];
    let foundation_chunk = &accounts[2];
    let global_config = &accounts[3];
    let system_program_account = &accounts[4];
    if !building_authority.is_signer {
        return Err(NicechunkChunkError::InvalidBuildingAuthority.into());
    }
    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkChunkError::InvalidPayer.into());
    }
    if !foundation_chunk.is_writable {
        return Err(NicechunkChunkError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;
    let (expected_authority, _) = Pubkey::find_program_address(
        &[BUILDING_CHUNK_AUTHORITY_SEED, global_config.key.as_ref()],
        &NICECHUNK_BUILDING_PROGRAM_ID,
    );
    require_key_eq(
        building_authority.key,
        &expected_authority,
        NicechunkChunkError::InvalidBuildingAuthority,
    )?;
    let config = validate_global_config(global_config)?;
    args.validate(&config)?;
    let bump = validate_foundation_chunk_pda(
        program_id,
        foundation_chunk.key,
        global_config.key,
        args.chunk_x,
        args.chunk_z,
    )?;
    if args.operation == FOUNDATION_OPERATION_REMOVE
        && foundation_chunk.owner == &system_program::ID
        && foundation_chunk.data_len() == 0
    {
        return Ok(());
    }
    ensure_foundation_chunk_v2(
        payer,
        foundation_chunk,
        global_config,
        system_program_account,
        program_id,
        args.chunk_x,
        args.chunk_z,
        bump,
        if args.operation == FOUNDATION_OPERATION_UPSERT {
            Some((&args.record, i32::from(config.chunk_size)))
        } else {
            None
        },
    )?;
    if args.operation == FOUNDATION_OPERATION_REMOVE {
        let mut data = foundation_chunk.try_borrow_mut_data()?;
        return FoundationChunkV2State::remove(
            &mut data,
            global_config.key,
            args.chunk_x,
            args.chunk_z,
            &args.record.owner,
            args.record.foundation_id,
        );
    }
    grow_foundation_chunk_v2_if_full(
        payer,
        foundation_chunk,
        global_config,
        system_program_account,
        args.chunk_x,
        args.chunk_z,
        &args.record.owner,
        args.record.foundation_id,
    )?;
    let mut data = foundation_chunk.try_borrow_mut_data()?;
    FoundationChunkV2State::append(
        &mut data,
        global_config.key,
        args.chunk_x,
        args.chunk_z,
        &args.record,
    )
}

#[allow(clippy::too_many_arguments)]
fn ensure_foundation_chunk_v2<'a>(
    payer: &AccountInfo<'a>,
    foundation_chunk: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    chunk_x: i32,
    chunk_z: i32,
    bump: u8,
    replacement: Option<(&FoundationRecordV2, i32)>,
) -> ProgramResult {
    if foundation_chunk.owner == &system_program::ID && foundation_chunk.data_len() == 0 {
        let capacity = FOUNDATION_CHUNK_V2_INITIAL_CAPACITY;
        let len = FoundationChunkV2State::len(capacity)?;
        let chunk_x_bytes = chunk_x.to_le_bytes();
        let chunk_z_bytes = chunk_z.to_le_bytes();
        let bump_seed = [bump];
        let seeds = [
            FOUNDATION_CHUNK_SEED,
            global_config.key.as_ref(),
            chunk_x_bytes.as_ref(),
            chunk_z_bytes.as_ref(),
            bump_seed.as_ref(),
        ];
        create_fixed_pda_account(
            payer,
            foundation_chunk,
            system_program_account,
            program_id,
            len,
            &seeds,
        )?;
        let mut data = foundation_chunk.try_borrow_mut_data()?;
        return FoundationChunkV2State::pack_empty(
            &mut data,
            bump,
            global_config.key,
            chunk_x,
            chunk_z,
            capacity,
        );
    }
    require_key_eq(
        foundation_chunk.owner,
        program_id,
        NicechunkChunkError::InvalidFoundationChunkData,
    )?;
    let magic = {
        let data = foundation_chunk.try_borrow_data()?;
        if data.len() < 8 {
            return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
        }
        data[0..8].try_into().unwrap_or([0; 8])
    };
    if magic == FOUNDATION_CHUNK_V2_MAGIC {
        return Ok(());
    }
    if magic != FOUNDATION_CHUNK_MAGIC {
        return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
    }
    let legacy = {
        let data = foundation_chunk.try_borrow_data()?;
        FoundationChunkState::records(&data, global_config.key, chunk_x, chunk_z)?
    };
    let migrated: Vec<_> = legacy
        .into_iter()
        .map(|record| FoundationRecordV2::from_legacy(&record))
        .filter(|record| {
            !replacement
                .map(|(candidate, chunk_size)| {
                    candidate.supersedes_legacy_index(record, chunk_size)
                })
                .unwrap_or(false)
        })
        .collect();
    let required = u16::try_from(migrated.len())
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or(NicechunkChunkError::FoundationChunkCapacityExceeded)?;
    let capacity = rounded_foundation_capacity(required)?;
    let len = FoundationChunkV2State::len(capacity)?;
    fund_account_to_rent_exempt(payer, foundation_chunk, system_program_account, len)?;
    foundation_chunk.realloc(len, true)?;
    let mut data = foundation_chunk.try_borrow_mut_data()?;
    FoundationChunkV2State::pack_empty(
        &mut data,
        bump,
        global_config.key,
        chunk_x,
        chunk_z,
        capacity,
    )?;
    for record in migrated {
        FoundationChunkV2State::append(&mut data, global_config.key, chunk_x, chunk_z, &record)?;
    }
    Ok(())
}

fn grow_foundation_chunk_v2_if_full<'a>(
    payer: &AccountInfo<'a>,
    foundation_chunk: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    chunk_x: i32,
    chunk_z: i32,
    owner: &Pubkey,
    foundation_id: u64,
) -> ProgramResult {
    let (count, capacity, exists) = {
        let data = foundation_chunk.try_borrow_data()?;
        let (count, capacity) =
            FoundationChunkV2State::validate(&data, global_config.key, chunk_x, chunk_z)?;
        let exists = FoundationChunkV2State::contains_foundation(
            &data,
            global_config.key,
            chunk_x,
            chunk_z,
            owner,
            foundation_id,
        )?;
        (count, capacity, exists)
    };
    if exists || count < capacity {
        return Ok(());
    }
    let next_capacity = capacity
        .checked_add(FOUNDATION_CHUNK_V2_GROWTH)
        .map(|value| value.min(FOUNDATION_CHUNK_V2_MAX_CAPACITY))
        .ok_or(NicechunkChunkError::FoundationChunkCapacityExceeded)?;
    if next_capacity <= capacity {
        return Err(NicechunkChunkError::FoundationChunkCapacityExceeded.into());
    }
    let len = FoundationChunkV2State::len(next_capacity)?;
    fund_account_to_rent_exempt(payer, foundation_chunk, system_program_account, len)?;
    foundation_chunk.realloc(len, true)?;
    let mut data = foundation_chunk.try_borrow_mut_data()?;
    data[12..14].copy_from_slice(&next_capacity.to_le_bytes());
    FoundationChunkV2State::validate(&data, global_config.key, chunk_x, chunk_z)?;
    Ok(())
}

fn rounded_foundation_capacity(required: u16) -> Result<u16, NicechunkChunkError> {
    let capacity = required
        .max(FOUNDATION_CHUNK_V2_INITIAL_CAPACITY)
        .saturating_add(FOUNDATION_CHUNK_V2_GROWTH - 1)
        / FOUNDATION_CHUNK_V2_GROWTH
        * FOUNDATION_CHUNK_V2_GROWTH;
    if capacity > FOUNDATION_CHUNK_V2_MAX_CAPACITY {
        return Err(NicechunkChunkError::FoundationChunkCapacityExceeded);
    }
    Ok(capacity)
}

fn validate_foundation_chunk_pda(
    program_id: &Pubkey,
    foundation_chunk: &Pubkey,
    global_config: &Pubkey,
    chunk_x: i32,
    chunk_z: i32,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let (expected, bump) = Pubkey::find_program_address(
        &[
            FOUNDATION_CHUNK_SEED,
            global_config.as_ref(),
            &chunk_x_bytes,
            &chunk_z_bytes,
        ],
        program_id,
    );
    require_key_eq(
        foundation_chunk,
        &expected,
        NicechunkChunkError::InvalidFoundationChunkPda,
    )?;
    Ok(bump)
}

struct RuleTableInitAccounts<'a, 'info> {
    payer: &'a AccountInfo<'info>,
    table: &'a AccountInfo<'info>,
    global_config: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> RuleTableInitAccounts<'a, 'info> {
    fn parse(
        accounts: &'a [AccountInfo<'info>],
    ) -> Result<Self, solana_program::program_error::ProgramError> {
        let account_info_iter = &mut accounts.iter();
        Ok(Self {
            payer: next_account_info(account_info_iter)?,
            table: next_account_info(account_info_iter)?,
            global_config: next_account_info(account_info_iter)?,
            system_program: next_account_info(account_info_iter)?,
        })
    }

    fn create(
        &self,
        program_id: &Pubkey,
        seed: &[u8],
        len: usize,
        pda_error: NicechunkChunkError,
        data_error: NicechunkChunkError,
    ) -> Result<u8, solana_program::program_error::ProgramError> {
        if !self.payer.is_signer || !self.payer.is_writable {
            return Err(NicechunkChunkError::InvalidPayer.into());
        }
        if !self.table.is_writable {
            return Err(NicechunkChunkError::InvalidWritableAccount.into());
        }
        require_key_eq(
            self.system_program.key,
            &system_program::ID,
            NicechunkChunkError::InvalidSystemProgram,
        )?;
        let config = validate_global_config(self.global_config)?;
        require_key_eq(
            self.payer.key,
            &config.development_wallet,
            NicechunkChunkError::InvalidRuleTableAuthority,
        )?;
        let bump = validate_rule_table_pda(
            program_id,
            self.table.key,
            self.global_config.key,
            seed,
            pda_error,
        )?;
        if self.table.owner == program_id {
            return Err(data_error.into());
        }
        if self.table.owner != &system_program::ID || self.table.data_len() != 0 {
            return Err(NicechunkChunkError::InvalidSystemAccount.into());
        }

        let lamports = Rent::get()?.minimum_balance(len);
        let create = system_create_account(
            self.payer.key,
            self.table.key,
            lamports,
            len as u64,
            program_id,
        );
        let bump_seed = [bump];
        let seeds = [seed, self.global_config.key.as_ref(), &bump_seed];
        invoke_signed(
            &create,
            &[
                self.payer.clone(),
                self.table.clone(),
                self.system_program.clone(),
            ],
            &[&seeds],
        )?;
        Ok(bump)
    }
}

fn initialize_resource_drop_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }
    let rule_count = ResourceDropTableState::validate_payload(payload)?;
    let len = ResourceDropTableState::len_for_rules(rule_count);
    let init = RuleTableInitAccounts::parse(accounts)?;
    let bump = init.create(
        program_id,
        RESOURCE_DROP_TABLE_SEED,
        len,
        NicechunkChunkError::InvalidResourceDropTablePda,
        NicechunkChunkError::InvalidResourceDropTableData,
    )?;
    let mut data = init.table.try_borrow_mut_data()?;
    ResourceDropTableState::pack_payload(&mut data, bump, payload)
}

fn initialize_surface_decoration_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }
    SurfaceDecorationTableState::validate_payload(payload)?;
    let init = RuleTableInitAccounts::parse(accounts)?;
    let bump = init.create(
        program_id,
        SURFACE_DECORATION_TABLE_SEED,
        SURFACE_DECORATION_TABLE_LEN,
        NicechunkChunkError::InvalidSurfaceDecorationTablePda,
        NicechunkChunkError::InvalidSurfaceDecorationTableData,
    )?;
    let mut data = init.table.try_borrow_mut_data()?;
    SurfaceDecorationTableState::pack_payload(&mut data, bump, 1, payload)
}

fn verify_surface_decoration(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 2 || payload.len() != 14 {
        return Err(NicechunkChunkError::InvalidInstruction.into());
    }
    let world_x = i32::from_le_bytes(
        payload[0..4]
            .try_into()
            .map_err(|_| NicechunkChunkError::InvalidInstruction)?,
    );
    let world_z = i32::from_le_bytes(
        payload[4..8]
            .try_into()
            .map_err(|_| NicechunkChunkError::InvalidInstruction)?,
    );
    let expected_surface_block_id = u16::from_le_bytes([payload[8], payload[9]]);
    let expected_decoration_id = u16::from_le_bytes([payload[10], payload[11]]);
    let expected_rule_id = u16::from_le_bytes([payload[12], payload[13]]);
    let account_info_iter = &mut accounts.iter();
    let table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    require_key_eq(
        table.owner,
        program_id,
        NicechunkChunkError::InvalidSurfaceDecorationTableData,
    )?;
    let config = validate_global_config(global_config)?;
    validate_rule_table_pda(
        program_id,
        table.key,
        global_config.key,
        SURFACE_DECORATION_TABLE_SEED,
        NicechunkChunkError::InvalidSurfaceDecorationTablePda,
    )?;
    let data = table.try_borrow_data()?;
    let found = surface_decoration_from_table(&config, &data, world_x, world_z)?;
    let matches = found
        .map(|entry| {
            entry.surface_block_id == expected_surface_block_id
                && entry.decoration_id == expected_decoration_id
                && entry.rule_id == expected_rule_id
        })
        .unwrap_or(expected_decoration_id == 0 && expected_rule_id == 0);
    if !matches {
        return Err(NicechunkChunkError::SurfaceDecorationMismatch.into());
    }
    Ok(())
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
    )?;
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

fn validate_rule_table_pda(
    program_id: &Pubkey,
    table: &Pubkey,
    global_config: &Pubkey,
    seed: &[u8],
    error: NicechunkChunkError,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let (expected_table, bump) =
        Pubkey::find_program_address(&[seed, global_config.as_ref()], program_id);
    require_key_eq(table, &expected_table, error)?;
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

struct CivilizationTableUpdateAccounts<'a, 'info> {
    executor: &'a AccountInfo<'info>,
    table: &'a AccountInfo<'info>,
    global_config: &'a AccountInfo<'info>,
    rule_book: &'a AccountInfo<'info>,
    tally: &'a AccountInfo<'info>,
    receipt: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    civilization_program: &'a AccountInfo<'info>,
    adapter_authority: &'a AccountInfo<'info>,
}

impl<'a, 'info> CivilizationTableUpdateAccounts<'a, 'info> {
    fn parse(
        accounts: &'a [AccountInfo<'info>],
    ) -> Result<Self, solana_program::program_error::ProgramError> {
        let account_info_iter = &mut accounts.iter();
        Ok(Self {
            executor: next_account_info(account_info_iter)?,
            table: next_account_info(account_info_iter)?,
            global_config: next_account_info(account_info_iter)?,
            rule_book: next_account_info(account_info_iter)?,
            tally: next_account_info(account_info_iter)?,
            receipt: next_account_info(account_info_iter)?,
            system_program: next_account_info(account_info_iter)?,
            civilization_program: next_account_info(account_info_iter)?,
            adapter_authority: next_account_info(account_info_iter)?,
        })
    }

    fn execute(&self, program_id: &Pubkey, payload: &[u8]) -> ProgramResult {
        if !self.executor.is_signer || !self.executor.is_writable {
            return Err(NicechunkChunkError::InvalidPayer.into());
        }
        if !self.table.is_writable || !self.rule_book.is_writable || !self.receipt.is_writable {
            return Err(NicechunkChunkError::InvalidWritableAccount.into());
        }
        require_key_eq(
            self.system_program.key,
            &system_program::ID,
            NicechunkChunkError::InvalidSystemProgram,
        )?;
        require_key_eq(
            self.civilization_program.key,
            &NICECHUNK_CIVILIZATION_PROGRAM_ID,
            NicechunkChunkError::InvalidCivilizationProgram,
        )?;
        require_key_eq(
            self.rule_book.owner,
            self.civilization_program.key,
            NicechunkChunkError::InvalidCivilizationRule,
        )?;
        require_key_eq(
            self.tally.owner,
            self.civilization_program.key,
            NicechunkChunkError::InvalidCivilizationTally,
        )?;
        {
            let rule_data = self.rule_book.try_borrow_data()?;
            civilization_adapter::validate_rule_book_for_chunk_patch(
                &rule_data,
                self.civilization_program.key,
                program_id,
                self.table.key,
                payload,
                civilization_adapter::CIVILIZATION_STATUS_FINALIZED,
            )?;
        }
        {
            let tally_data = self.tally.try_borrow_data()?;
            civilization_adapter::validate_tally_threshold(&tally_data, self.rule_book.key)?;
        }
        civilization_adapter::invoke_civilization_execute_receipt(
            self.executor,
            self.rule_book,
            self.tally,
            self.receipt,
            self.system_program,
            self.civilization_program,
            self.adapter_authority,
            program_id,
        )?;
        require_key_eq(
            self.receipt.owner,
            self.civilization_program.key,
            NicechunkChunkError::InvalidCivilizationReceipt,
        )?;
        {
            let rule_data = self.rule_book.try_borrow_data()?;
            civilization_adapter::validate_rule_book_for_chunk_patch(
                &rule_data,
                self.civilization_program.key,
                program_id,
                self.table.key,
                payload,
                civilization_adapter::CIVILIZATION_STATUS_EXECUTED,
            )?;
        }
        let receipt_data = self.receipt.try_borrow_data()?;
        civilization_adapter::validate_execution_receipt(&receipt_data, self.rule_book.key)
    }
}

fn apply_civilization_resource_drop_receipt(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 9 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }
    let rule_count = ResourceDropTableState::validate_payload(payload)?;
    let update = CivilizationTableUpdateAccounts::parse(accounts)?;
    require_key_eq(
        update.table.owner,
        program_id,
        NicechunkChunkError::InvalidResourceDropTableData,
    )?;
    validate_global_config(update.global_config)?;
    let bump = validate_rule_table_pda(
        program_id,
        update.table.key,
        update.global_config.key,
        RESOURCE_DROP_TABLE_SEED,
        NicechunkChunkError::InvalidResourceDropTablePda,
    )?;
    {
        let table_data = update.table.try_borrow_data()?;
        let existing_count = ResourceDropTableState::validate_header(&table_data)?;
        if existing_count != rule_count {
            return Err(NicechunkChunkError::InvalidResourceDropTableData.into());
        }
    }
    update.execute(program_id, payload)?;
    let mut data = update.table.try_borrow_mut_data()?;
    ResourceDropTableState::pack_payload(&mut data, bump, payload)
}

fn apply_civilization_surface_decoration_receipt(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 9 {
        return Err(NicechunkChunkError::InvalidAccountCount.into());
    }
    SurfaceDecorationTableState::validate_payload(payload)?;
    let update = CivilizationTableUpdateAccounts::parse(accounts)?;
    require_key_eq(
        update.table.owner,
        program_id,
        NicechunkChunkError::InvalidSurfaceDecorationTableData,
    )?;
    validate_global_config(update.global_config)?;
    let bump = validate_rule_table_pda(
        program_id,
        update.table.key,
        update.global_config.key,
        SURFACE_DECORATION_TABLE_SEED,
        NicechunkChunkError::InvalidSurfaceDecorationTablePda,
    )?;
    let revision = {
        let data = update.table.try_borrow_data()?;
        SurfaceDecorationTableState::validate_header(&data)?.1
    };
    update.execute(program_id, payload)?;
    let mut data = update.table.try_borrow_mut_data()?;
    SurfaceDecorationTableState::pack_payload(&mut data, bump, revision.saturating_add(1), payload)
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
        let create = system_create_account(
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
            let transfer = system_transfer(payer.key, player_progress.key, delta);
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    player_progress.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        let allocate = system_allocate(player_progress.key, PLAYER_PROGRESS_LEN as u64);
        invoke_signed(
            &allocate,
            &[player_progress.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_assign(player_progress.key, program_id);
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
    program_id: &Pubkey,
    backpack_program: &AccountInfo<'a>,
    chunk_broken: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    chunk_x: i32,
    chunk_z: i32,
    chunk_bump: u8,
    world_x: i32,
    packed_y: i16,
    world_z: i32,
    volume_mm3: u32,
    action_id: u64,
) -> ProgramResult {
    let mut data = [0_u8; 23];
    data[0] = 1;
    data[1..5].copy_from_slice(&world_x.to_le_bytes());
    data[5..7].copy_from_slice(&packed_y.to_le_bytes());
    data[7..11].copy_from_slice(&world_z.to_le_bytes());
    data[11..15].copy_from_slice(&volume_mm3.to_le_bytes());
    data[15..23].copy_from_slice(&action_id.to_le_bytes());
    let data = backpack_cpi_data(&data);
    let ix = Instruction {
        program_id: *backpack_program.key,
        accounts: vec![
            AccountMeta::new_readonly(*chunk_broken.key, true),
            AccountMeta::new_readonly(*global_config.key, false),
            AccountMeta::new_readonly(*player_profile.key, false),
            AccountMeta::new(*backpack.key, false),
            AccountMeta::new_readonly(*material_physics.key, false),
        ],
        data,
    };
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let seeds = &[
        CHUNK_BROKEN_SEED,
        global_config.key.as_ref(),
        &chunk_x_bytes,
        &chunk_z_bytes,
        &[chunk_bump],
    ];
    let expected = Pubkey::create_program_address(seeds, program_id)
        .map_err(|_| NicechunkChunkError::InvalidChunkBrokenPda)?;
    require_key_eq(
        chunk_broken.key,
        &expected,
        NicechunkChunkError::InvalidChunkBrokenPda,
    )?;
    invoke_signed(
        &ix,
        &[
            chunk_broken.clone(),
            global_config.clone(),
            player_profile.clone(),
            backpack.clone(),
            material_physics.clone(),
        ],
        &[seeds],
    )
}

#[derive(Clone, Copy)]
struct BackpackBlockReward {
    world_x: i32,
    packed_y: i16,
    world_z: i32,
    volume_mm3: u32,
    metadata: u32,
}

fn pack_surface_decoration_metadata(rule_id: u16, decoration_id: u16) -> u32 {
    (u32::from(rule_id) << 16) | u32::from(decoration_id)
}

#[allow(clippy::too_many_arguments)]
fn append_backpack_block_resources_lossy<'a>(
    program_id: &Pubkey,
    backpack_program: &AccountInfo<'a>,
    chunk_broken: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    chunk_x: i32,
    chunk_z: i32,
    chunk_bump: u8,
    rewards: &[BackpackBlockReward],
    action_id: u64,
) -> ProgramResult {
    if rewards.is_empty() {
        return Ok(());
    }
    let mut data = Vec::with_capacity(10 + rewards.len() * 18);
    data.push(6);
    data.push(rewards.len().min(u8::MAX as usize) as u8);
    data.extend_from_slice(&action_id.to_le_bytes());
    for reward in rewards {
        data.extend_from_slice(&reward.world_x.to_le_bytes());
        data.extend_from_slice(&reward.packed_y.to_le_bytes());
        data.extend_from_slice(&reward.world_z.to_le_bytes());
        data.extend_from_slice(&reward.volume_mm3.to_le_bytes());
        data.extend_from_slice(&reward.metadata.to_le_bytes());
    }
    let data = backpack_cpi_data(&data);
    let ix = Instruction {
        program_id: *backpack_program.key,
        accounts: vec![
            AccountMeta::new_readonly(*chunk_broken.key, true),
            AccountMeta::new_readonly(*global_config.key, false),
            AccountMeta::new_readonly(*player_profile.key, false),
            AccountMeta::new(*backpack.key, false),
            AccountMeta::new_readonly(*material_physics.key, false),
        ],
        data,
    };
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let seeds = &[
        CHUNK_BROKEN_SEED,
        global_config.key.as_ref(),
        &chunk_x_bytes,
        &chunk_z_bytes,
        &[chunk_bump],
    ];
    let expected = Pubkey::create_program_address(seeds, program_id)
        .map_err(|_| NicechunkChunkError::InvalidChunkBrokenPda)?;
    require_key_eq(
        chunk_broken.key,
        &expected,
        NicechunkChunkError::InvalidChunkBrokenPda,
    )?;
    invoke_signed(
        &ix,
        &[
            chunk_broken.clone(),
            global_config.clone(),
            player_profile.clone(),
            backpack.clone(),
            material_physics.clone(),
        ],
        &[seeds],
    )
}

fn backpack_cpi_data(data: &[u8]) -> Vec<u8> {
    let mut wrapped = Vec::with_capacity(data.len() + 1);
    wrapped.push(1);
    wrapped.extend_from_slice(data);
    wrapped
}

fn system_create_account(
    from: &Pubkey,
    to: &Pubkey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Instruction {
    let mut data = [0_u8; 52];
    data[4..12].copy_from_slice(&lamports.to_le_bytes());
    data[12..20].copy_from_slice(&space.to_le_bytes());
    data[20..52].copy_from_slice(owner.as_ref());
    Instruction {
        program_id: system_program::ID,
        accounts: vec![AccountMeta::new(*from, true), AccountMeta::new(*to, true)],
        data: data.to_vec(),
    }
}

fn system_assign(account: &Pubkey, owner: &Pubkey) -> Instruction {
    let mut data = [0_u8; 36];
    data[0] = 1;
    data[4..36].copy_from_slice(owner.as_ref());
    Instruction {
        program_id: system_program::ID,
        accounts: vec![AccountMeta::new(*account, true)],
        data: data.to_vec(),
    }
}

fn system_transfer(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    let mut data = [0_u8; 12];
    data[0] = 2;
    data[4..12].copy_from_slice(&lamports.to_le_bytes());
    Instruction {
        program_id: system_program::ID,
        accounts: vec![AccountMeta::new(*from, true), AccountMeta::new(*to, false)],
        data: data.to_vec(),
    }
}

fn system_allocate(account: &Pubkey, space: u64) -> Instruction {
    let mut data = [0_u8; 12];
    data[0] = 8;
    data[4..12].copy_from_slice(&space.to_le_bytes());
    Instruction {
        program_id: system_program::ID,
        accounts: vec![AccountMeta::new(*account, true)],
        data: data.to_vec(),
    }
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
) -> Result<bool, solana_program::program_error::ProgramError> {
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
            return Ok(false);
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
        return Ok(false);
    }

    if chunk_broken.owner != &system_program::ID || chunk_broken.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }

    let initial_len = ChunkBrokenState::len_for_capacity(CHUNK_BROKEN_INITIAL_CAPACITY);
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(initial_len);

    if chunk_broken.lamports() == 0 {
        let create = system_create_account(
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
            let transfer = system_transfer(payer.key, chunk_broken.key, delta);
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    chunk_broken.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }

        let allocate = system_allocate(chunk_broken.key, initial_len as u64);
        invoke_signed(
            &allocate,
            &[chunk_broken.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_assign(chunk_broken.key, program_id);
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
    )?;
    Ok(true)
}

fn grow_chunk_broken_to_fit<'a>(
    payer: &AccountInfo<'a>,
    chunk_broken: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    min_build_y: i16,
    required_count: u16,
) -> ProgramResult {
    if required_count > CHUNK_BROKEN_MAX_CAPACITY {
        return Err(NicechunkChunkError::ChunkBrokenCapacityExceeded.into());
    }
    let current_capacity = {
        let data = chunk_broken.try_borrow_data()?;
        let (_, capacity) = ChunkBrokenState::validate_header(&data, min_build_y)?;
        capacity
    };
    if required_count <= current_capacity {
        return Ok(());
    }
    let growth = u32::from(CHUNK_BROKEN_GROW_BY);
    let target_capacity = ((u32::from(required_count) + growth - 1) / growth * growth)
        .min(u32::from(CHUNK_BROKEN_MAX_CAPACITY)) as u16;
    let next_len = ChunkBrokenState::len_for_capacity(target_capacity);
    fund_account_to_rent_exempt(payer, chunk_broken, system_program_account, next_len)?;
    chunk_broken.realloc(next_len, false)?;
    let mut data = chunk_broken.try_borrow_mut_data()?;
    data[8..10].copy_from_slice(&target_capacity.to_le_bytes());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_or_grow_range_chunk_broken<'a>(
    payer: &AccountInfo<'a>,
    chunk_broken: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    min_build_y: i16,
    chunk_x: i32,
    chunk_z: i32,
    bump: u8,
    required_count: u16,
) -> Result<bool, solana_program::program_error::ProgramError> {
    if chunk_broken.owner == program_id {
        grow_chunk_broken_to_fit(
            payer,
            chunk_broken,
            system_program_account,
            min_build_y,
            required_count,
        )?;
        return Ok(false);
    }
    if chunk_broken.owner != &system_program::ID || chunk_broken.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }
    if required_count == 0 || required_count > CHUNK_BROKEN_MAX_CAPACITY {
        return Err(NicechunkChunkError::ChunkBrokenCapacityExceeded.into());
    }

    let growth = u32::from(CHUNK_BROKEN_GROW_BY);
    let capacity = ((u32::from(required_count.max(CHUNK_BROKEN_INITIAL_CAPACITY)) + growth - 1)
        / growth
        * growth)
        .min(u32::from(CHUNK_BROKEN_MAX_CAPACITY)) as u16;
    let len = ChunkBrokenState::len_for_capacity(capacity);
    let lamports = Rent::get()?.minimum_balance(len);
    let chunk_x_bytes = chunk_x.to_le_bytes();
    let chunk_z_bytes = chunk_z.to_le_bytes();
    let seeds = &[
        CHUNK_BROKEN_SEED,
        global_config.key.as_ref(),
        &chunk_x_bytes,
        &chunk_z_bytes,
        &[bump],
    ];

    if chunk_broken.lamports() == 0 {
        let create = system_create_account(
            payer.key,
            chunk_broken.key,
            lamports,
            len as u64,
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
            let transfer = system_transfer(
                payer.key,
                chunk_broken.key,
                lamports - chunk_broken.lamports(),
            );
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    chunk_broken.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        let allocate = system_allocate(chunk_broken.key, len as u64);
        invoke_signed(
            &allocate,
            &[chunk_broken.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_assign(chunk_broken.key, program_id);
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
            capacity,
        },
    )?;
    Ok(true)
}

fn create_fixed_pda_account<'a>(
    payer: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    len: usize,
    seeds: &[&[u8]],
) -> Result<bool, solana_program::program_error::ProgramError> {
    if target.owner == program_id {
        if target.data_len() != len {
            return Err(NicechunkChunkError::InvalidSystemAccount.into());
        }
        return Ok(false);
    }
    if target.owner != &system_program::ID || target.data_len() != 0 {
        return Err(NicechunkChunkError::InvalidSystemAccount.into());
    }
    let lamports = Rent::get()?.minimum_balance(len);
    if target.lamports() == 0 {
        let create = system_create_account(payer.key, target.key, lamports, len as u64, program_id);
        invoke_signed(
            &create,
            &[
                payer.clone(),
                target.clone(),
                system_program_account.clone(),
            ],
            &[seeds],
        )?;
    } else {
        if target.lamports() < lamports {
            let transfer = system_transfer(payer.key, target.key, lamports - target.lamports());
            invoke(
                &transfer,
                &[
                    payer.clone(),
                    target.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        let allocate = system_allocate(target.key, len as u64);
        invoke_signed(
            &allocate,
            &[target.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        let assign = system_assign(target.key, program_id);
        invoke_signed(
            &assign,
            &[target.clone(), system_program_account.clone()],
            &[seeds],
        )?;
    }
    Ok(true)
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
    let transfer = system_transfer(payer.key, target.key, delta);
    invoke(
        &transfer,
        &[
            payer.clone(),
            target.clone(),
            system_program_account.clone(),
        ],
    )
}

#[cfg(test)]
mod instruction_encoding_tests {
    use super::*;
    use solana_program::system_instruction;

    #[test]
    fn compact_system_instruction_encoding_matches_solana_abi() {
        let from = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let expected = [
            system_instruction::create_account(&from, &to, 123, 456, &owner),
            system_instruction::assign(&to, &owner),
            system_instruction::transfer(&from, &to, 789),
            system_instruction::allocate(&to, 456),
        ];
        let compact = [
            system_create_account(&from, &to, 123, 456, &owner),
            system_assign(&to, &owner),
            system_transfer(&from, &to, 789),
            system_allocate(&to, 456),
        ];
        for (expected, compact) in expected.iter().zip(compact.iter()) {
            assert_eq!(compact.program_id, expected.program_id);
            assert_eq!(compact.accounts, expected.accounts);
            assert_eq!(compact.data, expected.data);
        }
    }

    #[test]
    fn surface_decoration_metadata_keeps_rule_and_visual_identity() {
        let metadata = pack_surface_decoration_metadata(0x1234, 0xabcd);
        assert_eq!(metadata >> 16, 0x1234);
        assert_eq!(metadata & 0xffff, 0xabcd);
    }

    #[test]
    fn mining_action_id_is_stable_for_one_action_and_changes_with_identity() {
        let context = PlayerActionContext {
            config: GlobalConfigView {
                development_wallet: Pubkey::new_unique(),
                world_seed: state::CANONICAL_WORLD_SEED,
                chunk_size: state::CANONICAL_CHUNK_SIZE,
                min_build_y: state::CANONICAL_MIN_BUILD_Y,
                max_build_y: state::CANONICAL_MAX_BUILD_Y,
                max_terrain_height: state::CANONICAL_MAX_TERRAIN_HEIGHT,
                sea_level: state::CANONICAL_SEA_LEVEL,
            },
            owner: Pubkey::new_unique(),
            clock: Clock {
                slot: 123,
                epoch_start_timestamp: 0,
                epoch: 0,
                leader_schedule_epoch: 0,
                unix_timestamp: 0,
            },
        };
        let anchor = MineBlockArgs {
            world_x: 10,
            world_y: 20,
            world_z: 30,
            expected_block_id: 1,
        };
        let first = mining_action_id(&context, 1, &anchor);
        assert_ne!(first, 0);
        assert_eq!(first, mining_action_id(&context, 1, &anchor));
        assert_ne!(first, mining_action_id(&context, 2, &anchor));
        let moved = MineBlockArgs {
            world_x: 11,
            ..anchor
        };
        assert_ne!(first, mining_action_id(&context, 1, &moved));
    }
}
