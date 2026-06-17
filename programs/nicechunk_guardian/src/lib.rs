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

use cluster_config::{DEVELOPMENT_WALLET, NCK_MINT, NICECHUNK_CORE_PROGRAM_ID};
use errors::{require_key_eq, NicechunkGuardianError};
use state::{
    validate_host, validate_port, GuardianRegion, GuardianRegionInitArgs, GuardianRegistry,
    GUARDIAN_REGION_SEED, GUARDIAN_REGISTRY_SEED, GUARDIAN_STAKE_AMOUNT,
    GUARDIAN_TREASURY_AUTHORITY_SEED, REGION_STATUS_ACTIVE,
};

declare_id!("6frJyJSirfEwsztsxijcJLe29LSaceET1wanXSFwPQyE");

const GLOBAL_CONFIG_LEN: usize = 293;
const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
const TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;
const TOKEN_ACCOUNT_MIN_LEN: usize = 72;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkGuardianError::InvalidInstruction)?;

    match tag {
        0 => initialize_registry(program_id, accounts),
        1 => register_genesis_guardian(program_id, accounts, payload),
        2 => register_guardian(program_id, accounts, payload),
        3 => submit_guardian_proof(program_id, accounts, payload),
        4 => settle_guardian(program_id, accounts, payload),
        5 => update_guardian_endpoint(program_id, accounts, payload),
        _ => Err(NicechunkGuardianError::InvalidInstruction.into()),
    }
}

fn initialize_registry(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 7 {
        return Err(NicechunkGuardianError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let registry = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let treasury_authority = next_account_info(account_info_iter)?;
    let treasury_nck_token = next_account_info(account_info_iter)?;
    let nck_mint = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkGuardianError::InvalidPayer.into());
    }
    if !registry.is_writable {
        return Err(NicechunkGuardianError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkGuardianError::InvalidSystemProgram,
    )?;
    validate_global_config(global_config)?;
    require_key_eq(
        nck_mint.key,
        &NCK_MINT,
        NicechunkGuardianError::InvalidNckMint,
    )?;

    let (expected_registry, registry_bump) = derive_registry_pda(program_id, global_config.key);
    require_key_eq(
        registry.key,
        &expected_registry,
        NicechunkGuardianError::InvalidRegistryPda,
    )?;
    let (expected_treasury_authority, treasury_bump) =
        derive_treasury_authority_pda(program_id, global_config.key);
    require_key_eq(
        treasury_authority.key,
        &expected_treasury_authority,
        NicechunkGuardianError::InvalidTreasuryAuthority,
    )?;
    validate_token_account(treasury_nck_token, &NCK_MINT, treasury_authority.key)?;

    if registry.owner == program_id {
        return Err(NicechunkGuardianError::RegistryAlreadyInitialized.into());
    }
    if registry.owner != &system_program::ID || registry.data_len() != 0 {
        return Err(NicechunkGuardianError::InvalidSystemAccount.into());
    }

    create_or_allocate_pda(
        payer,
        registry,
        system_program_account,
        program_id,
        GuardianRegistry::LEN,
        &[
            GUARDIAN_REGISTRY_SEED,
            global_config.key.as_ref(),
            &[registry_bump],
        ],
    )?;

    let clock = Clock::get()?;
    let mut data = registry.try_borrow_mut_data()?;
    GuardianRegistry::pack(
        &mut data,
        registry_bump,
        treasury_bump,
        global_config.key,
        nck_mint.key,
        treasury_nck_token.key,
        clock.slot,
        clock.unix_timestamp,
    )
}

fn register_genesis_guardian(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 11 {
        return Err(NicechunkGuardianError::InvalidAccountCount.into());
    }
    register_guardian_inner(program_id, accounts, payload, true)
}

fn register_guardian(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 15 {
        return Err(NicechunkGuardianError::InvalidAccountCount.into());
    }
    register_guardian_inner(program_id, accounts, payload, false)
}

fn register_guardian_inner(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
    is_genesis: bool,
) -> ProgramResult {
    let args = RegisterGuardianArgs::unpack(payload)?;

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let owner = next_account_info(account_info_iter)?;
    let owner_nck_token = next_account_info(account_info_iter)?;
    let registry = next_account_info(account_info_iter)?;
    let guardian_region = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let treasury_authority = next_account_info(account_info_iter)?;
    let treasury_nck_token = next_account_info(account_info_iter)?;
    let nck_mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer || !payer.is_writable {
        return Err(NicechunkGuardianError::InvalidPayer.into());
    }
    if !owner.is_signer {
        return Err(NicechunkGuardianError::InvalidGuardianOwner.into());
    }
    if !guardian_region.is_writable || !registry.is_writable || !treasury_nck_token.is_writable {
        return Err(NicechunkGuardianError::InvalidWritableAccount.into());
    }
    validate_host(args.host)?;
    validate_port(args.port)?;
    validate_global_config(global_config)?;
    require_key_eq(
        nck_mint.key,
        &NCK_MINT,
        NicechunkGuardianError::InvalidNckMint,
    )?;
    require_key_eq(
        token_program.key,
        &spl_token::ID,
        NicechunkGuardianError::InvalidTokenProgram,
    )?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkGuardianError::InvalidSystemProgram,
    )?;

    let (expected_registry, _) = derive_registry_pda(program_id, global_config.key);
    require_key_eq(
        registry.key,
        &expected_registry,
        NicechunkGuardianError::InvalidRegistryPda,
    )?;
    require_key_eq(
        registry.owner,
        program_id,
        NicechunkGuardianError::InvalidRegistryOwner,
    )?;

    let (expected_treasury_authority, _) =
        derive_treasury_authority_pda(program_id, global_config.key);
    require_key_eq(
        treasury_authority.key,
        &expected_treasury_authority,
        NicechunkGuardianError::InvalidTreasuryAuthority,
    )?;

    {
        let registry_data = registry.try_borrow_data()?;
        GuardianRegistry::validate(&registry_data, global_config.key)?;
        let treasury = GuardianRegistry::treasury_token(&registry_data)?;
        require_key_eq(
            treasury_nck_token.key,
            &treasury,
            NicechunkGuardianError::InvalidTokenAccount,
        )?;
        if is_genesis {
            if GuardianRegistry::genesis_registered(&registry_data)? {
                return Err(NicechunkGuardianError::GenesisAlreadyRegistered.into());
            }
            require_key_eq(
                owner.key,
                &DEVELOPMENT_WALLET,
                NicechunkGuardianError::NoGenesisPermission,
            )?;
        }
    }

    validate_token_account(owner_nck_token, &NCK_MINT, owner.key)?;
    validate_token_account(treasury_nck_token, &NCK_MINT, treasury_authority.key)?;
    let (expected_region, region_bump) =
        derive_region_pda(program_id, global_config.key, args.region_x, args.region_y);
    require_key_eq(
        guardian_region.key,
        &expected_region,
        NicechunkGuardianError::InvalidGuardianRegionPda,
    )?;

    if guardian_region.owner == program_id {
        let data = guardian_region.try_borrow_data()?;
        if GuardianRegion::status(&data)? == REGION_STATUS_ACTIVE {
            return Err(NicechunkGuardianError::GuardianRegionAlreadyActive.into());
        }
    } else if guardian_region.owner != &system_program::ID || guardian_region.data_len() != 0 {
        return Err(NicechunkGuardianError::InvalidSystemAccount.into());
    }

    if !is_genesis {
        validate_adjacent_guardian_exists(
            program_id,
            account_info_iter.as_slice(),
            global_config.key,
            args.region_x,
            args.region_y,
        )?;
    }

    transfer_stake_to_treasury(
        owner_nck_token,
        treasury_nck_token,
        nck_mint,
        owner,
        token_program,
    )?;

    if guardian_region.owner != program_id {
        let region_x_bytes = args.region_x.to_le_bytes();
        let region_y_bytes = args.region_y.to_le_bytes();
        create_or_allocate_pda(
            payer,
            guardian_region,
            system_program_account,
            program_id,
            GuardianRegion::LEN,
            &[
                GUARDIAN_REGION_SEED,
                global_config.key.as_ref(),
                &region_x_bytes,
                &region_y_bytes,
                &[region_bump],
            ],
        )?;
    }

    let clock = Clock::get()?;
    let mut region_data = guardian_region.try_borrow_mut_data()?;
    GuardianRegion::pack(
        &mut region_data,
        &GuardianRegionInitArgs {
            bump: region_bump,
            status: REGION_STATUS_ACTIVE,
            region_x: args.region_x,
            region_y: args.region_y,
            owner: owner.key,
            operator: &args.operator,
            global_config: global_config.key,
            host: args.host,
            port: args.port,
            use_tls: args.use_tls,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )?;
    drop(region_data);

    let mut registry_data = registry.try_borrow_mut_data()?;
    GuardianRegistry::add_active(&mut registry_data, is_genesis)
}

fn submit_guardian_proof(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 || payload.len() != 8 {
        return Err(NicechunkGuardianError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let operator = next_account_info(account_info_iter)?;
    let registry = next_account_info(account_info_iter)?;
    let guardian_region = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    if !operator.is_signer {
        return Err(NicechunkGuardianError::InvalidOperatorAuthority.into());
    }
    if !guardian_region.is_writable || !registry.is_writable {
        return Err(NicechunkGuardianError::InvalidWritableAccount.into());
    }
    validate_registry_and_region_accounts(
        program_id,
        registry,
        guardian_region,
        global_config,
        payload,
    )?;

    let clock = Clock::get()?;
    let mut region_data = guardian_region.try_borrow_mut_data()?;
    let removed = GuardianRegion::settle(&mut region_data, clock.unix_timestamp)?;
    if removed {
        drop(region_data);
        let mut registry_data = registry.try_borrow_mut_data()?;
        GuardianRegistry::remove_active(&mut registry_data)?;
        return Err(NicechunkGuardianError::GuardianNotActive.into());
    }
    GuardianRegion::proof(
        &mut region_data,
        operator.key,
        clock.unix_timestamp,
        clock.slot,
    )
}

fn settle_guardian(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len() != 3 || payload.len() != 8 {
        return Err(NicechunkGuardianError::InvalidInstruction.into());
    }

    let account_info_iter = &mut accounts.iter();
    let registry = next_account_info(account_info_iter)?;
    let guardian_region = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    if !guardian_region.is_writable || !registry.is_writable {
        return Err(NicechunkGuardianError::InvalidWritableAccount.into());
    }
    validate_registry_and_region_accounts(
        program_id,
        registry,
        guardian_region,
        global_config,
        payload,
    )?;

    let clock = Clock::get()?;
    let mut region_data = guardian_region.try_borrow_mut_data()?;
    let removed = GuardianRegion::settle(&mut region_data, clock.unix_timestamp)?;
    drop(region_data);
    if removed {
        let mut registry_data = registry.try_borrow_mut_data()?;
        GuardianRegistry::remove_active(&mut registry_data)?;
    }
    Ok(())
}

fn update_guardian_endpoint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkGuardianError::InvalidAccountCount.into());
    }
    let args = UpdateGuardianEndpointArgs::unpack(payload)?;
    validate_host(args.host)?;
    validate_port(args.port)?;

    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let registry = next_account_info(account_info_iter)?;
    let guardian_region = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;

    if !owner.is_signer {
        return Err(NicechunkGuardianError::InvalidGuardianOwner.into());
    }
    if !guardian_region.is_writable {
        return Err(NicechunkGuardianError::InvalidWritableAccount.into());
    }

    validate_registry_and_region_accounts(
        program_id,
        registry,
        guardian_region,
        global_config,
        payload,
    )?;

    let clock = Clock::get()?;
    let mut region_data = guardian_region.try_borrow_mut_data()?;
    GuardianRegion::update_endpoint(
        &mut region_data,
        owner.key,
        args.host,
        args.port,
        args.use_tls,
        clock.slot,
    )
}

fn validate_registry_and_region_accounts(
    program_id: &Pubkey,
    registry: &AccountInfo,
    guardian_region: &AccountInfo,
    global_config: &AccountInfo,
    payload: &[u8],
) -> ProgramResult {
    validate_global_config(global_config)?;
    require_key_eq(
        registry.owner,
        program_id,
        NicechunkGuardianError::InvalidRegistryOwner,
    )?;
    require_key_eq(
        guardian_region.owner,
        program_id,
        NicechunkGuardianError::InvalidRegistryOwner,
    )?;
    let region_x = read_i32(payload, 0);
    let region_y = read_i32(payload, 4);
    let (expected_registry, _) = derive_registry_pda(program_id, global_config.key);
    let (expected_region, _) = derive_region_pda(program_id, global_config.key, region_x, region_y);
    require_key_eq(
        registry.key,
        &expected_registry,
        NicechunkGuardianError::InvalidRegistryPda,
    )?;
    require_key_eq(
        guardian_region.key,
        &expected_region,
        NicechunkGuardianError::InvalidGuardianRegionPda,
    )?;
    let registry_data = registry.try_borrow_data()?;
    GuardianRegistry::validate(&registry_data, global_config.key)?;
    drop(registry_data);
    let region_data = guardian_region.try_borrow_data()?;
    GuardianRegion::validate_active(&region_data, global_config.key, region_x, region_y)
}

fn validate_global_config(global_config: &AccountInfo) -> ProgramResult {
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkGuardianError::InvalidGlobalConfigOwner,
    )?;
    let data = global_config.try_borrow_data()?;
    if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
        return Err(NicechunkGuardianError::InvalidRegistryData.into());
    }
    Ok(())
}

fn validate_adjacent_guardian_exists(
    program_id: &Pubkey,
    neighbors: &[AccountInfo],
    global_config: &Pubkey,
    region_x: i32,
    region_y: i32,
) -> ProgramResult {
    if neighbors.len() != 4 {
        return Err(NicechunkGuardianError::InvalidAccountCount.into());
    }

    let expected = [
        (region_x.saturating_add(1), region_y),
        (region_x.saturating_sub(1), region_y),
        (region_x, region_y.saturating_add(1)),
        (region_x, region_y.saturating_sub(1)),
    ];
    let mut found_active = false;
    for (account, (neighbor_x, neighbor_y)) in neighbors.iter().zip(expected) {
        let (expected_pda, _) =
            derive_region_pda(program_id, global_config, neighbor_x, neighbor_y);
        require_key_eq(
            account.key,
            &expected_pda,
            NicechunkGuardianError::InvalidAdjacentGuardian,
        )?;
        if account.owner != program_id {
            continue;
        }
        let data = account.try_borrow_data()?;
        if GuardianRegion::validate_active(&data, global_config, neighbor_x, neighbor_y).is_ok() {
            found_active = true;
        }
    }

    if !found_active {
        return Err(NicechunkGuardianError::MissingAdjacentGuardian.into());
    }
    Ok(())
}

fn transfer_stake_to_treasury<'a>(
    owner_nck_token: &AccountInfo<'a>,
    treasury_nck_token: &AccountInfo<'a>,
    nck_mint: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = spl_token::instruction::transfer_checked(
        token_program.key,
        owner_nck_token.key,
        nck_mint.key,
        treasury_nck_token.key,
        owner.key,
        &[],
        GUARDIAN_STAKE_AMOUNT,
        6,
    )
    .map_err(|_| NicechunkGuardianError::InvalidInstruction)?;
    invoke(
        &ix,
        &[
            owner_nck_token.clone(),
            nck_mint.clone(),
            treasury_nck_token.clone(),
            owner.clone(),
            token_program.clone(),
        ],
    )
}

fn validate_token_account(
    token_account: &AccountInfo,
    mint: &Pubkey,
    owner: &Pubkey,
) -> ProgramResult {
    if token_account.owner != &spl_token::ID {
        return Err(NicechunkGuardianError::InvalidTokenAccount.into());
    }
    let data = token_account.try_borrow_data()?;
    if data.len() < TOKEN_ACCOUNT_MIN_LEN {
        return Err(NicechunkGuardianError::InvalidTokenAccount.into());
    }
    if &data[TOKEN_ACCOUNT_MINT_OFFSET..TOKEN_ACCOUNT_MINT_OFFSET + 32] != mint.as_ref()
        || &data[TOKEN_ACCOUNT_OWNER_OFFSET..TOKEN_ACCOUNT_OWNER_OFFSET + 32] != owner.as_ref()
    {
        return Err(NicechunkGuardianError::InvalidTokenAccount.into());
    }
    Ok(())
}

fn create_or_allocate_pda<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    len: usize,
    seeds: &[&[u8]],
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(len);

    if pda.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key, pda.key, lamports, len as u64, program_id,
        );
        invoke_signed(
            &create,
            &[payer.clone(), pda.clone(), system_program_account.clone()],
            &[seeds],
        )?;
        return Ok(());
    }

    if pda.lamports() < lamports {
        let transfer = system_instruction::transfer(payer.key, pda.key, lamports - pda.lamports());
        invoke(
            &transfer,
            &[payer.clone(), pda.clone(), system_program_account.clone()],
        )?;
    }

    let allocate = system_instruction::allocate(pda.key, len as u64);
    invoke_signed(
        &allocate,
        &[pda.clone(), system_program_account.clone()],
        &[seeds],
    )?;
    let assign = system_instruction::assign(pda.key, program_id);
    invoke_signed(
        &assign,
        &[pda.clone(), system_program_account.clone()],
        &[seeds],
    )?;
    Ok(())
}

fn derive_registry_pda(program_id: &Pubkey, global_config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[GUARDIAN_REGISTRY_SEED, global_config.as_ref()],
        program_id,
    )
}

fn derive_treasury_authority_pda(program_id: &Pubkey, global_config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[GUARDIAN_TREASURY_AUTHORITY_SEED, global_config.as_ref()],
        program_id,
    )
}

fn derive_region_pda(
    program_id: &Pubkey,
    global_config: &Pubkey,
    region_x: i32,
    region_y: i32,
) -> (Pubkey, u8) {
    let region_x_bytes = region_x.to_le_bytes();
    let region_y_bytes = region_y.to_le_bytes();
    Pubkey::find_program_address(
        &[
            GUARDIAN_REGION_SEED,
            global_config.as_ref(),
            &region_x_bytes,
            &region_y_bytes,
        ],
        program_id,
    )
}

struct RegisterGuardianArgs<'a> {
    region_x: i32,
    region_y: i32,
    port: u16,
    use_tls: bool,
    host: &'a [u8],
    operator: Pubkey,
}

struct UpdateGuardianEndpointArgs<'a> {
    port: u16,
    use_tls: bool,
    host: &'a [u8],
}

impl<'a> UpdateGuardianEndpointArgs<'a> {
    fn unpack(payload: &'a [u8]) -> Result<Self, NicechunkGuardianError> {
        if payload.len() < 12 {
            return Err(NicechunkGuardianError::InvalidInstruction);
        }
        let host_len = payload[11] as usize;
        let expected_len = 12_usize
            .checked_add(host_len)
            .ok_or(NicechunkGuardianError::InvalidInstruction)?;
        if payload.len() != expected_len {
            return Err(NicechunkGuardianError::InvalidInstruction);
        }
        Ok(Self {
            port: u16::from_le_bytes([payload[8], payload[9]]),
            use_tls: payload[10] == 1,
            host: &payload[12..],
        })
    }
}

impl<'a> RegisterGuardianArgs<'a> {
    fn unpack(payload: &'a [u8]) -> Result<Self, NicechunkGuardianError> {
        if payload.len() < 44 {
            return Err(NicechunkGuardianError::InvalidInstruction);
        }
        let host_len = payload[11] as usize;
        let expected_len = 12_usize
            .checked_add(host_len)
            .and_then(|len| len.checked_add(32))
            .ok_or(NicechunkGuardianError::InvalidInstruction)?;
        if payload.len() != expected_len {
            return Err(NicechunkGuardianError::InvalidInstruction);
        }
        let operator_offset = 12 + host_len;
        Ok(Self {
            region_x: read_i32(payload, 0),
            region_y: read_i32(payload, 4),
            port: u16::from_le_bytes([payload[8], payload[9]]),
            use_tls: payload[10] == 1,
            host: &payload[12..operator_offset],
            operator: Pubkey::new_from_array(
                payload[operator_offset..operator_offset + 32]
                    .try_into()
                    .map_err(|_| NicechunkGuardianError::InvalidInstruction)?,
            ),
        })
    }
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_len_is_256() {
        assert_eq!(GuardianRegion::LEN, 256);
    }

    #[test]
    fn test_registry_len_is_160() {
        assert_eq!(GuardianRegistry::LEN, 160);
    }

    #[test]
    fn test_parse_register_payload() {
        let operator = Pubkey::new_unique();
        let mut payload = Vec::new();
        payload.extend_from_slice(&2_i32.to_le_bytes());
        payload.extend_from_slice(&(-1_i32).to_le_bytes());
        payload.extend_from_slice(&8899_u16.to_le_bytes());
        payload.push(1);
        payload.push(9);
        payload.extend_from_slice(b"127.0.0.1");
        payload.extend_from_slice(operator.as_ref());

        let args = RegisterGuardianArgs::unpack(&payload).unwrap();
        assert_eq!(args.region_x, 2);
        assert_eq!(args.region_y, -1);
        assert_eq!(args.port, 8899);
        assert!(args.use_tls);
        assert_eq!(args.host, b"127.0.0.1");
        assert_eq!(args.operator, operator);
    }

    #[test]
    fn test_parse_update_endpoint_payload() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&2_i32.to_le_bytes());
        payload.extend_from_slice(&(-1_i32).to_le_bytes());
        payload.extend_from_slice(&8899_u16.to_le_bytes());
        payload.push(1);
        payload.push(9);
        payload.extend_from_slice(b"127.0.0.1");

        let args = UpdateGuardianEndpointArgs::unpack(&payload).unwrap();
        assert_eq!(args.port, 8899);
        assert!(args.use_tls);
        assert_eq!(args.host, b"127.0.0.1");
    }

    #[test]
    fn test_host_validation_rejects_scheme() {
        assert!(validate_host(b"guardian.example.com").is_ok());
        assert!(validate_host(b"https://guardian.example.com").is_err());
    }
}
