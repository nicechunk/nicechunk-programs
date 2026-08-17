use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};
use spl_token::state::Account as TokenAccount;

use crate::{
    cluster_config::{MARKET_TREASURY, NCK_MINT},
    errors::{require_key_eq, NicechunkMarketError},
    NCK_DECIMALS,
};

pub const TREASURY_SWAP_STATE_SEED: &[u8] = b"treasury-swap-v1";
pub const TREASURY_SWAP_AUTHORITY_SEED: &[u8] = b"treasury-swap-authority-v1";
pub const TREASURY_SWAP_SOL_VAULT_SEED: &[u8] = b"treasury-swap-sol-v1";
pub const TREASURY_SWAP_NCK_VAULT_SEED: &[u8] = b"treasury-swap-nck-v1";

pub const TREASURY_SWAP_MAGIC: [u8; 8] = *b"NCKSWP01";
pub const TREASURY_SWAP_VERSION: u16 = 1;
pub const TREASURY_SWAP_STATE_LEN: usize = 160;
pub const MAX_SWAP_FEE_BPS: u16 = 1_000;

const BPS_DENOMINATOR: u128 = 10_000;
const NCK_BASE_UNITS: u128 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreasurySwapState {
    pub state_bump: u8,
    pub authority_bump: u8,
    pub sol_vault_bump: u8,
    pub nck_vault_bump: u8,
    pub paused: bool,
    pub fee_bps: u16,
    pub admin: Pubkey,
    pub nck_mint: Pubkey,
    pub lamports_per_nck: u64,
    pub minimum_nck_units: u64,
    pub maximum_nck_units: u64,
    pub revision: u64,
    pub updated_slot: u64,
    pub total_sol_to_nck_lamports: u64,
    pub total_sol_to_nck_units: u64,
    pub total_nck_to_sol_units: u64,
    pub total_nck_to_sol_lamports: u64,
}

impl TreasurySwapState {
    const FEE_BPS_OFFSET: usize = 16;
    const ADMIN_OFFSET: usize = 24;
    const NCK_MINT_OFFSET: usize = 56;
    const LAMPORTS_PER_NCK_OFFSET: usize = 88;
    const MINIMUM_NCK_UNITS_OFFSET: usize = 96;
    const MAXIMUM_NCK_UNITS_OFFSET: usize = 104;
    const REVISION_OFFSET: usize = 112;
    const UPDATED_SLOT_OFFSET: usize = 120;
    const TOTAL_SOL_TO_NCK_LAMPORTS_OFFSET: usize = 128;
    const TOTAL_SOL_TO_NCK_UNITS_OFFSET: usize = 136;
    const TOTAL_NCK_TO_SOL_UNITS_OFFSET: usize = 144;
    const TOTAL_NCK_TO_SOL_LAMPORTS_OFFSET: usize = 152;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != TREASURY_SWAP_STATE_LEN
            || data[0..8] != TREASURY_SWAP_MAGIC
            || read_u16(data, 8) != TREASURY_SWAP_VERSION
            || data[15] != 0
            || data[18..24].iter().any(|byte| *byte != 0)
        {
            return Err(NicechunkMarketError::InvalidSwapState);
        }
        let paused = match data[14] {
            0 => false,
            1 => true,
            _ => return Err(NicechunkMarketError::InvalidSwapState),
        };
        let state = Self {
            state_bump: data[10],
            authority_bump: data[11],
            sol_vault_bump: data[12],
            nck_vault_bump: data[13],
            paused,
            fee_bps: read_u16(data, Self::FEE_BPS_OFFSET),
            admin: read_pubkey(data, Self::ADMIN_OFFSET),
            nck_mint: read_pubkey(data, Self::NCK_MINT_OFFSET),
            lamports_per_nck: read_u64(data, Self::LAMPORTS_PER_NCK_OFFSET),
            minimum_nck_units: read_u64(data, Self::MINIMUM_NCK_UNITS_OFFSET),
            maximum_nck_units: read_u64(data, Self::MAXIMUM_NCK_UNITS_OFFSET),
            revision: read_u64(data, Self::REVISION_OFFSET),
            updated_slot: read_u64(data, Self::UPDATED_SLOT_OFFSET),
            total_sol_to_nck_lamports: read_u64(data, Self::TOTAL_SOL_TO_NCK_LAMPORTS_OFFSET),
            total_sol_to_nck_units: read_u64(data, Self::TOTAL_SOL_TO_NCK_UNITS_OFFSET),
            total_nck_to_sol_units: read_u64(data, Self::TOTAL_NCK_TO_SOL_UNITS_OFFSET),
            total_nck_to_sol_lamports: read_u64(data, Self::TOTAL_NCK_TO_SOL_LAMPORTS_OFFSET),
        };
        validate_config(
            state.lamports_per_nck,
            state.minimum_nck_units,
            state.maximum_nck_units,
            state.fee_bps,
        )?;
        if state.admin != MARKET_TREASURY || state.nck_mint != NCK_MINT || state.revision == 0 {
            return Err(NicechunkMarketError::InvalidSwapState);
        }
        Ok(state)
    }

    pub fn pack(&self, data: &mut [u8]) -> ProgramResult {
        if data.len() != TREASURY_SWAP_STATE_LEN {
            return Err(NicechunkMarketError::InvalidSwapState.into());
        }
        validate_config(
            self.lamports_per_nck,
            self.minimum_nck_units,
            self.maximum_nck_units,
            self.fee_bps,
        )?;
        if self.admin != MARKET_TREASURY || self.nck_mint != NCK_MINT || self.revision == 0 {
            return Err(NicechunkMarketError::InvalidSwapState.into());
        }
        data.fill(0);
        data[0..8].copy_from_slice(&TREASURY_SWAP_MAGIC);
        data[8..10].copy_from_slice(&TREASURY_SWAP_VERSION.to_le_bytes());
        data[10] = self.state_bump;
        data[11] = self.authority_bump;
        data[12] = self.sol_vault_bump;
        data[13] = self.nck_vault_bump;
        data[14] = u8::from(self.paused);
        data[Self::FEE_BPS_OFFSET..Self::FEE_BPS_OFFSET + 2]
            .copy_from_slice(&self.fee_bps.to_le_bytes());
        data[Self::ADMIN_OFFSET..Self::ADMIN_OFFSET + 32].copy_from_slice(self.admin.as_ref());
        data[Self::NCK_MINT_OFFSET..Self::NCK_MINT_OFFSET + 32]
            .copy_from_slice(self.nck_mint.as_ref());
        write_u64(data, Self::LAMPORTS_PER_NCK_OFFSET, self.lamports_per_nck);
        write_u64(data, Self::MINIMUM_NCK_UNITS_OFFSET, self.minimum_nck_units);
        write_u64(data, Self::MAXIMUM_NCK_UNITS_OFFSET, self.maximum_nck_units);
        write_u64(data, Self::REVISION_OFFSET, self.revision);
        write_u64(data, Self::UPDATED_SLOT_OFFSET, self.updated_slot);
        write_u64(
            data,
            Self::TOTAL_SOL_TO_NCK_LAMPORTS_OFFSET,
            self.total_sol_to_nck_lamports,
        );
        write_u64(
            data,
            Self::TOTAL_SOL_TO_NCK_UNITS_OFFSET,
            self.total_sol_to_nck_units,
        );
        write_u64(
            data,
            Self::TOTAL_NCK_TO_SOL_UNITS_OFFSET,
            self.total_nck_to_sol_units,
        );
        write_u64(
            data,
            Self::TOTAL_NCK_TO_SOL_LAMPORTS_OFFSET,
            self.total_nck_to_sol_lamports,
        );
        Ok(())
    }

    fn record_sol_to_nck(
        &mut self,
        lamports_in: u64,
        nck_units_out: u64,
        slot: u64,
    ) -> ProgramResult {
        self.total_sol_to_nck_lamports = self
            .total_sol_to_nck_lamports
            .checked_add(lamports_in)
            .ok_or(NicechunkMarketError::SwapStateOverflow)?;
        self.total_sol_to_nck_units = self
            .total_sol_to_nck_units
            .checked_add(nck_units_out)
            .ok_or(NicechunkMarketError::SwapStateOverflow)?;
        self.updated_slot = slot;
        Ok(())
    }

    fn record_nck_to_sol(
        &mut self,
        nck_units_in: u64,
        lamports_out: u64,
        slot: u64,
    ) -> ProgramResult {
        self.total_nck_to_sol_units = self
            .total_nck_to_sol_units
            .checked_add(nck_units_in)
            .ok_or(NicechunkMarketError::SwapStateOverflow)?;
        self.total_nck_to_sol_lamports = self
            .total_nck_to_sol_lamports
            .checked_add(lamports_out)
            .ok_or(NicechunkMarketError::SwapStateOverflow)?;
        self.updated_slot = slot;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InitializeArgs {
    lamports_per_nck: u64,
    minimum_nck_units: u64,
    maximum_nck_units: u64,
    fee_bps: u16,
}

impl InitializeArgs {
    const LEN: usize = 26;

    fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let args = Self {
            lamports_per_nck: read_u64(data, 0),
            minimum_nck_units: read_u64(data, 8),
            maximum_nck_units: read_u64(data, 16),
            fee_bps: read_u16(data, 24),
        };
        validate_config(
            args.lamports_per_nck,
            args.minimum_nck_units,
            args.maximum_nck_units,
            args.fee_bps,
        )?;
        Ok(args)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConfigureArgs {
    config: InitializeArgs,
    paused: bool,
}

impl ConfigureArgs {
    const LEN: usize = 27;

    fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN || data[26] > 1 {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        Ok(Self {
            config: InitializeArgs::unpack(&data[..InitializeArgs::LEN])?,
            paused: data[26] == 1,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AmountArgs {
    amount: u64,
}

impl AmountArgs {
    const LEN: usize = 8;

    fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let amount = read_u64(data, 0);
        if amount == 0 {
            return Err(NicechunkMarketError::InvalidSwapAmount);
        }
        Ok(Self { amount })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SwapArgs {
    amount_in: u64,
    minimum_amount_out: u64,
    expected_revision: u64,
    deadline_slot: u64,
}

impl SwapArgs {
    const LEN: usize = 32;

    fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let args = Self {
            amount_in: read_u64(data, 0),
            minimum_amount_out: read_u64(data, 8),
            expected_revision: read_u64(data, 16),
            deadline_slot: read_u64(data, 24),
        };
        if args.amount_in == 0 || args.minimum_amount_out == 0 || args.expected_revision == 0 {
            return Err(NicechunkMarketError::InvalidSwapAmount);
        }
        Ok(args)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapQuote {
    pub amount_out: u64,
    pub fee_amount: u64,
}

pub fn quote_sol_to_nck(
    lamports_in: u64,
    state: &TreasurySwapState,
) -> Result<SwapQuote, NicechunkMarketError> {
    if lamports_in == 0 {
        return Err(NicechunkMarketError::InvalidSwapAmount);
    }
    let gross_nck_units = (u128::from(lamports_in))
        .checked_mul(NCK_BASE_UNITS)
        .ok_or(NicechunkMarketError::SwapMathOverflow)?
        .checked_div(u128::from(state.lamports_per_nck))
        .ok_or(NicechunkMarketError::SwapMathOverflow)?;
    enforce_nck_limits(gross_nck_units, state)?;
    apply_output_fee(gross_nck_units, state.fee_bps)
}

pub fn quote_nck_to_sol(
    nck_units_in: u64,
    state: &TreasurySwapState,
) -> Result<SwapQuote, NicechunkMarketError> {
    if nck_units_in == 0 {
        return Err(NicechunkMarketError::InvalidSwapAmount);
    }
    enforce_nck_limits(u128::from(nck_units_in), state)?;
    let gross_lamports = u128::from(nck_units_in)
        .checked_mul(u128::from(state.lamports_per_nck))
        .ok_or(NicechunkMarketError::SwapMathOverflow)?
        .checked_div(NCK_BASE_UNITS)
        .ok_or(NicechunkMarketError::SwapMathOverflow)?;
    apply_output_fee(gross_lamports, state.fee_bps)
}

fn apply_output_fee(gross_amount: u128, fee_bps: u16) -> Result<SwapQuote, NicechunkMarketError> {
    let net_amount = gross_amount
        .checked_mul(
            BPS_DENOMINATOR
                .checked_sub(u128::from(fee_bps))
                .ok_or(NicechunkMarketError::SwapMathOverflow)?,
        )
        .ok_or(NicechunkMarketError::SwapMathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(NicechunkMarketError::SwapMathOverflow)?;
    if net_amount == 0 || net_amount > u128::from(u64::MAX) || gross_amount > u128::from(u64::MAX) {
        return Err(NicechunkMarketError::InvalidSwapAmount);
    }
    Ok(SwapQuote {
        amount_out: net_amount as u64,
        fee_amount: gross_amount
            .checked_sub(net_amount)
            .ok_or(NicechunkMarketError::SwapMathOverflow)? as u64,
    })
}

fn enforce_nck_limits(
    nck_units: u128,
    state: &TreasurySwapState,
) -> Result<(), NicechunkMarketError> {
    if nck_units < u128::from(state.minimum_nck_units)
        || nck_units > u128::from(state.maximum_nck_units)
    {
        return Err(NicechunkMarketError::SwapAmountOutsideLimits);
    }
    Ok(())
}

fn validate_config(
    lamports_per_nck: u64,
    minimum_nck_units: u64,
    maximum_nck_units: u64,
    fee_bps: u16,
) -> Result<(), NicechunkMarketError> {
    if lamports_per_nck == 0
        || minimum_nck_units == 0
        || maximum_nck_units < minimum_nck_units
        || fee_bps > MAX_SWAP_FEE_BPS
    {
        return Err(NicechunkMarketError::InvalidSwapConfig);
    }
    Ok(())
}

pub fn initialize_treasury_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = InitializeArgs::unpack(payload)?;
    if accounts.len() != 8 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let admin = &accounts[0];
    let state_account = &accounts[1];
    let sol_vault = &accounts[2];
    let nck_vault = &accounts[3];
    let authority = &accounts[4];
    let nck_mint = &accounts[5];
    let system_program_account = &accounts[6];
    let token_program = &accounts[7];

    validate_admin(admin)?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    require_key_eq(
        token_program.key,
        &spl_token::ID,
        NicechunkMarketError::InvalidTokenProgram,
    )?;
    validate_nck_mint(nck_mint)?;

    let (expected_state, state_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_STATE_SEED], program_id);
    let (expected_authority, authority_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_AUTHORITY_SEED], program_id);
    let (expected_sol_vault, sol_vault_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_SOL_VAULT_SEED], program_id);
    let (expected_nck_vault, nck_vault_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_NCK_VAULT_SEED], program_id);
    require_key_eq(
        state_account.key,
        &expected_state,
        NicechunkMarketError::InvalidSwapState,
    )?;
    require_key_eq(
        authority.key,
        &expected_authority,
        NicechunkMarketError::InvalidSwapAuthority,
    )?;
    require_key_eq(
        sol_vault.key,
        &expected_sol_vault,
        NicechunkMarketError::InvalidSwapSolVault,
    )?;
    require_key_eq(
        nck_vault.key,
        &expected_nck_vault,
        NicechunkMarketError::InvalidSwapNckVault,
    )?;
    for account in [state_account, sol_vault, nck_vault] {
        if account.owner != &system_program::ID || account.data_len() != 0 {
            return Err(NicechunkMarketError::SwapAlreadyInitialized.into());
        }
    }
    if !state_account.is_writable || !sol_vault.is_writable || !nck_vault.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }

    let rent = Rent::get()?;
    create_pda_account(
        admin,
        state_account,
        system_program_account,
        program_id,
        TREASURY_SWAP_STATE_LEN,
        rent.minimum_balance(TREASURY_SWAP_STATE_LEN),
        &[TREASURY_SWAP_STATE_SEED, &[state_bump]],
    )?;
    create_pda_account(
        admin,
        sol_vault,
        system_program_account,
        program_id,
        0,
        rent.minimum_balance(0),
        &[TREASURY_SWAP_SOL_VAULT_SEED, &[sol_vault_bump]],
    )?;
    create_pda_account(
        admin,
        nck_vault,
        system_program_account,
        &spl_token::ID,
        TokenAccount::LEN,
        rent.minimum_balance(TokenAccount::LEN),
        &[TREASURY_SWAP_NCK_VAULT_SEED, &[nck_vault_bump]],
    )?;
    let initialize_vault = spl_token::instruction::initialize_account3(
        token_program.key,
        nck_vault.key,
        nck_mint.key,
        authority.key,
    )
    .map_err(|_| NicechunkMarketError::InvalidSwapNckVault)?;
    invoke(
        &initialize_vault,
        &[nck_vault.clone(), nck_mint.clone(), token_program.clone()],
    )?;

    TreasurySwapState {
        state_bump,
        authority_bump,
        sol_vault_bump,
        nck_vault_bump,
        paused: true,
        fee_bps: args.fee_bps,
        admin: MARKET_TREASURY,
        nck_mint: NCK_MINT,
        lamports_per_nck: args.lamports_per_nck,
        minimum_nck_units: args.minimum_nck_units,
        maximum_nck_units: args.maximum_nck_units,
        revision: 1,
        updated_slot: solana_program::clock::Clock::get()?.slot,
        total_sol_to_nck_lamports: 0,
        total_sol_to_nck_units: 0,
        total_nck_to_sol_units: 0,
        total_nck_to_sol_lamports: 0,
    }
    .pack(&mut state_account.try_borrow_mut_data()?)
}

pub fn configure_treasury_swap(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = ConfigureArgs::unpack(payload)?;
    let expected_account_count = if args.paused { 2 } else { 4 };
    if accounts.len() != expected_account_count {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let admin = &accounts[0];
    let state_account = &accounts[1];
    validate_admin(admin)?;
    if !state_account.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    let mut state = load_state(program_id, state_account)?;
    state.lamports_per_nck = args.config.lamports_per_nck;
    state.minimum_nck_units = args.config.minimum_nck_units;
    state.maximum_nck_units = args.config.maximum_nck_units;
    state.fee_bps = args.config.fee_bps;
    state.paused = args.paused;
    if !state.paused {
        let sol_vault = &accounts[2];
        let nck_vault = &accounts[3];
        validate_sol_vault(program_id, sol_vault)?;
        validate_nck_vault(program_id, nck_vault, state.authority_bump)?;
        validate_activation_liquidity(&state, sol_vault, nck_vault)?;
    }
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(NicechunkMarketError::SwapStateOverflow)?;
    state.updated_slot = solana_program::clock::Clock::get()?.slot;
    state.pack(&mut state_account.try_borrow_mut_data()?)
}

pub fn deposit_treasury_swap_sol(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = AmountArgs::unpack(payload)?;
    if accounts.len() != 4 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let admin = &accounts[0];
    let state_account = &accounts[1];
    let sol_vault = &accounts[2];
    let system_program_account = &accounts[3];
    validate_admin(admin)?;
    load_state(program_id, state_account)?;
    validate_sol_vault(program_id, sol_vault)?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    let transfer = system_instruction::transfer(admin.key, sol_vault.key, args.amount);
    invoke(
        &transfer,
        &[
            admin.clone(),
            sol_vault.clone(),
            system_program_account.clone(),
        ],
    )
}

pub fn withdraw_treasury_swap_sol(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = AmountArgs::unpack(payload)?;
    if accounts.len() != 3 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let admin = &accounts[0];
    let state_account = &accounts[1];
    let sol_vault = &accounts[2];
    validate_admin(admin)?;
    let state = load_state(program_id, state_account)?;
    require_swap_paused(&state)?;
    validate_sol_vault(program_id, sol_vault)?;
    transfer_sol_from_vault(sol_vault, admin, args.amount)
}

pub fn deposit_treasury_swap_nck(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = AmountArgs::unpack(payload)?;
    if accounts.len() != 6 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let admin = &accounts[0];
    let state_account = &accounts[1];
    let admin_nck_token = &accounts[2];
    let nck_vault = &accounts[3];
    let nck_mint = &accounts[4];
    let token_program = &accounts[5];
    validate_admin(admin)?;
    let state = load_state(program_id, state_account)?;
    validate_nck_mint_and_program(nck_mint, token_program)?;
    validate_token_account(admin_nck_token, &NCK_MINT, &MARKET_TREASURY)?;
    validate_nck_vault(program_id, nck_vault, state.authority_bump)?;
    transfer_nck(
        admin_nck_token,
        nck_vault,
        nck_mint,
        admin,
        token_program,
        args.amount,
        None,
    )
}

pub fn withdraw_treasury_swap_nck(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = AmountArgs::unpack(payload)?;
    if accounts.len() != 7 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let admin = &accounts[0];
    let state_account = &accounts[1];
    let authority = &accounts[2];
    let admin_nck_token = &accounts[3];
    let nck_vault = &accounts[4];
    let nck_mint = &accounts[5];
    let token_program = &accounts[6];
    validate_admin(admin)?;
    let state = load_state(program_id, state_account)?;
    require_swap_paused(&state)?;
    validate_authority(program_id, authority, state.authority_bump)?;
    validate_nck_mint_and_program(nck_mint, token_program)?;
    validate_token_account(admin_nck_token, &NCK_MINT, &MARKET_TREASURY)?;
    validate_nck_vault(program_id, nck_vault, state.authority_bump)?;
    transfer_nck(
        nck_vault,
        admin_nck_token,
        nck_mint,
        authority,
        token_program,
        args.amount,
        Some(&[TREASURY_SWAP_AUTHORITY_SEED, &[state.authority_bump]]),
    )
}

pub fn swap_sol_for_nck(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = SwapArgs::unpack(payload)?;
    if accounts.len() != 9 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let user = &accounts[0];
    let state_account = &accounts[1];
    let sol_vault = &accounts[2];
    let authority = &accounts[3];
    let nck_vault = &accounts[4];
    let user_nck_token = &accounts[5];
    let nck_mint = &accounts[6];
    let system_program_account = &accounts[7];
    let token_program = &accounts[8];
    validate_user(user)?;
    if !state_account.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    let mut state = load_state(program_id, state_account)?;
    validate_swap_request(&state, &args)?;
    validate_sol_vault(program_id, sol_vault)?;
    validate_authority(program_id, authority, state.authority_bump)?;
    validate_nck_mint_and_program(nck_mint, token_program)?;
    validate_token_account(user_nck_token, &NCK_MINT, user.key)?;
    validate_nck_vault(program_id, nck_vault, state.authority_bump)?;
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    let quote = quote_sol_to_nck(args.amount_in, &state)?;
    enforce_minimum_amount_out(quote.amount_out, args.minimum_amount_out)?;
    if token_balance(nck_vault)? < quote.amount_out {
        return Err(NicechunkMarketError::InsufficientSwapLiquidity.into());
    }

    let transfer_sol = system_instruction::transfer(user.key, sol_vault.key, args.amount_in);
    invoke(
        &transfer_sol,
        &[
            user.clone(),
            sol_vault.clone(),
            system_program_account.clone(),
        ],
    )?;
    transfer_nck(
        nck_vault,
        user_nck_token,
        nck_mint,
        authority,
        token_program,
        quote.amount_out,
        Some(&[TREASURY_SWAP_AUTHORITY_SEED, &[state.authority_bump]]),
    )?;
    state.record_sol_to_nck(
        args.amount_in,
        quote.amount_out,
        solana_program::clock::Clock::get()?.slot,
    )?;
    state.pack(&mut state_account.try_borrow_mut_data()?)
}

pub fn swap_nck_for_sol(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = SwapArgs::unpack(payload)?;
    if accounts.len() != 7 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let user = &accounts[0];
    let state_account = &accounts[1];
    let sol_vault = &accounts[2];
    let nck_vault = &accounts[3];
    let user_nck_token = &accounts[4];
    let nck_mint = &accounts[5];
    let token_program = &accounts[6];
    validate_user(user)?;
    if !state_account.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    let mut state = load_state(program_id, state_account)?;
    validate_swap_request(&state, &args)?;
    validate_sol_vault(program_id, sol_vault)?;
    validate_nck_mint_and_program(nck_mint, token_program)?;
    validate_token_account(user_nck_token, &NCK_MINT, user.key)?;
    validate_nck_vault(program_id, nck_vault, state.authority_bump)?;
    let quote = quote_nck_to_sol(args.amount_in, &state)?;
    enforce_minimum_amount_out(quote.amount_out, args.minimum_amount_out)?;
    let available_sol = available_sol_liquidity(sol_vault)?;
    if available_sol < quote.amount_out {
        return Err(NicechunkMarketError::InsufficientSwapLiquidity.into());
    }

    transfer_nck(
        user_nck_token,
        nck_vault,
        nck_mint,
        user,
        token_program,
        args.amount_in,
        None,
    )?;
    transfer_sol_from_vault(sol_vault, user, quote.amount_out)?;
    state.record_nck_to_sol(
        args.amount_in,
        quote.amount_out,
        solana_program::clock::Clock::get()?.slot,
    )?;
    state.pack(&mut state_account.try_borrow_mut_data()?)
}

fn validate_swap_request(state: &TreasurySwapState, args: &SwapArgs) -> ProgramResult {
    if state.paused {
        return Err(NicechunkMarketError::SwapPaused.into());
    }
    if state.revision != args.expected_revision {
        return Err(NicechunkMarketError::SwapConfigRevisionMismatch.into());
    }
    if solana_program::clock::Clock::get()?.slot > args.deadline_slot {
        return Err(NicechunkMarketError::SwapDeadlineExpired.into());
    }
    Ok(())
}

fn enforce_minimum_amount_out(amount_out: u64, minimum_amount_out: u64) -> ProgramResult {
    if amount_out < minimum_amount_out {
        return Err(NicechunkMarketError::SwapAmountOutTooLow.into());
    }
    Ok(())
}

fn require_swap_paused(state: &TreasurySwapState) -> ProgramResult {
    if !state.paused {
        return Err(NicechunkMarketError::SwapMustBePaused.into());
    }
    Ok(())
}

fn validate_activation_liquidity(
    state: &TreasurySwapState,
    sol_vault: &AccountInfo,
    nck_vault: &AccountInfo,
) -> ProgramResult {
    let (required_sol, required_nck) = required_activation_liquidity(state)?;
    if available_sol_liquidity(sol_vault)? < required_sol
        || token_balance(nck_vault)? < required_nck
    {
        return Err(NicechunkMarketError::InsufficientSwapLiquidity.into());
    }
    Ok(())
}

fn required_activation_liquidity(
    state: &TreasurySwapState,
) -> Result<(u64, u64), NicechunkMarketError> {
    let required_sol = quote_nck_to_sol(state.maximum_nck_units, state)?.amount_out;
    let required_nck =
        apply_output_fee(u128::from(state.maximum_nck_units), state.fee_bps)?.amount_out;
    Ok((required_sol, required_nck))
}

fn available_sol_liquidity(sol_vault: &AccountInfo) -> Result<u64, ProgramError> {
    Ok(sol_vault
        .lamports()
        .saturating_sub(Rent::get()?.minimum_balance(0)))
}

fn validate_admin(admin: &AccountInfo) -> ProgramResult {
    if !admin.is_signer || !admin.is_writable || admin.key != &MARKET_TREASURY {
        return Err(NicechunkMarketError::UnauthorizedSwapAdmin.into());
    }
    Ok(())
}

fn validate_user(user: &AccountInfo) -> ProgramResult {
    if !user.is_signer || !user.is_writable {
        return Err(NicechunkMarketError::InvalidBuyer.into());
    }
    Ok(())
}

fn validate_nck_mint(nck_mint: &AccountInfo) -> ProgramResult {
    require_key_eq(
        nck_mint.key,
        &NCK_MINT,
        NicechunkMarketError::InvalidNckMint,
    )?;
    require_key_eq(
        nck_mint.owner,
        &spl_token::ID,
        NicechunkMarketError::InvalidNckMint,
    )
}

fn validate_nck_mint_and_program(
    nck_mint: &AccountInfo,
    token_program: &AccountInfo,
) -> ProgramResult {
    require_key_eq(
        token_program.key,
        &spl_token::ID,
        NicechunkMarketError::InvalidTokenProgram,
    )?;
    validate_nck_mint(nck_mint)
}

fn load_state(
    program_id: &Pubkey,
    state_account: &AccountInfo,
) -> Result<TreasurySwapState, ProgramError> {
    if state_account.owner != program_id {
        return Err(NicechunkMarketError::InvalidSwapState.into());
    }
    let state = TreasurySwapState::unpack(&state_account.try_borrow_data()?)?;
    let (expected, bump) = Pubkey::find_program_address(&[TREASURY_SWAP_STATE_SEED], program_id);
    if state_account.key != &expected || state.state_bump != bump {
        return Err(NicechunkMarketError::InvalidSwapState.into());
    }
    let (_, authority_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_AUTHORITY_SEED], program_id);
    let (_, sol_vault_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_SOL_VAULT_SEED], program_id);
    let (_, nck_vault_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_NCK_VAULT_SEED], program_id);
    if state.authority_bump != authority_bump
        || state.sol_vault_bump != sol_vault_bump
        || state.nck_vault_bump != nck_vault_bump
    {
        return Err(NicechunkMarketError::InvalidSwapState.into());
    }
    Ok(state)
}

fn validate_authority(
    program_id: &Pubkey,
    authority: &AccountInfo,
    expected_bump: u8,
) -> ProgramResult {
    let (expected, bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_AUTHORITY_SEED], program_id);
    if authority.key != &expected || bump != expected_bump {
        return Err(NicechunkMarketError::InvalidSwapAuthority.into());
    }
    Ok(())
}

fn validate_sol_vault(program_id: &Pubkey, sol_vault: &AccountInfo) -> ProgramResult {
    let (expected, _) = Pubkey::find_program_address(&[TREASURY_SWAP_SOL_VAULT_SEED], program_id);
    if sol_vault.key != &expected
        || sol_vault.owner != program_id
        || sol_vault.data_len() != 0
        || !sol_vault.is_writable
    {
        return Err(NicechunkMarketError::InvalidSwapSolVault.into());
    }
    Ok(())
}

fn validate_nck_vault(
    program_id: &Pubkey,
    nck_vault: &AccountInfo,
    authority_bump: u8,
) -> ProgramResult {
    let (expected_vault, _) =
        Pubkey::find_program_address(&[TREASURY_SWAP_NCK_VAULT_SEED], program_id);
    let (expected_authority, derived_bump) =
        Pubkey::find_program_address(&[TREASURY_SWAP_AUTHORITY_SEED], program_id);
    if nck_vault.key != &expected_vault || !nck_vault.is_writable || derived_bump != authority_bump
    {
        return Err(NicechunkMarketError::InvalidSwapNckVault.into());
    }
    validate_token_account(nck_vault, &NCK_MINT, &expected_authority)
        .map_err(|_| NicechunkMarketError::InvalidSwapNckVault.into())
}

fn validate_token_account(
    token_account: &AccountInfo,
    mint: &Pubkey,
    owner: &Pubkey,
) -> ProgramResult {
    if token_account.owner != &spl_token::ID {
        return Err(NicechunkMarketError::InvalidTokenAccount.into());
    }
    let account = TokenAccount::unpack(&token_account.try_borrow_data()?)
        .map_err(|_| NicechunkMarketError::InvalidTokenAccount)?;
    if &account.mint != mint || &account.owner != owner {
        return Err(NicechunkMarketError::InvalidTokenAccount.into());
    }
    Ok(())
}

fn token_balance(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    Ok(TokenAccount::unpack(&token_account.try_borrow_data()?)
        .map_err(|_| NicechunkMarketError::InvalidTokenAccount)?
        .amount)
}

fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    owner: &Pubkey,
    space: usize,
    lamports: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    let create =
        system_instruction::create_account(payer.key, account.key, lamports, space as u64, owner);
    invoke_signed(
        &create,
        &[
            payer.clone(),
            account.clone(),
            system_program_account.clone(),
        ],
        &[signer_seeds],
    )
}

fn transfer_nck<'a>(
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: Option<&[&[u8]]>,
) -> ProgramResult {
    let transfer = spl_token::instruction::transfer_checked(
        token_program.key,
        source.key,
        mint.key,
        destination.key,
        authority.key,
        &[],
        amount,
        NCK_DECIMALS,
    )
    .map_err(|_| NicechunkMarketError::InvalidInstruction)?;
    let account_infos = [
        source.clone(),
        mint.clone(),
        destination.clone(),
        authority.clone(),
        token_program.clone(),
    ];
    if let Some(seeds) = signer_seeds {
        invoke_signed(&transfer, &account_infos, &[seeds])
    } else {
        invoke(&transfer, &account_infos)
    }
}

fn transfer_sol_from_vault(
    sol_vault: &AccountInfo,
    destination: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    let vault_balance = sol_vault.lamports();
    let destination_balance = destination.lamports();
    let remaining = vault_balance
        .checked_sub(amount)
        .ok_or(NicechunkMarketError::InsufficientSwapLiquidity)?;
    if remaining < Rent::get()?.minimum_balance(0) {
        return Err(NicechunkMarketError::InsufficientSwapLiquidity.into());
    }
    let destination_after = destination_balance
        .checked_add(amount)
        .ok_or(NicechunkMarketError::SwapMathOverflow)?;
    **sol_vault.try_borrow_mut_lamports()? = remaining;
    **destination.try_borrow_mut_lamports()? = destination_after;
    Ok(())
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().expect("u64 slice"))
}

fn write_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_pubkey(data: &[u8], offset: usize) -> Pubkey {
    Pubkey::new_from_array(data[offset..offset + 32].try_into().expect("pubkey slice"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(rate: u64, fee_bps: u16) -> TreasurySwapState {
        TreasurySwapState {
            state_bump: 1,
            authority_bump: 2,
            sol_vault_bump: 3,
            nck_vault_bump: 4,
            paused: false,
            fee_bps,
            admin: MARKET_TREASURY,
            nck_mint: NCK_MINT,
            lamports_per_nck: rate,
            minimum_nck_units: 1,
            maximum_nck_units: u64::MAX,
            revision: 1,
            updated_slot: 10,
            total_sol_to_nck_lamports: 0,
            total_sol_to_nck_units: 0,
            total_nck_to_sol_units: 0,
            total_nck_to_sol_lamports: 0,
        }
    }

    #[test]
    fn state_round_trip_preserves_every_field() {
        let expected = state(25_000_000, 30);
        let mut data = [0_u8; TREASURY_SWAP_STATE_LEN];
        expected.pack(&mut data).unwrap();
        assert_eq!(TreasurySwapState::unpack(&data).unwrap(), expected);
        assert_eq!(&data[0..8], &TREASURY_SWAP_MAGIC);
        assert_eq!(data.len(), TREASURY_SWAP_STATE_LEN);
        assert_eq!(data[15], 0);
        assert!(data[18..24].iter().all(|byte| *byte == 0));

        for offset in [15, 18, 23] {
            let mut forged = data;
            forged[offset] = 1;
            assert!(matches!(
                TreasurySwapState::unpack(&forged),
                Err(NicechunkMarketError::InvalidSwapState)
            ));
        }
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        assert!(matches!(
            validate_config(0, 1, 1, 0),
            Err(NicechunkMarketError::InvalidSwapConfig)
        ));
        assert!(matches!(
            validate_config(1, 2, 1, 0),
            Err(NicechunkMarketError::InvalidSwapConfig)
        ));
        assert!(matches!(
            validate_config(1, 1, 1, MAX_SWAP_FEE_BPS + 1),
            Err(NicechunkMarketError::InvalidSwapConfig)
        ));
    }

    #[test]
    fn fixed_price_quotes_are_symmetric_before_rounding() {
        let state = state(25_000_000, 0);
        assert_eq!(
            quote_sol_to_nck(100_000_000, &state).unwrap(),
            SwapQuote {
                amount_out: 4_000_000,
                fee_amount: 0,
            }
        );
        assert_eq!(
            quote_nck_to_sol(4_000_000, &state).unwrap(),
            SwapQuote {
                amount_out: 100_000_000,
                fee_amount: 0,
            }
        );
    }

    #[test]
    fn output_fee_is_conservative_in_both_directions() {
        let state = state(25_000_000, 100);
        assert_eq!(
            quote_sol_to_nck(100_000_000, &state).unwrap().amount_out,
            3_960_000
        );
        assert_eq!(
            quote_sol_to_nck(100_000_000, &state).unwrap().fee_amount,
            40_000
        );
        assert_eq!(
            quote_nck_to_sol(4_000_000, &state).unwrap().amount_out,
            99_000_000
        );
        assert_eq!(
            quote_nck_to_sol(4_000_000, &state).unwrap().fee_amount,
            1_000_000
        );
    }

    #[test]
    fn floor_rounding_never_creates_round_trip_profit() {
        let state = state(33_333_333, 0);
        for lamports in [34_u64, 100, 10_001, 1_000_003, 999_999_937] {
            let nck = quote_sol_to_nck(lamports, &state).unwrap().amount_out;
            let returned = quote_nck_to_sol(nck, &state).unwrap().amount_out;
            assert!(returned <= lamports, "{returned} exceeds {lamports}");
        }
        for nck_units in [2_u64, 100, 10_001, 1_000_003, 999_999_937] {
            let lamports = quote_nck_to_sol(nck_units, &state).unwrap().amount_out;
            let returned = quote_sol_to_nck(lamports, &state).unwrap().amount_out;
            assert!(returned <= nck_units, "{returned} exceeds {nck_units}");
        }
    }

    #[test]
    fn configured_nck_trade_limits_apply_to_both_directions() {
        let mut state = state(1_000_000_000, 0);
        state.minimum_nck_units = 1_000_000;
        state.maximum_nck_units = 10_000_000;
        assert!(matches!(
            quote_sol_to_nck(999_999_999, &state),
            Err(NicechunkMarketError::SwapAmountOutsideLimits)
        ));
        assert!(matches!(
            quote_nck_to_sol(10_000_001, &state),
            Err(NicechunkMarketError::SwapAmountOutsideLimits)
        ));
        assert!(quote_sol_to_nck(1_000_000_000, &state).is_ok());
        assert!(quote_nck_to_sol(10_000_000, &state).is_ok());
    }

    #[test]
    fn zero_and_dust_outputs_are_rejected() {
        let state = state(u64::MAX, 0);
        assert!(matches!(
            quote_sol_to_nck(0, &state),
            Err(NicechunkMarketError::InvalidSwapAmount)
        ));
        assert!(matches!(
            quote_sol_to_nck(1, &state),
            Err(NicechunkMarketError::SwapAmountOutsideLimits)
                | Err(NicechunkMarketError::InvalidSwapAmount)
        ));
    }

    #[test]
    fn stale_revision_and_slippage_are_explicit_errors() {
        let state = state(1_000_000_000, 0);
        let args = SwapArgs {
            amount_in: 1_000_000_000,
            minimum_amount_out: 1_000_000,
            expected_revision: 2,
            deadline_slot: u64::MAX,
        };
        assert!(matches!(
            validate_swap_request(&state, &args),
            Err(ProgramError::Custom(code)) if code == NicechunkMarketError::SwapConfigRevisionMismatch as u32
        ));
        assert!(matches!(
            enforce_minimum_amount_out(999, 1_000),
            Err(ProgramError::Custom(code)) if code == NicechunkMarketError::SwapAmountOutTooLow as u32
        ));
    }

    #[test]
    fn withdrawals_require_pause_and_activation_sizes_both_reserves() {
        let mut state = state(25_000_000, 100);
        state.maximum_nck_units = 4_000_000;
        assert!(matches!(
            require_swap_paused(&state),
            Err(ProgramError::Custom(code)) if code == NicechunkMarketError::SwapMustBePaused as u32
        ));
        state.paused = true;
        assert!(require_swap_paused(&state).is_ok());
        assert_eq!(
            required_activation_liquidity(&state).unwrap(),
            (99_000_000, 3_960_000)
        );
    }
}
