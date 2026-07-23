#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::{
        instructions::{load_current_index_checked, load_instruction_at_checked},
        Sysvar,
    },
};

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

pub mod cluster_config;
pub mod errors;
pub mod state;

use cluster_config::{
    NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_BOOTSTRAP_AUTHORITY, NICECHUNK_CHUNK_PROGRAM_ID,
    NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkSkillsError};
use state::{
    BurdenMiningRule, MiningCoordinate, MiningTravelRule, PlayerSkillsState, RuleTableState,
    SourceRule, LEVEL_COUNT, PLAYER_SKILLS_LEN, PLAYER_SKILLS_SEED, RULE_RECORD_LEN,
    RULE_TABLE_LEN, RULE_TABLE_SEED, SOURCE_SEED_GLOBAL_OWNER, SOURCE_SEED_OWNER,
};

declare_id!("5gkdfmRJogdSdPrT8rvnEkPdn2N2fLBnQ6YDdegUcu3P");

const GLOBAL_CONFIG_LEN: usize = 293;
const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
const GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET: usize = 53;
const SYNC_MINING_COORDINATE_LEN: usize = 12;
const CHUNK_MINE_WITH_REWARDS_TAG: u8 = 8;
const CHUNK_FELL_TREE_WITH_REWARDS_TAG: u8 = 9;
const PLAYER_PROFILE_SEED: &[u8] = b"player-v7";
const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
const PLAYER_PROFILE_LEN: usize = 773;
const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;
const PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET: usize = 393;
const BACKPACK_SEED: &[u8] = b"backpack";
const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
const BACKPACK_VERSION: u16 = 4;
const BACKPACK_LEN: usize = 8_048;
const BACKPACK_ID_OFFSET: usize = 12;
const BACKPACK_OWNER_OFFSET: usize = 20;
const BACKPACK_FLAGS_OFFSET: usize = 55;
const BACKPACK_TOTAL_MASS_INITIALIZED: u8 = 1;
const BACKPACK_LAST_MINE_PRE_MASS_OFFSET: usize = 98;
const BACKPACK_MINE_SEQUENCE_OFFSET: usize = 114;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkSkillsError::InvalidInstruction)?;
    match *tag {
        0 => initialize_rule_table(program_id, accounts),
        1 => set_skill_thresholds(program_id, accounts, payload),
        2 => upsert_source_rule(program_id, accounts, payload),
        3 => sync_player_skills(program_id, accounts, payload),
        4 => set_rule_table_authority(program_id, accounts),
        5 => set_mining_travel_rule(program_id, accounts, payload),
        6 => set_burden_mining_rule(program_id, accounts, payload),
        _ => Err(NicechunkSkillsError::InvalidInstruction.into()),
    }
}

fn set_burden_mining_rule(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 18 {
        return Err(NicechunkSkillsError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let rule_table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    validate_rule_admin_accounts(program_id, authority, rule_table, global_config)?;
    let rule = BurdenMiningRule {
        enabled: payload[0] != 0,
        skill_index: payload[1],
        max_effective_mass_grams: read_u64(payload, 2),
        work_per_xp: read_u64(payload, 10),
    };
    let clock = Clock::get()?;
    let mut data = rule_table.try_borrow_mut_data()?;
    RuleTableState::set_burden_mining_rule(
        &mut data,
        global_config.key,
        authority.key,
        &rule,
        clock.slot,
    )
}

fn initialize_rule_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkSkillsError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let rule_table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !authority.is_signer || !authority.is_writable {
        return Err(NicechunkSkillsError::InvalidPayer.into());
    }
    if !rule_table.is_writable {
        return Err(NicechunkSkillsError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkSkillsError::InvalidSystemProgram,
    )?;
    let treasury = validate_global_config(global_config)?;
    if authority.key != &treasury && authority.key != &NICECHUNK_BOOTSTRAP_AUTHORITY {
        return Err(NicechunkSkillsError::UnauthorizedAuthority.into());
    }
    let (expected_rule_table, bump) =
        Pubkey::find_program_address(&[RULE_TABLE_SEED, global_config.key.as_ref()], program_id);
    require_key_eq(
        rule_table.key,
        &expected_rule_table,
        NicechunkSkillsError::InvalidRuleTablePda,
    )?;
    if rule_table.owner == program_id {
        return Err(NicechunkSkillsError::RuleTableAlreadyInitialized.into());
    }
    create_or_allocate_pda(
        authority,
        rule_table,
        system_program_account,
        program_id,
        RULE_TABLE_LEN,
        &[RULE_TABLE_SEED, global_config.key.as_ref(), &[bump]],
    )?;
    let clock = Clock::get()?;
    let mut data = rule_table.try_borrow_mut_data()?;
    RuleTableState::pack_empty(
        &mut data,
        bump,
        authority.key,
        global_config.key,
        clock.slot,
        clock.unix_timestamp,
    )
}

fn set_skill_thresholds(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 1 + LEVEL_COUNT * 8 {
        return Err(NicechunkSkillsError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let rule_table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    validate_rule_admin_accounts(program_id, authority, rule_table, global_config)?;

    let mut thresholds = [0_u64; LEVEL_COUNT];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        *threshold = read_u64(payload, 1 + index * 8);
    }
    let clock = Clock::get()?;
    let mut data = rule_table.try_borrow_mut_data()?;
    RuleTableState::set_thresholds(
        &mut data,
        global_config.key,
        authority.key,
        payload[0] as usize,
        &thresholds,
        clock.slot,
    )
}

fn upsert_source_rule(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 1 + RULE_RECORD_LEN {
        return Err(NicechunkSkillsError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let rule_table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    validate_rule_admin_accounts(program_id, authority, rule_table, global_config)?;

    let rule = SourceRule::unpack(&payload[1..])?;
    let clock = Clock::get()?;
    let mut data = rule_table.try_borrow_mut_data()?;
    RuleTableState::upsert_rule(
        &mut data,
        global_config.key,
        authority.key,
        payload[0] as usize,
        &rule,
        clock.slot,
    )
}

fn set_rule_table_authority(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkSkillsError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let rule_table = next_account_info(account_info_iter)?;
    let new_authority = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    validate_rule_admin_accounts(program_id, authority, rule_table, global_config)?;
    let clock = Clock::get()?;
    let mut data = rule_table.try_borrow_mut_data()?;
    RuleTableState::set_authority(
        &mut data,
        global_config.key,
        authority.key,
        new_authority.key,
        clock.slot,
    )
}

fn set_mining_travel_rule(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 6 {
        return Err(NicechunkSkillsError::InvalidInstruction.into());
    }
    let account_info_iter = &mut accounts.iter();
    let authority = next_account_info(account_info_iter)?;
    let rule_table = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    validate_rule_admin_accounts(program_id, authority, rule_table, global_config)?;

    let rule = MiningTravelRule {
        enabled: payload[0] != 0,
        minimum_distance: u16::from_le_bytes([payload[1], payload[2]]),
        skill_index: payload[3],
        xp_award: u16::from_le_bytes([payload[4], payload[5]]),
    };
    let clock = Clock::get()?;
    let mut data = rule_table.try_borrow_mut_data()?;
    RuleTableState::set_mining_travel_rule(
        &mut data,
        global_config.key,
        authority.key,
        &rule,
        clock.slot,
    )
}

fn sync_player_skills(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() < 6 {
        return Err(NicechunkSkillsError::InvalidAccountCount.into());
    }
    let payer = &accounts[0];
    let owner = &accounts[1];
    let player_skills = &accounts[2];
    let rule_table = &accounts[3];
    let global_config = &accounts[4];
    let system_program_account = &accounts[5];
    let mining_coordinate = parse_optional_mining_coordinate(payload)?;
    let source_accounts = if mining_coordinate.is_some() {
        if accounts.len() < 7 {
            return Err(NicechunkSkillsError::InvalidAccountCount.into());
        }
        validate_mining_proof(
            &accounts[6],
            mining_coordinate.expect("coordinate checked above"),
            owner.key,
            global_config.key,
        )?;
        &accounts[7..]
    } else {
        &accounts[6..]
    };

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkSkillsError::InvalidPayer.into());
    }
    if !player_skills.is_writable {
        return Err(NicechunkSkillsError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkSkillsError::InvalidSystemProgram,
    )?;
    validate_global_config(global_config)?;
    require_key_eq(
        rule_table.owner,
        program_id,
        NicechunkSkillsError::InvalidRuleTableOwner,
    )?;
    let (expected_rule_table, _) =
        Pubkey::find_program_address(&[RULE_TABLE_SEED, global_config.key.as_ref()], program_id);
    require_key_eq(
        rule_table.key,
        &expected_rule_table,
        NicechunkSkillsError::InvalidRuleTablePda,
    )?;
    let (expected_player_skills, bump) = Pubkey::find_program_address(
        &[
            PLAYER_SKILLS_SEED,
            global_config.key.as_ref(),
            owner.key.as_ref(),
        ],
        program_id,
    );
    require_key_eq(
        player_skills.key,
        &expected_player_skills,
        NicechunkSkillsError::InvalidPlayerSkillsPda,
    )?;

    let clock = Clock::get()?;
    if player_skills.owner != program_id {
        create_or_allocate_pda(
            payer,
            player_skills,
            system_program_account,
            program_id,
            PLAYER_SKILLS_LEN,
            &[
                PLAYER_SKILLS_SEED,
                global_config.key.as_ref(),
                owner.key.as_ref(),
                &[bump],
            ],
        )?;
        let mut data = player_skills.try_borrow_mut_data()?;
        PlayerSkillsState::pack_empty(
            &mut data,
            bump,
            owner.key,
            global_config.key,
            clock.slot,
            clock.unix_timestamp,
        )?;
    }
    require_key_eq(
        player_skills.owner,
        program_id,
        NicechunkSkillsError::InvalidPlayerSkillsOwner,
    )?;

    let rule_table_data = rule_table.try_borrow_data()?;
    let table_state = RuleTableState::validate(&rule_table_data, global_config.key)?;
    let mut player_skills_data = player_skills.try_borrow_mut_data()?;
    PlayerSkillsState::validate(&player_skills_data, owner.key, global_config.key)?;

    for rule_index in 0..table_state.rule_count as usize {
        let rule = RuleTableState::rule(&rule_table_data, rule_index)?;
        if !rule.enabled {
            continue;
        }
        let expected_source = derive_source_pda(&rule, global_config.key, owner.key)?;
        let Some(source) = source_accounts
            .iter()
            .find(|account| account.key == &expected_source)
        else {
            continue;
        };
        let current_counter =
            validate_and_read_source(source, &rule, global_config.key, owner.key)?;
        PlayerSkillsState::apply_counter(
            &mut player_skills_data,
            owner.key,
            global_config.key,
            rule_index,
            &rule,
            current_counter,
        )?;
    }
    if let Some(rule) = RuleTableState::burden_mining_rule(&rule_table_data)? {
        if let Some((pre_mine_mass_grams, mine_sequence)) =
            burden_snapshot_from_sources(source_accounts, owner.key, global_config.key)?
        {
            PlayerSkillsState::apply_burden_mining_action(
                &mut player_skills_data,
                owner.key,
                global_config.key,
                rule,
                pre_mine_mass_grams,
                mine_sequence,
            )?;
        }
    }
    if let Some(coordinate) = mining_coordinate {
        let rule = RuleTableState::mining_travel_rule(&rule_table_data)?;
        PlayerSkillsState::record_mining_coordinate(
            &mut player_skills_data,
            owner.key,
            global_config.key,
            coordinate,
            rule,
            clock.slot,
        )?;
    }
    PlayerSkillsState::recompute_levels(
        &mut player_skills_data,
        owner.key,
        global_config.key,
        &rule_table_data,
        clock.slot,
    )
}

fn parse_optional_mining_coordinate(
    payload: &[u8],
) -> Result<Option<MiningCoordinate>, solana_program::program_error::ProgramError> {
    match payload.len() {
        0 => Ok(None),
        SYNC_MINING_COORDINATE_LEN => Ok(Some(MiningCoordinate {
            x: read_i32(payload, 0),
            y: read_i32(payload, 4),
            z: read_i32(payload, 8),
        })),
        _ => Err(NicechunkSkillsError::InvalidInstruction.into()),
    }
}

fn validate_mining_proof(
    instructions_sysvar: &AccountInfo,
    coordinate: MiningCoordinate,
    owner: &Pubkey,
    global_config: &Pubkey,
) -> ProgramResult {
    require_key_eq(
        instructions_sysvar.key,
        &solana_program::sysvar::instructions::ID,
        NicechunkSkillsError::InvalidInstructionsSysvar,
    )?;
    let (player_progress, _) = Pubkey::find_program_address(
        &[b"player-progress", global_config.as_ref(), owner.as_ref()],
        &NICECHUNK_CHUNK_PROGRAM_ID,
    );
    let current_index = load_current_index_checked(instructions_sysvar)
        .map_err(|_| NicechunkSkillsError::InvalidInstructionsSysvar)?;
    for index in (0..current_index as usize).rev() {
        let instruction = load_instruction_at_checked(index, instructions_sysvar)
            .map_err(|_| NicechunkSkillsError::InvalidInstructionsSysvar)?;
        if mining_instruction_matches(&instruction, coordinate, &player_progress, global_config) {
            return Ok(());
        }
    }
    Err(NicechunkSkillsError::InvalidMiningProof.into())
}

fn mining_instruction_matches(
    instruction: &Instruction,
    coordinate: MiningCoordinate,
    player_progress: &Pubkey,
    global_config: &Pubkey,
) -> bool {
    if instruction.program_id != NICECHUNK_CHUNK_PROGRAM_ID || instruction.data.len() != 13 {
        return false;
    }
    let (progress_index, global_config_index) = match instruction.data[0] {
        CHUNK_MINE_WITH_REWARDS_TAG => (3, 6),
        CHUNK_FELL_TREE_WITH_REWARDS_TAG => (3, 4),
        _ => return false,
    };
    if instruction.accounts.len() <= global_config_index
        || instruction.accounts[progress_index].pubkey != *player_progress
        || instruction.accounts[global_config_index].pubkey != *global_config
    {
        return false;
    }
    MiningCoordinate {
        x: read_i32(&instruction.data, 1),
        y: read_i16(&instruction.data, 5) as i32,
        z: read_i32(&instruction.data, 7),
    } == coordinate
}

fn validate_rule_admin_accounts(
    program_id: &Pubkey,
    authority: &AccountInfo,
    rule_table: &AccountInfo,
    global_config: &AccountInfo,
) -> ProgramResult {
    if !authority.is_signer {
        return Err(NicechunkSkillsError::UnauthorizedAuthority.into());
    }
    if !rule_table.is_writable {
        return Err(NicechunkSkillsError::InvalidWritableAccount.into());
    }
    validate_global_config(global_config)?;
    require_key_eq(
        rule_table.owner,
        program_id,
        NicechunkSkillsError::InvalidRuleTableOwner,
    )?;
    let (expected_rule_table, _) =
        Pubkey::find_program_address(&[RULE_TABLE_SEED, global_config.key.as_ref()], program_id);
    require_key_eq(
        rule_table.key,
        &expected_rule_table,
        NicechunkSkillsError::InvalidRuleTablePda,
    )?;
    let data = rule_table.try_borrow_data()?;
    RuleTableState::validate_authority(&data, global_config.key, authority.key)
}

fn validate_global_config(
    global_config: &AccountInfo,
) -> Result<Pubkey, solana_program::program_error::ProgramError> {
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkSkillsError::InvalidGlobalConfigOwner,
    )?;
    let data = global_config.try_borrow_data()?;
    if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
        return Err(NicechunkSkillsError::InvalidGlobalConfig.into());
    }
    let bytes: [u8; 32] = data
        [GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET..GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET + 32]
        .try_into()
        .map_err(|_| NicechunkSkillsError::InvalidGlobalConfig)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn derive_source_pda(
    rule: &SourceRule,
    global_config: &Pubkey,
    owner: &Pubkey,
) -> Result<Pubkey, solana_program::program_error::ProgramError> {
    let (address, _) = match rule.seed_layout {
        SOURCE_SEED_GLOBAL_OWNER => Pubkey::find_program_address(
            &[rule.seed(), global_config.as_ref(), owner.as_ref()],
            &rule.source_program,
        ),
        SOURCE_SEED_OWNER => {
            Pubkey::find_program_address(&[rule.seed(), owner.as_ref()], &rule.source_program)
        }
        _ => return Err(NicechunkSkillsError::InvalidRule.into()),
    };
    Ok(address)
}

fn validate_and_read_source(
    source: &AccountInfo,
    rule: &SourceRule,
    global_config: &Pubkey,
    owner: &Pubkey,
) -> Result<u64, solana_program::program_error::ProgramError> {
    require_key_eq(
        source.owner,
        &rule.source_program,
        NicechunkSkillsError::InvalidSourceOwner,
    )?;
    let data = source.try_borrow_data()?;
    if data.len() < 8 || data[0..8] != rule.source_magic {
        return Err(NicechunkSkillsError::InvalidSourceData.into());
    }
    validate_embedded_pubkey(&data, rule.owner_offset as usize, owner)?;
    validate_embedded_pubkey(&data, rule.global_config_offset as usize, global_config)?;
    rule.counter_from_source(&data).map_err(Into::into)
}

fn burden_snapshot_from_sources(
    source_accounts: &[AccountInfo],
    owner: &Pubkey,
    global_config: &Pubkey,
) -> Result<Option<(u64, u64)>, solana_program::program_error::ProgramError> {
    let (expected_profile, _) = Pubkey::find_program_address(
        &[PLAYER_PROFILE_SEED, owner.as_ref()],
        &NICECHUNK_PLAYER_PROGRAM_ID,
    );
    let Some(profile) = source_accounts
        .iter()
        .find(|account| account.key == &expected_profile)
    else {
        return Ok(None);
    };
    require_key_eq(
        profile.owner,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkSkillsError::InvalidSourceOwner,
    )?;
    let profile_data = profile.try_borrow_data()?;
    if profile_data.len() != PLAYER_PROFILE_LEN
        || profile_data[0..8] != PLAYER_PROFILE_MAGIC
        || &profile_data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
            != owner.as_ref()
        || &profile_data
            [PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
    {
        return Err(NicechunkSkillsError::InvalidSourceData.into());
    }
    let equipped_backpack = read_pubkey_at(
        &profile_data,
        PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET,
        NicechunkSkillsError::InvalidSourceData,
    )?;
    drop(profile_data);
    if equipped_backpack == Pubkey::default() {
        return Ok(None);
    }
    let Some(backpack) = source_accounts
        .iter()
        .find(|account| account.key == &equipped_backpack)
    else {
        return Ok(None);
    };
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkSkillsError::InvalidSourceOwner,
    )?;
    let data = backpack.try_borrow_data()?;
    if data.len() != BACKPACK_LEN
        || data[0..8] != BACKPACK_MAGIC
        || read_u16(&data, 8) != BACKPACK_VERSION
        || data[11] != 1
        || &data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32] != owner.as_ref()
    {
        return Err(NicechunkSkillsError::InvalidBackpackSource.into());
    }
    let backpack_id = read_u64(&data, BACKPACK_ID_OFFSET);
    let backpack_id_bytes = backpack_id.to_le_bytes();
    let (expected_backpack, _) = Pubkey::find_program_address(
        &[BACKPACK_SEED, owner.as_ref(), &backpack_id_bytes],
        &NICECHUNK_BACKPACK_PROGRAM_ID,
    );
    require_key_eq(
        backpack.key,
        &expected_backpack,
        NicechunkSkillsError::InvalidBackpackSource,
    )?;
    if data[BACKPACK_FLAGS_OFFSET] & BACKPACK_TOTAL_MASS_INITIALIZED == 0 {
        return Err(NicechunkSkillsError::BackpackMassMigrationRequired.into());
    }
    Ok(Some((
        read_u64(&data, BACKPACK_LAST_MINE_PRE_MASS_OFFSET),
        read_u64(&data, BACKPACK_MINE_SEQUENCE_OFFSET),
    )))
}

fn validate_embedded_pubkey(data: &[u8], offset: usize, expected: &Pubkey) -> ProgramResult {
    let Some(bytes) = data.get(offset..offset + 32) else {
        return Err(NicechunkSkillsError::InvalidSourceData.into());
    };
    if bytes != expected.as_ref() {
        return Err(NicechunkSkillsError::InvalidSourceAccount.into());
    }
    Ok(())
}

fn create_or_allocate_pda<'a>(
    payer: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    data_len: usize,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    if account.owner != &system_program::ID || account.data_len() != 0 {
        return Err(NicechunkSkillsError::InvalidSystemAccount.into());
    }
    let required_lamports = Rent::get()?.minimum_balance(data_len);
    if account.lamports() == 0 {
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                account.key,
                required_lamports,
                data_len as u64,
                program_id,
            ),
            &[
                payer.clone(),
                account.clone(),
                system_program_account.clone(),
            ],
            &[signer_seeds],
        )?;
        return Ok(());
    }
    if account.lamports() < required_lamports {
        solana_program::program::invoke(
            &system_instruction::transfer(
                payer.key,
                account.key,
                required_lamports.saturating_sub(account.lamports()),
            ),
            &[
                payer.clone(),
                account.clone(),
                system_program_account.clone(),
            ],
        )?;
    }
    invoke_signed(
        &system_instruction::allocate(account.key, data_len as u64),
        &[account.clone(), system_program_account.clone()],
        &[signer_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(account.key, program_id),
        &[account.clone(), system_program_account.clone()],
        &[signer_seeds],
    )
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_i16(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_pubkey_at(
    data: &[u8],
    offset: usize,
    error: NicechunkSkillsError,
) -> Result<Pubkey, solana_program::program_error::ProgramError> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(error)?
        .try_into()
        .map_err(|_| error)?;
    Ok(Pubkey::new_from_array(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::instruction::AccountMeta;
    use state::{SKILL_COUNT, SOURCE_SEED_MAX_LEN};

    fn sample_rule(seed_layout: u8) -> SourceRule {
        let mut seed = [0_u8; SOURCE_SEED_MAX_LEN];
        seed[..15].copy_from_slice(b"player-progress");
        SourceRule {
            enabled: true,
            metric_width: 8,
            flags: 1,
            seed_layout,
            rule_id: 1,
            source_program: Pubkey::new_unique(),
            source_magic: *b"NCKPRG01",
            seed,
            seed_len: 15,
            owner_offset: 12,
            global_config_offset: 44,
            metric_offset: 76,
            max_delta_per_sync: 1_000,
            unit_divisor: 1,
            xp_per_unit: [1_u32; SKILL_COUNT],
        }
    }

    #[test]
    fn source_pda_derivation_supports_both_live_layouts() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        for layout in [SOURCE_SEED_GLOBAL_OWNER, SOURCE_SEED_OWNER] {
            let rule = sample_rule(layout);
            let derived = derive_source_pda(&rule, &global_config, &owner).unwrap();
            let expected = match layout {
                SOURCE_SEED_GLOBAL_OWNER => {
                    Pubkey::find_program_address(
                        &[rule.seed(), global_config.as_ref(), owner.as_ref()],
                        &rule.source_program,
                    )
                    .0
                }
                _ => {
                    Pubkey::find_program_address(
                        &[rule.seed(), owner.as_ref()],
                        &rule.source_program,
                    )
                    .0
                }
            };
            assert_eq!(derived, expected);
        }
    }

    #[test]
    fn mining_proof_matches_reward_mine_and_tree_layouts() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let player_progress = Pubkey::find_program_address(
            &[b"player-progress", global_config.as_ref(), owner.as_ref()],
            &NICECHUNK_CHUNK_PROGRAM_ID,
        )
        .0;
        let coordinate = MiningCoordinate {
            x: -245,
            y: 91,
            z: 1_024,
        };
        for (tag, global_index, account_count) in [
            (CHUNK_MINE_WITH_REWARDS_TAG, 6_usize, 12_usize),
            (CHUNK_FELL_TREE_WITH_REWARDS_TAG, 4_usize, 9_usize),
        ] {
            let mut data = vec![0_u8; 13];
            data[0] = tag;
            data[1..5].copy_from_slice(&coordinate.x.to_le_bytes());
            data[5..7].copy_from_slice(&(coordinate.y as i16).to_le_bytes());
            data[7..11].copy_from_slice(&coordinate.z.to_le_bytes());
            data[11..13].copy_from_slice(&22_u16.to_le_bytes());
            let mut accounts =
                vec![AccountMeta::new_readonly(Pubkey::new_unique(), false); account_count];
            accounts[3].pubkey = player_progress;
            accounts[global_index].pubkey = global_config;
            let instruction = Instruction {
                program_id: NICECHUNK_CHUNK_PROGRAM_ID,
                accounts,
                data,
            };
            assert!(mining_instruction_matches(
                &instruction,
                coordinate,
                &player_progress,
                &global_config,
            ));
        }
    }

    #[test]
    fn mining_proof_rejects_untrusted_coordinate_and_program() {
        let player_progress = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; 13];
        data[0] = CHUNK_MINE_WITH_REWARDS_TAG;
        data[1..5].copy_from_slice(&10_i32.to_le_bytes());
        data[5..7].copy_from_slice(&20_i16.to_le_bytes());
        data[7..11].copy_from_slice(&30_i32.to_le_bytes());
        let mut accounts = vec![AccountMeta::new_readonly(Pubkey::new_unique(), false); 12];
        accounts[3].pubkey = player_progress;
        accounts[6].pubkey = global_config;
        let mut instruction = Instruction {
            program_id: NICECHUNK_CHUNK_PROGRAM_ID,
            accounts,
            data,
        };
        assert!(!mining_instruction_matches(
            &instruction,
            MiningCoordinate {
                x: 11,
                y: 20,
                z: 30
            },
            &player_progress,
            &global_config,
        ));
        instruction.program_id = Pubkey::new_unique();
        assert!(!mining_instruction_matches(
            &instruction,
            MiningCoordinate {
                x: 10,
                y: 20,
                z: 30
            },
            &player_progress,
            &global_config,
        ));
    }
}
