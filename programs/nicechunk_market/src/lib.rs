#![allow(unexpected_cfgs)]

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    declare_id,
    entrypoint::ProgramResult,
    hash::hashv,
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
pub mod membership;
pub mod state;
pub mod treasury_swap;

use cluster_config::{
    MARKET_TREASURY, NCK_MINT, NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_BUILDING_PROGRAM_ID,
    NICECHUNK_CORE_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkMarketError};
use membership::{
    MarketUserState, CONTRACT_TYPE_BLANK_LAND, MARKET_USER_SEED, MAX_CONTRACT_PURCHASE_QUANTITY,
};
use state::{
    CreateListingArgs, ListingAccount, ListingInitArgs, LISTING_SEED, MARKET_AUTHORITY_SEED,
    SOURCE_BACKPACK, SOURCE_EQUIPMENT,
};

pub(crate) const NCK_DECIMALS: u8 = 6;
const TOKEN_ACCOUNT_MIN_LEN: usize = 165;
const TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;
const BACKPACK_HEADER_LEN: usize = 128;
const BACKPACK_LEN: usize = 8048;
const BACKPACK_VERSION: u16 = 4;
const BACKPACK_SEED: &[u8] = b"backpack";
const MATERIAL_PHYSICS_SEED: &[u8] = b"material-physics-v2";
const BACKPACK_ID_OFFSET: usize = 12;
const BACKPACK_SLOT_RECORD_LEN: usize = 80;
const BACKPACK_OWNER_OFFSET: usize = 20;
const BACKPACK_CAPACITY_OFFSET: usize = 52;
const BACKPACK_ITEM_COUNT_OFFSET: usize = 53;
const BACKPACK_FLAGS_OFFSET: usize = 55;
const BACKPACK_FLAG_MASS_STATE_VALID: u8 = 1;
const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
const BACKPACK_SLOT_KIND_ITEM: u8 = 2;
const BACKPACK_ITEM_CATEGORY_BLUEPRINT: u8 = 3;
const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
const PLAYER_PROFILE_VERSION: u16 = 7;
const PLAYER_PROFILE_LEN: usize = 773;
const PLAYER_PROFILE_SEED: &[u8] = b"player-v7";
const PLAYER_PROFILE_INITIALIZED_OFFSET: usize = 11;
const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;
const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET: usize = 102;
const PLAYER_PROFILE_EQUIPMENT_OFFSET: usize = 103;
const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT: usize = 9;
const PLAYER_EQUIPMENT_MAGIC: [u8; 8] = *b"NCKEQP01";
const PLAYER_EQUIPMENT_VERSION: u16 = 1;
const PLAYER_EQUIPMENT_SEED: &[u8] = b"player-equipment-v1";
const PLAYER_EQUIPMENT_LEN: usize = 7_040;
const PLAYER_EQUIPMENT_OWNER_OFFSET: usize = 12;
const PLAYER_EQUIPMENT_PROFILE_OFFSET: usize = 44;
const PLAYER_EQUIPMENT_GLOBAL_CONFIG_OFFSET: usize = 76;
const PLAYER_EQUIPMENT_SLOT_COUNT_OFFSET: usize = 108;
const PLAYER_EQUIPMENT_SLOTS_OFFSET: usize = 128;
const PLAYER_EQUIPMENT_SLOT_LEN: usize = 768;
const PLAYER_EQUIPMENT_RECORD_STATE_OFFSET: usize = 0;
const PLAYER_EQUIPMENT_RECORD_SLOT_OFFSET: usize = 1;
const PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET: usize = 3;
const PLAYER_EQUIPMENT_RECORD_BACKPACK_OFFSET: usize = 8;
const PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET: usize = 40;
const PLAYER_EQUIPMENT_FLAG_CUSTODY: u8 = 1 << 1;
const MARKET_FEE_BPS: u16 = 100;
const BPS_DENOMINATOR: u64 = 10_000;
pub const BLANK_LAND_CONTRACT_PRICE_BASE_UNITS: u64 = 10_000_000;
pub const LAND_CONTRACT_AUTHORITY_SEED: &[u8] = b"land-contract-authority-v1";
const GLOBAL_CONFIG_SEED: &[u8] = b"global-config";

declare_id!("1PwPzFtdJ5gQqku5gBo4b6Wvo48Qe8NuXSogUP8TWpR");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkMarketError::InvalidInstruction)?;

    match tag {
        0 => create_listing(program_id, accounts, payload),
        1 => cancel_listing(program_id, accounts),
        2 => buy_listing(program_id, accounts),
        3 => join_market(program_id, accounts),
        4 => buy_treasury_contract(program_id, accounts, payload),
        5 => update_land_contract_reservation(
            program_id,
            accounts,
            payload,
            LandContractReservationOperation::Reserve,
        ),
        6 => update_land_contract_reservation(
            program_id,
            accounts,
            payload,
            LandContractReservationOperation::Consume,
        ),
        7 => update_land_contract_reservation(
            program_id,
            accounts,
            payload,
            LandContractReservationOperation::Release,
        ),
        8 => treasury_swap::initialize_treasury_swap(program_id, accounts, payload),
        9 => treasury_swap::configure_treasury_swap(program_id, accounts, payload),
        10 => treasury_swap::deposit_treasury_swap_sol(program_id, accounts, payload),
        11 => treasury_swap::withdraw_treasury_swap_sol(program_id, accounts, payload),
        12 => treasury_swap::deposit_treasury_swap_nck(program_id, accounts, payload),
        13 => treasury_swap::withdraw_treasury_swap_nck(program_id, accounts, payload),
        14 => treasury_swap::swap_sol_for_nck(program_id, accounts, payload),
        15 => treasury_swap::swap_nck_for_sol(program_id, accounts, payload),
        _ => Err(NicechunkMarketError::InvalidInstruction.into()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContractAmountArgs {
    contract_type: u8,
    quantity: u32,
}

impl ContractAmountArgs {
    const LEN: usize = 5;

    fn unpack(payload: &[u8], maximum: Option<u32>) -> Result<Self, NicechunkMarketError> {
        if payload.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let args = Self {
            contract_type: payload[0],
            quantity: u32::from_le_bytes(
                payload[1..5]
                    .try_into()
                    .map_err(|_| NicechunkMarketError::InvalidInstruction)?,
            ),
        };
        if args.contract_type != CONTRACT_TYPE_BLANK_LAND {
            return Err(NicechunkMarketError::InvalidContractType);
        }
        if args.quantity == 0 || maximum.is_some_and(|limit| args.quantity > limit) {
            return Err(NicechunkMarketError::InvalidContractQuantity);
        }
        Ok(args)
    }
}

fn buy_treasury_contract(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    let args = ContractAmountArgs::unpack(payload, Some(MAX_CONTRACT_PURCHASE_QUANTITY))?;
    if accounts.len() != 6 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let buyer = &accounts[0];
    let market_user = &accounts[1];
    let buyer_nck_token = &accounts[2];
    let treasury_nck_token = &accounts[3];
    let nck_mint = &accounts[4];
    let token_program = &accounts[5];
    if !buyer.is_signer {
        return Err(NicechunkMarketError::InvalidBuyer.into());
    }
    if !market_user.is_writable || !buyer_nck_token.is_writable || !treasury_nck_token.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    require_key_eq(
        nck_mint.key,
        &NCK_MINT,
        NicechunkMarketError::InvalidNckMint,
    )?;
    require_key_eq(
        token_program.key,
        &spl_token::ID,
        NicechunkMarketError::InvalidTokenProgram,
    )?;
    validate_existing_market_user(program_id, market_user, buyer.key)?;
    validate_token_account(buyer_nck_token, &NCK_MINT, buyer.key)?;
    validate_token_account(treasury_nck_token, &NCK_MINT, &MARKET_TREASURY)?;
    let payment = blank_land_contract_payment(args.quantity)?;
    transfer_nck(
        buyer_nck_token,
        treasury_nck_token,
        nck_mint,
        buyer,
        token_program,
        payment,
    )?;
    let clock = Clock::get()?;
    let mut data = market_user.try_borrow_mut_data()?;
    MarketUserState::validate(&data, buyer.key)?;
    MarketUserState::credit_blank_land_contracts(&mut data, args.quantity, clock.slot)
}

fn blank_land_contract_payment(quantity: u32) -> Result<u64, NicechunkMarketError> {
    BLANK_LAND_CONTRACT_PRICE_BASE_UNITS
        .checked_mul(u64::from(quantity))
        .ok_or(NicechunkMarketError::InvalidContractQuantity)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LandContractReservationOperation {
    Reserve,
    Consume,
    Release,
}

fn update_land_contract_reservation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
    operation: LandContractReservationOperation,
) -> ProgramResult {
    let args = ContractAmountArgs::unpack(payload, None)?;
    if accounts.len() != 4 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let contract_authority = &accounts[0];
    let market_user = &accounts[1];
    let owner = &accounts[2];
    let global_config = &accounts[3];
    if !contract_authority.is_signer {
        return Err(NicechunkMarketError::InvalidContractAuthority.into());
    }
    if !market_user.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    let (expected_global_config, _) =
        Pubkey::find_program_address(&[GLOBAL_CONFIG_SEED], &NICECHUNK_CORE_PROGRAM_ID);
    require_key_eq(
        global_config.key,
        &expected_global_config,
        NicechunkMarketError::InvalidGlobalConfig,
    )?;
    require_key_eq(
        global_config.owner,
        &NICECHUNK_CORE_PROGRAM_ID,
        NicechunkMarketError::InvalidGlobalConfig,
    )?;
    let (expected_authority, _) = Pubkey::find_program_address(
        &[LAND_CONTRACT_AUTHORITY_SEED, global_config.key.as_ref()],
        &NICECHUNK_BUILDING_PROGRAM_ID,
    );
    require_key_eq(
        contract_authority.key,
        &expected_authority,
        NicechunkMarketError::InvalidContractAuthority,
    )?;
    validate_existing_market_user(program_id, market_user, owner.key)?;
    let clock = Clock::get()?;
    let mut data = market_user.try_borrow_mut_data()?;
    MarketUserState::validate(&data, owner.key)?;
    match operation {
        LandContractReservationOperation::Reserve => {
            MarketUserState::reserve_blank_land_contracts(&mut data, args.quantity, clock.slot)
        }
        LandContractReservationOperation::Consume => {
            MarketUserState::consume_reserved_blank_land_contracts(
                &mut data,
                args.quantity,
                clock.slot,
            )
        }
        LandContractReservationOperation::Release => {
            MarketUserState::release_reserved_blank_land_contracts(
                &mut data,
                args.quantity,
                clock.slot,
            )
        }
    }
}

#[cfg(test)]
mod land_contract_tests {
    use super::*;

    #[test]
    fn contract_payload_is_typed_and_bounded_for_treasury_sales() {
        let mut payload = [0_u8; ContractAmountArgs::LEN];
        payload[0] = CONTRACT_TYPE_BLANK_LAND;
        payload[1..5].copy_from_slice(&12_u32.to_le_bytes());
        assert_eq!(
            ContractAmountArgs::unpack(&payload, Some(MAX_CONTRACT_PURCHASE_QUANTITY)).unwrap(),
            ContractAmountArgs {
                contract_type: CONTRACT_TYPE_BLANK_LAND,
                quantity: 12,
            }
        );
        payload[0] = CONTRACT_TYPE_BLANK_LAND + 1;
        assert!(matches!(
            ContractAmountArgs::unpack(&payload, None),
            Err(NicechunkMarketError::InvalidContractType)
        ));
        payload[0] = CONTRACT_TYPE_BLANK_LAND;
        payload[1..5].copy_from_slice(&(MAX_CONTRACT_PURCHASE_QUANTITY + 1).to_le_bytes());
        assert!(matches!(
            ContractAmountArgs::unpack(&payload, Some(MAX_CONTRACT_PURCHASE_QUANTITY)),
            Err(NicechunkMarketError::InvalidContractQuantity)
        ));
    }

    #[test]
    fn blank_land_contract_price_is_exactly_ten_nck() {
        assert_eq!(
            BLANK_LAND_CONTRACT_PRICE_BASE_UNITS,
            10 * 10_u64.pow(NCK_DECIMALS as u32)
        );
        assert_eq!(blank_land_contract_payment(1).unwrap(), 10_000_000);
        assert_eq!(
            blank_land_contract_payment(MAX_CONTRACT_PURCHASE_QUANTITY).unwrap(),
            40_960_000_000
        );
    }

    #[test]
    fn land_contract_reservation_operations_are_distinct() {
        assert_ne!(
            LandContractReservationOperation::Reserve,
            LandContractReservationOperation::Consume
        );
        assert_ne!(
            LandContractReservationOperation::Consume,
            LandContractReservationOperation::Release
        );
    }
}

fn join_market(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 3 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let market_user = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    if !owner.is_signer || !owner.is_writable || !market_user.is_writable {
        return Err(NicechunkMarketError::InvalidMarketUser.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    validate_market_user_pda(program_id, market_user.key, owner.key)?;
    if market_user.owner == program_id {
        return Err(NicechunkMarketError::MarketAlreadyJoined.into());
    }
    if market_user.owner != &system_program::ID || market_user.data_len() != 0 {
        return Err(NicechunkMarketError::InvalidSystemAccount.into());
    }

    let clock = Clock::get()?;
    create_market_user_account(
        program_id,
        owner,
        market_user,
        system_program_account,
        owner.key,
        clock.slot,
    )
}

fn create_listing(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    let args = CreateListingArgs::unpack(payload)?;
    if args.source_type != SOURCE_BACKPACK && args.source_type != SOURCE_EQUIPMENT {
        return Err(NicechunkMarketError::InvalidInstruction.into());
    }
    let expected_account_count = if args.source_type == SOURCE_BACKPACK {
        6
    } else {
        8
    };
    if accounts.len() != expected_account_count {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let seller = next_account_info(account_info_iter)?;
    let listing = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !seller.is_signer || !seller.is_writable {
        return Err(NicechunkMarketError::InvalidSeller.into());
    }
    if !listing.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    let bump = validate_listing_pda(program_id, listing.key, seller.key, args.listing_id)?;
    if listing.owner == program_id {
        return Err(NicechunkMarketError::ListingAlreadyInitialized.into());
    }
    if listing.owner != &system_program::ID || listing.data_len() != 0 {
        return Err(NicechunkMarketError::InvalidSystemAccount.into());
    }

    let mut backpack_source = None;
    let mut equipment_source = None;
    let source_slot = match args.source_type {
        SOURCE_BACKPACK => {
            let backpack = next_account_info(account_info_iter)?;
            let backpack_program = next_account_info(account_info_iter)?;
            require_key_eq(
                backpack_program.key,
                &NICECHUNK_BACKPACK_PROGRAM_ID,
                NicechunkMarketError::InvalidBackpackProgram,
            )?;
            let source_slot =
                read_backpack_slot_for_listing(backpack, seller.key, args.source_index)?;
            backpack_source = Some((backpack, backpack_program));
            source_slot
        }
        SOURCE_EQUIPMENT => {
            let player_profile = next_account_info(account_info_iter)?;
            let player_equipment = next_account_info(account_info_iter)?;
            let global_config = next_account_info(account_info_iter)?;
            let player_program = next_account_info(account_info_iter)?;
            require_key_eq(
                player_program.key,
                &NICECHUNK_PLAYER_PROGRAM_ID,
                NicechunkMarketError::InvalidPlayerProgram,
            )?;
            let source_slot = read_equipment_slot_for_listing(
                player_profile,
                player_equipment,
                seller.key,
                args.source_index,
                player_program.key,
                global_config.key,
            )?;
            equipment_source = Some((
                player_profile,
                player_equipment,
                global_config,
                player_program,
            ));
            source_slot
        }
        _ => return Err(NicechunkMarketError::InvalidInstruction.into()),
    };
    let market_user = next_account_info(account_info_iter)?;
    validate_transferable_source_slot(&source_slot)?;

    let clock = Clock::get()?;
    validate_existing_market_user(program_id, market_user, seller.key)?;
    if !market_user.is_writable {
        return Err(NicechunkMarketError::InvalidMarketUser.into());
    }
    {
        let mut data = market_user.try_borrow_mut_data()?;
        MarketUserState::validate(&data, seller.key)?;
        MarketUserState::increment_active(&mut data, clock.slot)?;
    }

    create_listing_pda(
        seller,
        listing,
        system_program_account,
        program_id,
        args.listing_id,
        bump,
    )?;

    {
        let mut data = listing.try_borrow_mut_data()?;
        ListingAccount::pack(
            &mut data,
            &ListingInitArgs {
                bump,
                seller: seller.key,
                listing_id: args.listing_id,
                currency: args.currency,
                source_type: args.source_type,
                source_index: args.source_index,
                price_base_units: args.price_base_units,
                source_slot,
                created_slot: clock.slot,
                created_at: clock.unix_timestamp,
            },
        )?;
    }

    if let Some((backpack, backpack_program)) = backpack_source {
        remove_backpack_resource(seller, backpack, backpack_program, args.source_index as u16)?;
    }
    if let Some((player_profile, player_equipment, global_config, player_program)) =
        equipment_source
    {
        release_player_equipment_to_market(
            seller,
            player_profile,
            player_equipment,
            global_config,
            listing,
            player_program,
        )?;
    }
    Ok(())
}

fn cancel_listing(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 8 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let seller = next_account_info(account_info_iter)?;
    let listing = next_account_info(account_info_iter)?;

    if !seller.is_signer || !seller.is_writable {
        return Err(NicechunkMarketError::InvalidSeller.into());
    }
    if !listing.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    require_key_eq(
        listing.owner,
        program_id,
        NicechunkMarketError::InvalidListingOwner,
    )?;

    let data = listing.try_borrow_data()?;
    ListingAccount::validate_active_seller(&data, seller.key)?;
    let listing_id = ListingAccount::listing_id(&data)?;
    let source_slot = ListingAccount::source_slot(&data)?;
    drop(data);

    validate_listing_pda(program_id, listing.key, seller.key, listing_id)?;

    let backpack = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let market_authority = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let market_user = next_account_info(account_info_iter)?;
    validate_material_physics_accounts(backpack_program, material_physics, global_config)?;
    validate_existing_market_user(program_id, market_user, seller.key)?;
    append_market_slot_to_backpack(
        program_id,
        market_authority,
        seller,
        backpack,
        backpack_program,
        material_physics,
        &source_slot,
    )?;

    let clock = Clock::get()?;
    {
        let mut data = listing.try_borrow_mut_data()?;
        ListingAccount::mark_canceled(&mut data, clock.slot, clock.unix_timestamp)?;
    }
    decrement_market_user_active(market_user, seller.key, clock.slot)?;
    Ok(())
}

fn buy_listing(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 4 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }

    let account_info_iter = &mut accounts.iter();
    let buyer = next_account_info(account_info_iter)?;
    let seller = next_account_info(account_info_iter)?;
    let listing = next_account_info(account_info_iter)?;

    if !buyer.is_signer || !buyer.is_writable {
        return Err(NicechunkMarketError::InvalidBuyer.into());
    }
    if !seller.is_writable {
        return Err(NicechunkMarketError::InvalidSeller.into());
    }
    if buyer.key == seller.key {
        return Err(NicechunkMarketError::InvalidBuyer.into());
    }
    if !listing.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    require_key_eq(
        listing.owner,
        program_id,
        NicechunkMarketError::InvalidListingOwner,
    )?;
    let data = listing.try_borrow_data()?;
    ListingAccount::validate_active(&data)?;
    let listing_seller = ListingAccount::seller(&data)?;
    require_key_eq(
        seller.key,
        &listing_seller,
        NicechunkMarketError::UnauthorizedSeller,
    )?;
    let listing_id = ListingAccount::listing_id(&data)?;
    validate_listing_pda(program_id, listing.key, seller.key, listing_id)?;
    let currency = ListingAccount::currency(&data)?;
    let price_base_units = ListingAccount::price_base_units(&data)?;
    let source_slot = ListingAccount::source_slot(&data)?;
    drop(data);
    validate_transferable_source_slot(&source_slot)?;

    let base_account_count = match currency {
        state::CURRENCY_SOL => 5,
        state::CURRENCY_NCK => 8,
        _ => return Err(NicechunkMarketError::UnsupportedCurrency.into()),
    };
    let expected_account_count = base_account_count + 7;
    if accounts.len() != expected_account_count {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }

    let mut sol_payment_accounts = None;
    let mut nck_payment_accounts = None;
    match currency {
        state::CURRENCY_SOL => {
            let system_program_account = next_account_info(account_info_iter)?;
            let treasury = next_account_info(account_info_iter)?;
            require_key_eq(
                system_program_account.key,
                &system_program::ID,
                NicechunkMarketError::InvalidSystemProgram,
            )?;
            require_key_eq(
                treasury.key,
                &MARKET_TREASURY,
                NicechunkMarketError::InvalidTreasury,
            )?;
            sol_payment_accounts = Some((system_program_account, treasury));
        }
        state::CURRENCY_NCK => {
            let buyer_nck_token = next_account_info(account_info_iter)?;
            let seller_nck_token = next_account_info(account_info_iter)?;
            let treasury_nck_token = next_account_info(account_info_iter)?;
            let nck_mint = next_account_info(account_info_iter)?;
            let token_program = next_account_info(account_info_iter)?;
            if !buyer_nck_token.is_writable
                || !seller_nck_token.is_writable
                || !treasury_nck_token.is_writable
            {
                return Err(NicechunkMarketError::InvalidWritableAccount.into());
            }
            require_key_eq(
                nck_mint.key,
                &NCK_MINT,
                NicechunkMarketError::InvalidNckMint,
            )?;
            require_key_eq(
                token_program.key,
                &spl_token::ID,
                NicechunkMarketError::InvalidTokenProgram,
            )?;
            validate_token_account(buyer_nck_token, &NCK_MINT, buyer.key)?;
            validate_token_account(seller_nck_token, &NCK_MINT, seller.key)?;
            validate_token_account(treasury_nck_token, &NCK_MINT, &MARKET_TREASURY)?;
            nck_payment_accounts = Some((
                buyer_nck_token,
                seller_nck_token,
                treasury_nck_token,
                nck_mint,
                token_program,
            ));
        }
        _ => return Err(NicechunkMarketError::UnsupportedCurrency.into()),
    }

    let buyer_backpack = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let market_authority = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let seller_market_user = next_account_info(account_info_iter)?;
    let buyer_market_user = next_account_info(account_info_iter)?;
    validate_material_physics_accounts(backpack_program, material_physics, global_config)?;
    validate_existing_market_user(program_id, seller_market_user, seller.key)?;
    validate_existing_market_user(program_id, buyer_market_user, buyer.key)?;
    let (seller_amount, fee_amount) = split_market_payment(price_base_units)?;
    if let Some((system_program_account, treasury)) = sol_payment_accounts {
        if seller_amount > 0 {
            let seller_payment = system_instruction::transfer(buyer.key, seller.key, seller_amount);
            invoke(
                &seller_payment,
                &[
                    buyer.clone(),
                    seller.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        if fee_amount > 0 {
            let fee_payment = system_instruction::transfer(buyer.key, treasury.key, fee_amount);
            invoke(
                &fee_payment,
                &[
                    buyer.clone(),
                    treasury.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
    }
    if let Some((buyer_nck_token, seller_nck_token, treasury_nck_token, nck_mint, token_program)) =
        nck_payment_accounts
    {
        if seller_amount > 0 {
            transfer_nck(
                buyer_nck_token,
                seller_nck_token,
                nck_mint,
                buyer,
                token_program,
                seller_amount,
            )?;
        }
        if fee_amount > 0 {
            transfer_nck(
                buyer_nck_token,
                treasury_nck_token,
                nck_mint,
                buyer,
                token_program,
                fee_amount,
            )?;
        }
    }
    append_market_slot_to_backpack(
        program_id,
        market_authority,
        buyer,
        buyer_backpack,
        backpack_program,
        material_physics,
        &source_slot,
    )?;

    let clock = Clock::get()?;
    {
        let mut data = listing.try_borrow_mut_data()?;
        ListingAccount::mark_sold(&mut data, buyer.key, clock.slot, clock.unix_timestamp)?;
    }
    decrement_market_user_active(seller_market_user, seller.key, clock.slot)?;
    Ok(())
}

fn split_market_payment(price_base_units: u64) -> Result<(u64, u64), NicechunkMarketError> {
    let fee = price_base_units
        .checked_mul(MARKET_FEE_BPS as u64)
        .ok_or(NicechunkMarketError::InvalidFee)?
        / BPS_DENOMINATOR;
    let seller_amount = price_base_units
        .checked_sub(fee)
        .ok_or(NicechunkMarketError::InvalidFee)?;
    Ok((seller_amount, fee))
}

fn transfer_nck<'a>(
    source_token: &AccountInfo<'a>,
    destination_token: &AccountInfo<'a>,
    nck_mint: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let ix = spl_token::instruction::transfer_checked(
        token_program.key,
        source_token.key,
        nck_mint.key,
        destination_token.key,
        owner.key,
        &[],
        amount,
        NCK_DECIMALS,
    )
    .map_err(|_| NicechunkMarketError::InvalidInstruction)?;
    invoke(
        &ix,
        &[
            source_token.clone(),
            nck_mint.clone(),
            destination_token.clone(),
            owner.clone(),
            token_program.clone(),
        ],
    )
}

fn read_backpack_slot_for_listing(
    backpack: &AccountInfo,
    owner: &Pubkey,
    source_index: u8,
) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], solana_program::program_error::ProgramError> {
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    let data = backpack.try_borrow_data()?;
    validate_backpack_data_and_pda(backpack.key, &data, owner)?;
    copy_backpack_slot_at(&data, source_index)
}

fn read_equipment_slot_for_listing(
    player_profile: &AccountInfo,
    player_equipment: &AccountInfo,
    owner: &Pubkey,
    equipment_slot: u8,
    player_program: &Pubkey,
    global_config: &Pubkey,
) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], solana_program::program_error::ProgramError> {
    require_key_eq(
        player_profile.owner,
        player_program,
        NicechunkMarketError::InvalidPlayerProgram,
    )?;
    require_key_eq(
        player_equipment.owner,
        player_program,
        NicechunkMarketError::InvalidPlayerProgram,
    )?;
    let profile_data = player_profile.try_borrow_data()?;
    validate_player_profile_data_and_pda(
        player_profile.key,
        &profile_data,
        owner,
        player_program,
        global_config,
    )?;
    if equipment_slot as usize >= PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT {
        return Err(NicechunkMarketError::InvalidEquipmentSource.into());
    }
    let equipment_data = player_equipment.try_borrow_data()?;
    validate_player_equipment_data_and_pda(
        player_equipment.key,
        &equipment_data,
        owner,
        player_profile.key,
        player_program,
        global_config,
    )?;
    validate_equipment_source_record(&profile_data, &equipment_data, equipment_slot)
}

fn validate_equipment_source_record(
    profile_data: &[u8],
    equipment_data: &[u8],
    equipment_slot: u8,
) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], solana_program::program_error::ProgramError> {
    if profile_data.len() != PLAYER_PROFILE_LEN
        || equipment_data.len() != PLAYER_EQUIPMENT_LEN
        || equipment_slot as usize >= PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT
    {
        return Err(NicechunkMarketError::InvalidEquipmentSource.into());
    }
    let offset =
        PLAYER_EQUIPMENT_SLOTS_OFFSET + equipment_slot as usize * PLAYER_EQUIPMENT_SLOT_LEN;
    if equipment_data[offset + PLAYER_EQUIPMENT_RECORD_STATE_OFFSET] != 1
        || equipment_data[offset + PLAYER_EQUIPMENT_RECORD_SLOT_OFFSET] != equipment_slot
        || equipment_data[offset + PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET]
            & PLAYER_EQUIPMENT_FLAG_CUSTODY
            == 0
    {
        return Err(NicechunkMarketError::InvalidEquipmentSource.into());
    }
    let source_slot = copy_valid_backpack_slot(
        &equipment_data[offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET
            ..offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN],
    )?;
    let identity = Pubkey::new_from_array(
        hashv(&[
            b"equipment-v2",
            &equipment_data[offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_OFFSET
                ..offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_OFFSET + 32],
            &source_slot,
        ])
        .to_bytes(),
    );
    let profile_offset = PLAYER_PROFILE_EQUIPMENT_OFFSET + equipment_slot as usize * 32;
    if &profile_data[profile_offset..profile_offset + 32] != identity.as_ref() {
        return Err(NicechunkMarketError::InvalidEquipmentSource.into());
    }
    Ok(source_slot)
}

fn validate_player_equipment_data_and_pda(
    player_equipment: &Pubkey,
    data: &[u8],
    owner: &Pubkey,
    player_profile: &Pubkey,
    player_program: &Pubkey,
    global_config: &Pubkey,
) -> ProgramResult {
    if data.len() != PLAYER_EQUIPMENT_LEN
        || data[0..8] != PLAYER_EQUIPMENT_MAGIC
        || u16::from_le_bytes([data[8], data[9]]) != PLAYER_EQUIPMENT_VERSION
        || data[11] != 1
        || data[PLAYER_EQUIPMENT_SLOT_COUNT_OFFSET] as usize != PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT
        || &data[PLAYER_EQUIPMENT_OWNER_OFFSET..PLAYER_EQUIPMENT_OWNER_OFFSET + 32]
            != owner.as_ref()
        || &data[PLAYER_EQUIPMENT_PROFILE_OFFSET..PLAYER_EQUIPMENT_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        || &data[PLAYER_EQUIPMENT_GLOBAL_CONFIG_OFFSET..PLAYER_EQUIPMENT_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
    {
        return Err(NicechunkMarketError::InvalidEquipmentSource.into());
    }
    for slot in 0..PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT {
        let offset = PLAYER_EQUIPMENT_SLOTS_OFFSET + slot * PLAYER_EQUIPMENT_SLOT_LEN;
        let state = data[offset + PLAYER_EQUIPMENT_RECORD_STATE_OFFSET];
        if state > 1
            || data[offset + PLAYER_EQUIPMENT_RECORD_SLOT_OFFSET] as usize != slot
            || (state == 1
                && data[offset + PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET]
                    & PLAYER_EQUIPMENT_FLAG_CUSTODY
                    == 0)
        {
            return Err(NicechunkMarketError::InvalidEquipmentSource.into());
        }
    }
    let (expected, _) =
        Pubkey::find_program_address(&[PLAYER_EQUIPMENT_SEED, owner.as_ref()], player_program);
    require_key_eq(
        player_equipment,
        &expected,
        NicechunkMarketError::InvalidEquipmentSource,
    )
}

fn validate_backpack_data_and_pda(backpack: &Pubkey, data: &[u8], owner: &Pubkey) -> ProgramResult {
    if data.len() != BACKPACK_LEN
        || data[0..8] != *b"NCKBPK01"
        || u16::from_le_bytes([data[8], data[9]]) != BACKPACK_VERSION
        || data[11] != 1
        || data[BACKPACK_FLAGS_OFFSET] & BACKPACK_FLAG_MASS_STATE_VALID == 0
    {
        return Err(NicechunkMarketError::InvalidBackpackData.into());
    }
    if &data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32] != owner.as_ref() {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    let capacity = data[BACKPACK_CAPACITY_OFFSET];
    let item_count = data[BACKPACK_ITEM_COUNT_OFFSET];
    if capacity == 0 || capacity > 99 || item_count > capacity {
        return Err(NicechunkMarketError::InvalidBackpackData.into());
    }
    let backpack_id = read_u64(data, BACKPACK_ID_OFFSET);
    let backpack_id_bytes = backpack_id.to_le_bytes();
    let (expected, _) = Pubkey::find_program_address(
        &[BACKPACK_SEED, owner.as_ref(), &backpack_id_bytes],
        &NICECHUNK_BACKPACK_PROGRAM_ID,
    );
    require_key_eq(
        backpack,
        &expected,
        NicechunkMarketError::InvalidEscrowInventory,
    )
}

fn validate_player_profile_data_and_pda(
    player_profile: &Pubkey,
    data: &[u8],
    owner: &Pubkey,
    player_program: &Pubkey,
    global_config: &Pubkey,
) -> ProgramResult {
    if data.len() != PLAYER_PROFILE_LEN
        || data[0..8] != PLAYER_PROFILE_MAGIC
        || u16::from_le_bytes([data[8], data[9]]) != PLAYER_PROFILE_VERSION
        || data[PLAYER_PROFILE_INITIALIZED_OFFSET] != 1
        || data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] as usize
            != PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT
    {
        return Err(NicechunkMarketError::InvalidPlayerProfile.into());
    }
    if &data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32] != owner.as_ref() {
        return Err(NicechunkMarketError::InvalidPlayerProfile.into());
    }
    if &data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
        != global_config.as_ref()
    {
        return Err(NicechunkMarketError::InvalidPlayerProfile.into());
    }
    let (expected, _) =
        Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.as_ref()], player_program);
    require_key_eq(
        player_profile,
        &expected,
        NicechunkMarketError::InvalidPlayerProfile,
    )
}

fn copy_backpack_slot_at(
    data: &[u8],
    source_index: u8,
) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], solana_program::program_error::ProgramError> {
    let item_count = data[BACKPACK_ITEM_COUNT_OFFSET];
    if source_index >= item_count {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    let offset = BACKPACK_HEADER_LEN + source_index as usize * BACKPACK_SLOT_RECORD_LEN;
    copy_valid_backpack_slot(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])
}

fn copy_valid_backpack_slot(
    slot: &[u8],
) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], solana_program::program_error::ProgramError> {
    if slot.len() != BACKPACK_SLOT_RECORD_LEN {
        return Err(NicechunkMarketError::InvalidBackpackData.into());
    }
    if slot[0] != BACKPACK_SLOT_KIND_BLOCK && slot[0] != BACKPACK_SLOT_KIND_ITEM {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    if u32::from_le_bytes([slot[4], slot[5], slot[6], slot[7]]) == 0 {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    let mut source_slot = [0_u8; BACKPACK_SLOT_RECORD_LEN];
    source_slot.copy_from_slice(slot);
    Ok(source_slot)
}

fn validate_transferable_source_slot(
    source_slot: &[u8; BACKPACK_SLOT_RECORD_LEN],
) -> ProgramResult {
    if source_slot[0] == BACKPACK_SLOT_KIND_ITEM
        && source_slot[1] == BACKPACK_ITEM_CATEGORY_BLUEPRINT
    {
        return Err(NicechunkMarketError::NonTransferableItem.into());
    }
    Ok(())
}

fn release_player_equipment_to_market<'a>(
    seller: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    player_equipment: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    listing: &AccountInfo<'a>,
    player_program: &AccountInfo<'a>,
) -> ProgramResult {
    let ix = solana_program::instruction::Instruction {
        program_id: NICECHUNK_PLAYER_PROGRAM_ID,
        accounts: vec![
            solana_program::instruction::AccountMeta::new_readonly(*seller.key, true),
            solana_program::instruction::AccountMeta::new(*player_profile.key, false),
            solana_program::instruction::AccountMeta::new(*player_equipment.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*global_config.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*listing.key, false),
        ],
        data: vec![16],
    };
    invoke(
        &ix,
        &[
            seller.clone(),
            player_profile.clone(),
            player_equipment.clone(),
            global_config.clone(),
            listing.clone(),
            player_program.clone(),
        ],
    )
}

fn remove_backpack_resource<'a>(
    seller: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    backpack_program: &AccountInfo<'a>,
    source_index: u16,
) -> ProgramResult {
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    if source_index > u8::MAX as u16 {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    let ix = solana_program::instruction::Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            solana_program::instruction::AccountMeta::new(*seller.key, true),
            solana_program::instruction::AccountMeta::new(*backpack.key, false),
        ],
        data: backpack_cpi_data(&[2, source_index as u8]),
    };
    invoke(
        &ix,
        &[seller.clone(), backpack.clone(), backpack_program.clone()],
    )
}

fn append_market_slot_to_backpack<'a>(
    program_id: &Pubkey,
    market_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    backpack_program: &AccountInfo<'a>,
    material_physics: &AccountInfo<'a>,
    source_slot: &[u8; BACKPACK_SLOT_RECORD_LEN],
) -> ProgramResult {
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    let (expected_authority, bump) =
        Pubkey::find_program_address(&[MARKET_AUTHORITY_SEED], program_id);
    require_key_eq(
        market_authority.key,
        &expected_authority,
        NicechunkMarketError::InvalidMarketAuthority,
    )?;
    let mut data = Vec::with_capacity(1 + BACKPACK_SLOT_RECORD_LEN);
    data.push(3);
    data.extend_from_slice(source_slot);
    let data = backpack_cpi_data(&data);
    let ix = solana_program::instruction::Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            solana_program::instruction::AccountMeta::new_readonly(*market_authority.key, true),
            solana_program::instruction::AccountMeta::new_readonly(*owner.key, false),
            solana_program::instruction::AccountMeta::new(*backpack.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*material_physics.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[
            market_authority.clone(),
            owner.clone(),
            backpack.clone(),
            material_physics.clone(),
            backpack_program.clone(),
        ],
        &[&[MARKET_AUTHORITY_SEED, &[bump]]],
    )
}

fn validate_material_physics_accounts(
    backpack_program: &AccountInfo,
    material_physics: &AccountInfo,
    global_config: &AccountInfo,
) -> ProgramResult {
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    let (expected, _) = Pubkey::find_program_address(
        &[MATERIAL_PHYSICS_SEED, global_config.key.as_ref()],
        backpack_program.key,
    );
    require_key_eq(
        material_physics.key,
        &expected,
        NicechunkMarketError::InvalidMaterialPhysics,
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

fn validate_token_account(
    token_account: &AccountInfo,
    mint: &Pubkey,
    owner: &Pubkey,
) -> ProgramResult {
    if token_account.owner != &spl_token::ID {
        return Err(NicechunkMarketError::InvalidTokenAccount.into());
    }
    let data = token_account.try_borrow_data()?;
    if data.len() < TOKEN_ACCOUNT_MIN_LEN {
        return Err(NicechunkMarketError::InvalidTokenAccount.into());
    }
    if &data[TOKEN_ACCOUNT_MINT_OFFSET..TOKEN_ACCOUNT_MINT_OFFSET + 32] != mint.as_ref()
        || &data[TOKEN_ACCOUNT_OWNER_OFFSET..TOKEN_ACCOUNT_OWNER_OFFSET + 32] != owner.as_ref()
    {
        return Err(NicechunkMarketError::InvalidTokenAccount.into());
    }
    Ok(())
}

fn validate_market_user_pda(
    program_id: &Pubkey,
    market_user: &Pubkey,
    owner: &Pubkey,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let (expected_market_user, bump) =
        Pubkey::find_program_address(&[MARKET_USER_SEED, owner.as_ref()], program_id);
    require_key_eq(
        market_user,
        &expected_market_user,
        NicechunkMarketError::InvalidMarketUser,
    )?;
    Ok(bump)
}

fn create_market_user_account<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    market_user: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    owner: &Pubkey,
    current_slot: u64,
) -> ProgramResult {
    if !payer.is_signer || !payer.is_writable || !market_user.is_writable {
        return Err(NicechunkMarketError::InvalidMarketUser.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    let bump = validate_market_user_pda(program_id, market_user.key, owner)?;
    if market_user.owner != &system_program::ID || market_user.data_len() != 0 {
        return Err(NicechunkMarketError::InvalidMarketUser.into());
    }

    let rent = Rent::get()?;
    let create = system_instruction::create_account(
        payer.key,
        market_user.key,
        rent.minimum_balance(MarketUserState::LEN),
        MarketUserState::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            payer.clone(),
            market_user.clone(),
            system_program_account.clone(),
        ],
        &[&[MARKET_USER_SEED, owner.as_ref(), &[bump]]],
    )?;
    MarketUserState::pack(
        &mut market_user.try_borrow_mut_data()?,
        bump,
        owner,
        current_slot,
    )
}

fn validate_existing_market_user(
    program_id: &Pubkey,
    market_user: &AccountInfo,
    owner: &Pubkey,
) -> ProgramResult {
    if market_user.owner != program_id {
        return Err(NicechunkMarketError::InvalidMarketUser.into());
    }
    validate_market_user_pda(program_id, market_user.key, owner)?;
    MarketUserState::validate(&market_user.try_borrow_data()?, owner)
}

fn decrement_market_user_active(
    market_user: &AccountInfo,
    owner: &Pubkey,
    current_slot: u64,
) -> ProgramResult {
    if !market_user.is_writable {
        return Err(NicechunkMarketError::InvalidMarketUser.into());
    }
    let mut data = market_user.try_borrow_mut_data()?;
    MarketUserState::validate(&data, owner)?;
    MarketUserState::decrement_active(&mut data, current_slot)
}

fn validate_listing_pda(
    program_id: &Pubkey,
    listing: &Pubkey,
    seller: &Pubkey,
    listing_id: u64,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let listing_id_bytes = listing_id.to_le_bytes();
    let (expected_listing, bump) = Pubkey::find_program_address(
        &[LISTING_SEED, seller.as_ref(), &listing_id_bytes],
        program_id,
    );
    require_key_eq(
        listing,
        &expected_listing,
        NicechunkMarketError::InvalidListingPda,
    )?;
    Ok(bump)
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

fn create_listing_pda<'a>(
    seller: &AccountInfo<'a>,
    listing: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    listing_id: u64,
    bump: u8,
) -> ProgramResult {
    let listing_id_bytes = listing_id.to_le_bytes();
    let seeds = &[
        LISTING_SEED,
        seller.key.as_ref(),
        &listing_id_bytes,
        &[bump],
    ];
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(ListingAccount::LEN);
    let create = system_instruction::create_account(
        seller.key,
        listing.key,
        lamports,
        ListingAccount::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[
            seller.clone(),
            listing.clone(),
            system_program_account.clone(),
        ],
        &[seeds],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_rejects_non_transferable_blueprint_slots() {
        let mut blueprint = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        blueprint[0] = BACKPACK_SLOT_KIND_ITEM;
        blueprint[1] = BACKPACK_ITEM_CATEGORY_BLUEPRINT;
        blueprint[4..8].copy_from_slice(&1_u32.to_le_bytes());
        assert!(matches!(
            validate_transferable_source_slot(&blueprint),
            Err(solana_program::program_error::ProgramError::Custom(6633))
        ));

        let mut forged = blueprint;
        forged[1] = 2;
        assert!(validate_transferable_source_slot(&forged).is_ok());
    }

    #[test]
    fn market_rejects_retired_or_uninitialized_player_profiles() {
        let owner = Pubkey::new_unique();
        let player_program = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let (player_profile, _) =
            Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.as_ref()], &player_program);
        let mut data = vec![0_u8; PLAYER_PROFILE_LEN];
        data[0..8].copy_from_slice(&PLAYER_PROFILE_MAGIC);
        data[8..10].copy_from_slice(&PLAYER_PROFILE_VERSION.to_le_bytes());
        data[PLAYER_PROFILE_INITIALIZED_OFFSET] = 1;
        data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] =
            PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT as u8;

        validate_player_profile_data_and_pda(
            &player_profile,
            &data,
            &owner,
            &player_program,
            &global_config,
        )
        .unwrap();

        let mut retired = data.clone();
        retired[8..10].copy_from_slice(&(PLAYER_PROFILE_VERSION - 1).to_le_bytes());
        assert!(validate_player_profile_data_and_pda(
            &player_profile,
            &retired,
            &owner,
            &player_program,
            &global_config,
        )
        .is_err());

        let mut uninitialized = data;
        uninitialized[PLAYER_PROFILE_INITIALIZED_OFFSET] = 0;
        assert!(validate_player_profile_data_and_pda(
            &player_profile,
            &uninitialized,
            &owner,
            &player_program,
            &global_config,
        )
        .is_err());
    }

    #[test]
    fn market_equipment_source_requires_current_custody_and_matching_identity() {
        let owner = Pubkey::new_unique();
        let player_program = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let backpack = Pubkey::new_unique();
        let equipment_slot = 5_u8;
        let (player_profile, profile_bump) =
            Pubkey::find_program_address(&[PLAYER_PROFILE_SEED, owner.as_ref()], &player_program);
        let (player_equipment, equipment_bump) =
            Pubkey::find_program_address(&[PLAYER_EQUIPMENT_SEED, owner.as_ref()], &player_program);

        let mut profile_data = vec![0_u8; PLAYER_PROFILE_LEN];
        profile_data[0..8].copy_from_slice(&PLAYER_PROFILE_MAGIC);
        profile_data[8..10].copy_from_slice(&PLAYER_PROFILE_VERSION.to_le_bytes());
        profile_data[10] = profile_bump;
        profile_data[PLAYER_PROFILE_INITIALIZED_OFFSET] = 1;
        profile_data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        profile_data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        profile_data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] =
            PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT as u8;

        let mut equipment_data = vec![0_u8; PLAYER_EQUIPMENT_LEN];
        equipment_data[0..8].copy_from_slice(&PLAYER_EQUIPMENT_MAGIC);
        equipment_data[8..10].copy_from_slice(&PLAYER_EQUIPMENT_VERSION.to_le_bytes());
        equipment_data[10] = equipment_bump;
        equipment_data[11] = 1;
        equipment_data[PLAYER_EQUIPMENT_OWNER_OFFSET..PLAYER_EQUIPMENT_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        equipment_data[PLAYER_EQUIPMENT_PROFILE_OFFSET..PLAYER_EQUIPMENT_PROFILE_OFFSET + 32]
            .copy_from_slice(player_profile.as_ref());
        equipment_data
            [PLAYER_EQUIPMENT_GLOBAL_CONFIG_OFFSET..PLAYER_EQUIPMENT_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        equipment_data[PLAYER_EQUIPMENT_SLOT_COUNT_OFFSET] =
            PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT as u8;
        for slot in 0..PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT {
            let offset = PLAYER_EQUIPMENT_SLOTS_OFFSET + slot * PLAYER_EQUIPMENT_SLOT_LEN;
            equipment_data[offset + PLAYER_EQUIPMENT_RECORD_SLOT_OFFSET] = slot as u8;
        }

        let offset =
            PLAYER_EQUIPMENT_SLOTS_OFFSET + equipment_slot as usize * PLAYER_EQUIPMENT_SLOT_LEN;
        equipment_data[offset + PLAYER_EQUIPMENT_RECORD_STATE_OFFSET] = 1;
        equipment_data[offset + PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET] =
            PLAYER_EQUIPMENT_FLAG_CUSTODY;
        equipment_data[offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_OFFSET
            ..offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_OFFSET + 32]
            .copy_from_slice(backpack.as_ref());
        let mut source_slot = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        source_slot[0] = BACKPACK_SLOT_KIND_BLOCK;
        source_slot[4..8].copy_from_slice(&1_u32.to_le_bytes());
        source_slot[8..12].copy_from_slice(&17_i32.to_le_bytes());
        equipment_data[offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET
            ..offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN]
            .copy_from_slice(&source_slot);
        let identity = hashv(&[b"equipment-v2", backpack.as_ref(), &source_slot]).to_bytes();
        let profile_offset = PLAYER_PROFILE_EQUIPMENT_OFFSET + equipment_slot as usize * 32;
        profile_data[profile_offset..profile_offset + 32].copy_from_slice(&identity);

        validate_player_profile_data_and_pda(
            &player_profile,
            &profile_data,
            &owner,
            &player_program,
            &global_config,
        )
        .unwrap();
        validate_player_equipment_data_and_pda(
            &player_equipment,
            &equipment_data,
            &owner,
            &player_profile,
            &player_program,
            &global_config,
        )
        .unwrap();
        assert_eq!(
            validate_equipment_source_record(&profile_data, &equipment_data, equipment_slot)
                .unwrap(),
            source_slot
        );

        let mut non_custodied = equipment_data.clone();
        non_custodied[offset + PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET] = 0;
        assert!(validate_player_equipment_data_and_pda(
            &player_equipment,
            &non_custodied,
            &owner,
            &player_profile,
            &player_program,
            &global_config,
        )
        .is_err());
        assert!(
            validate_equipment_source_record(&profile_data, &non_custodied, equipment_slot,)
                .is_err()
        );

        let mut wrong_version = equipment_data.clone();
        wrong_version[8..10].copy_from_slice(&(PLAYER_EQUIPMENT_VERSION + 1).to_le_bytes());
        assert!(validate_player_equipment_data_and_pda(
            &player_equipment,
            &wrong_version,
            &owner,
            &player_profile,
            &player_program,
            &global_config,
        )
        .is_err());

        let mut wrong_identity = profile_data.clone();
        wrong_identity[profile_offset] ^= 1;
        assert!(
            validate_equipment_source_record(&wrong_identity, &equipment_data, equipment_slot,)
                .is_err()
        );
        assert!(validate_player_equipment_data_and_pda(
            &Pubkey::new_unique(),
            &equipment_data,
            &owner,
            &player_profile,
            &player_program,
            &global_config,
        )
        .is_err());
        assert!(validate_player_equipment_data_and_pda(
            &player_equipment,
            &equipment_data,
            &Pubkey::new_unique(),
            &player_profile,
            &player_program,
            &global_config,
        )
        .is_err());
    }
}
