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

use cluster_config::NCK_MINT;
use errors::{require_key_eq, NicechunkError};
use state::{GlobalConfig, GLOBAL_CONFIG_SEED, NCK_DECIMALS, NCK_GENESIS_SUPPLY, TOKEN_PROGRAM_ID};

declare_id!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");

// Nicechunk Core is a minimal native Solana genesis program.
//
// The fairness model is intentionally strict:
// - no admin account is stored;
// - no pause, withdraw, or update-config instruction exists;
// - genesis values are compiled constants, not initializer-provided choices;
// - after deployment, the upgrade authority should be closed externally.
//
// Instruction `0` initializes the deterministic GlobalConfig PDA exactly once.
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data != [0] {
        return Err(NicechunkError::InvalidInstruction.into());
    }

    initialize_global_config(program_id, accounts)
}

fn initialize_global_config(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 4 {
        return Err(NicechunkError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let nck_mint = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !payer.is_signer {
        return Err(NicechunkError::InvalidPayer.into());
    }
    if !payer.is_writable {
        return Err(NicechunkError::InvalidWritableAccount.into());
    }
    if !global_config.is_writable {
        return Err(NicechunkError::InvalidWritableAccount.into());
    }

    let (expected_global_config, bump) =
        Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], program_id);
    require_key_eq(
        global_config.key,
        &expected_global_config,
        NicechunkError::InvalidGlobalConfigPda,
    )?;
    require_key_eq(nck_mint.key, &NCK_MINT, NicechunkError::InvalidNckMint)?;
    require_key_eq(
        nck_mint.owner,
        &TOKEN_PROGRAM_ID,
        NicechunkError::InvalidNckMint,
    )?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkError::InvalidSystemProgram,
    )?;

    if global_config.owner == program_id {
        return Err(NicechunkError::GlobalConfigAlreadyInitialized.into());
    }
    if global_config.owner != &system_program::ID || global_config.data_len() != 0 {
        return Err(NicechunkError::InvalidSystemAccount.into());
    }

    let mint_data = nck_mint.try_borrow_data()?;
    validate_nck_mint(&mint_data)?;
    drop(mint_data);

    create_or_allocate_global_config_pda(
        payer,
        global_config,
        system_program_account,
        program_id,
        bump,
    )?;

    if global_config.owner != program_id {
        return Err(NicechunkError::InvalidGlobalConfigOwner.into());
    }

    let mut data = global_config.try_borrow_mut_data()?;
    let clock = Clock::get()?;
    GlobalConfig::pack(&mut data, bump, clock.slot, clock.unix_timestamp)?;

    Ok(())
}

fn create_or_allocate_global_config_pda<'a>(
    payer: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    bump: u8,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(GlobalConfig::LEN);

    if global_config.lamports() == 0 {
        let create = system_instruction::create_account(
            payer.key,
            global_config.key,
            lamports,
            GlobalConfig::LEN as u64,
            program_id,
        );
        invoke_signed(
            &create,
            &[
                payer.clone(),
                global_config.clone(),
                system_program_account.clone(),
            ],
            &[&[GLOBAL_CONFIG_SEED, &[bump]]],
        )?;
        return Ok(());
    }

    if global_config.lamports() < lamports {
        let delta = lamports
            .checked_sub(global_config.lamports())
            .ok_or(NicechunkError::InvalidGlobalConfigFunding)?;
        let transfer = system_instruction::transfer(payer.key, global_config.key, delta);
        invoke(
            &transfer,
            &[
                payer.clone(),
                global_config.clone(),
                system_program_account.clone(),
            ],
        )?;
    }

    let allocate = system_instruction::allocate(global_config.key, GlobalConfig::LEN as u64);
    invoke_signed(
        &allocate,
        &[global_config.clone(), system_program_account.clone()],
        &[&[GLOBAL_CONFIG_SEED, &[bump]]],
    )?;

    let assign = system_instruction::assign(global_config.key, program_id);
    invoke_signed(
        &assign,
        &[global_config.clone(), system_program_account.clone()],
        &[&[GLOBAL_CONFIG_SEED, &[bump]]],
    )?;

    Ok(())
}

pub fn validate_nck_mint(mint_data: &[u8]) -> ProgramResult {
    // SPL Token Mint layout:
    // 0..36   mint_authority: COption<Pubkey>
    // 36..44  supply: u64
    // 44      decimals: u8
    // 45      is_initialized: bool
    // 46..82  freeze_authority: COption<Pubkey>
    if mint_data.len() < 82 {
        return Err(NicechunkError::InvalidNckMint.into());
    }
    if mint_data[44] != NCK_DECIMALS {
        return Err(NicechunkError::InvalidNckDecimals.into());
    }
    if mint_data[45] != 1 {
        return Err(NicechunkError::InvalidNckMint.into());
    }
    if read_u64_le(&mint_data[36..44]) != NCK_GENESIS_SUPPLY {
        return Err(NicechunkError::InvalidNckGenesisSupply.into());
    }
    if read_u32_le(&mint_data[0..4]) != 0 || read_u32_le(&mint_data[46..50]) != 0 {
        return Err(NicechunkError::InvalidNckAuthority.into());
    }
    Ok(())
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u64_le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster_config::{DEVELOPMENT_WALLET, NCK_MINT};
    use state::*;

    #[test]
    fn global_config_len_matches_pack() {
        let mut data = [0_u8; GlobalConfig::LEN];
        GlobalConfig::pack(&mut data, 253, 123, 456).unwrap();

        assert_eq!(&data[0..8], &CONFIG_MAGIC);
        assert_eq!(u16::from_le_bytes([data[8], data[9]]), CONFIG_VERSION);
        assert_eq!(data[10], 253);
        assert_eq!(data[11], 1);
        assert_eq!(&data[12..44], NCK_MINT.as_ref());
        assert_eq!(data[44], NCK_DECIMALS);
        assert_eq!(read_u64_le(&data[45..53]), NCK_GENESIS_SUPPLY);
        assert_eq!(&data[53..85], DEVELOPMENT_WALLET.as_ref());
        assert_eq!(u16::from_le_bytes([data[85], data[86]]), WORLD_ID);
    }

    #[test]
    fn test_global_config_len_is_293() {
        assert_eq!(GlobalConfig::LEN, 293);
    }

    #[test]
    fn test_pack_writes_exact_len() {
        let mut data = [0_u8; GlobalConfig::LEN];
        GlobalConfig::pack(&mut data, 253, 123, 456).unwrap();
        assert_ne!(&data[GlobalConfig::LEN - 8..GlobalConfig::LEN], &[0_u8; 8]);
    }

    #[test]
    fn test_pack_rejects_wrong_len() {
        let mut data = [0_u8; GlobalConfig::LEN - 1];
        assert!(GlobalConfig::pack(&mut data, 253, 123, 456).is_err());
    }

    #[test]
    fn economics_are_fixed() {
        assert_eq!(STARTER_PACK_PRICE_LAMPORTS, 100_000_000);
        assert_eq!(GENESIS_PASS_PRICE_LAMPORTS, 1_000_000_000);
        assert_eq!(GUARDIAN_STAKE_AMOUNT, 100_000_000_000);
        assert_eq!(
            SOL_TO_LIQUIDITY_BPS + SOL_TO_REWARD_BPS + SOL_TO_DEVELOPMENT_BPS,
            10_000
        );
    }

    #[test]
    fn test_validate_nck_mint_accepts_valid_mint() {
        let mint = valid_mint_data();
        assert!(validate_nck_mint(&mint).is_ok());
    }

    #[test]
    fn test_validate_nck_mint_rejects_wrong_decimals() {
        let mut mint = valid_mint_data();
        mint[44] = 9;
        assert!(validate_nck_mint(&mint).is_err());
    }

    #[test]
    fn test_validate_nck_mint_rejects_wrong_supply() {
        let mut mint = valid_mint_data();
        mint[36..44].copy_from_slice(&(NCK_GENESIS_SUPPLY - 1).to_le_bytes());
        assert!(validate_nck_mint(&mint).is_err());
    }

    #[test]
    fn test_validate_nck_mint_rejects_mint_authority() {
        let mut mint = valid_mint_data();
        mint[0..4].copy_from_slice(&1_u32.to_le_bytes());
        assert!(validate_nck_mint(&mint).is_err());
    }

    #[test]
    fn test_validate_nck_mint_rejects_freeze_authority() {
        let mut mint = valid_mint_data();
        mint[46..50].copy_from_slice(&1_u32.to_le_bytes());
        assert!(validate_nck_mint(&mint).is_err());
    }

    fn valid_mint_data() -> [u8; 82] {
        let mut data = [0_u8; 82];
        data[36..44].copy_from_slice(&NCK_GENESIS_SUPPLY.to_le_bytes());
        data[44] = NCK_DECIMALS;
        data[45] = 1;
        data
    }
}
