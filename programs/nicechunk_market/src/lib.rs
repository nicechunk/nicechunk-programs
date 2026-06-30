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

use cluster_config::{MARKET_TREASURY, NCK_MINT, NICECHUNK_BACKPACK_PROGRAM_ID};
use errors::{require_key_eq, NicechunkMarketError};
use state::{
    AssetAccount, AssetInitArgs, BackpackResourceRecord, CreateAssetArgs, CreateListingArgs,
    ListingAccount, ListingInitArgs, ASSET_SEED, LISTING_SEED, MARKET_AUTHORITY_SEED, SOURCE_ASSET,
    SOURCE_BACKPACK,
};

const NCK_DECIMALS: u8 = 6;
const TOKEN_ACCOUNT_MIN_LEN: usize = 165;
const TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;
const BACKPACK_HEADER_LEN: usize = 128;
const BACKPACK_LEGACY_VERSION: u16 = 1;
const BACKPACK_VERSION: u16 = 2;
const BACKPACK_LEGACY_RECORD_LEN: usize = 10;
const BACKPACK_SLOT_RECORD_LEN: usize = 64;
const BACKPACK_OWNER_OFFSET: usize = 20;
const BACKPACK_ITEM_COUNT_OFFSET: usize = 53;
const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
const MARKET_FEE_BPS: u16 = 100;
const BPS_DENOMINATOR: u64 = 10_000;

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
        3 => initialize_asset(program_id, accounts, payload),
        _ => Err(NicechunkMarketError::InvalidInstruction.into()),
    }
}

fn initialize_asset(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let args = CreateAssetArgs::unpack(payload)?;
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let asset = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;

    if !owner.is_signer || !owner.is_writable {
        return Err(NicechunkMarketError::InvalidSeller.into());
    }
    if !asset.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkMarketError::InvalidSystemProgram,
    )?;
    let bump = validate_asset_pda(program_id, asset.key, owner.key, args.asset_id)?;
    if asset.owner == program_id {
        return Err(NicechunkMarketError::AssetAlreadyInitialized.into());
    }
    if asset.owner != &system_program::ID || asset.data_len() != 0 {
        return Err(NicechunkMarketError::InvalidSystemAccount.into());
    }

    create_asset_pda(
        owner,
        asset,
        system_program_account,
        program_id,
        args.asset_id,
        bump,
    )?;

    let clock = Clock::get()?;
    let mut data = asset.try_borrow_mut_data()?;
    AssetAccount::pack(
        &mut data,
        &AssetInitArgs {
            bump,
            owner: owner.key,
            asset_id: args.asset_id,
            category: args.category,
            quantity: args.quantity,
            item_hash: args.item_hash,
            item_code: args.item_code,
            stack_count: args.stack_count,
            durability: args.durability,
            payload_len: args.payload_len,
            payload: args.payload,
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn create_listing(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    let args = CreateListingArgs::unpack(payload)?;
    let expected_accounts = match args.source_kind {
        SOURCE_BACKPACK => 5,
        SOURCE_ASSET => 4,
        _ => return Err(NicechunkMarketError::InvalidSourceKind.into()),
    };
    if accounts.len() != expected_accounts {
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

    let escrow_asset = if args.source_kind == SOURCE_BACKPACK {
        let backpack = next_account_info(account_info_iter)?;
        let backpack_program = next_account_info(account_info_iter)?;
        require_key_eq(
            backpack_program.key,
            &NICECHUNK_BACKPACK_PROGRAM_ID,
            NicechunkMarketError::InvalidBackpackProgram,
        )?;
        require_key_eq(
            backpack.key,
            &args.source_inventory,
            NicechunkMarketError::InvalidEscrowInventory,
        )?;
        validate_backpack_resource(
            backpack,
            seller.key,
            args.source_index,
            &args.resource_record,
        )?;
        remove_backpack_resource(seller, backpack, backpack_program, args.source_index)?;
        None
    } else {
        let asset = next_account_info(account_info_iter)?;
        require_key_eq(
            asset.key,
            &args.source_inventory,
            NicechunkMarketError::InvalidEscrowInventory,
        )?;
        validate_asset_for_listing(
            asset,
            seller.key,
            args.category,
            args.quantity,
            &args.item_hash,
        )?;
        Some(asset)
    };

    create_listing_pda(
        seller,
        listing,
        system_program_account,
        program_id,
        args.listing_id,
        bump,
    )?;

    let clock = Clock::get()?;
    {
        let mut data = listing.try_borrow_mut_data()?;
        ListingAccount::pack(
            &mut data,
            &ListingInitArgs {
                bump,
                seller: seller.key,
                listing_id: args.listing_id,
                category: args.category,
                currency: args.currency,
                source_kind: args.source_kind,
                source_index: args.source_index,
                quantity: args.quantity,
                price_base_units: args.price_base_units,
                item_hash: args.item_hash,
                source_inventory: &args.source_inventory,
                resource_record: args.resource_record,
                created_slot: clock.slot,
                created_at: clock.unix_timestamp,
            },
        )?;
    }
    if let Some(asset) = escrow_asset {
        let mut data = asset.try_borrow_mut_data()?;
        AssetAccount::lock_for_listing(&mut data, listing.key, clock.slot)?;
    }
    Ok(())
}

fn cancel_listing(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 2 && accounts.len() != 3 && accounts.len() != 5 {
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
    let source_kind = ListingAccount::source_kind(&data)?;
    let source_inventory = ListingAccount::source_inventory(&data)?;
    let resource_record = ListingAccount::resource_record(&data)?;
    drop(data);

    validate_listing_pda(program_id, listing.key, seller.key, listing_id)?;

    let clock = Clock::get()?;
    if source_kind == SOURCE_BACKPACK {
        if accounts.len() != 5 {
            return Err(NicechunkMarketError::InvalidAccountCount.into());
        }
        let backpack = next_account_info(account_info_iter)?;
        let backpack_program = next_account_info(account_info_iter)?;
        let market_authority = next_account_info(account_info_iter)?;
        require_key_eq(
            backpack.key,
            &source_inventory,
            NicechunkMarketError::InvalidEscrowInventory,
        )?;
        append_market_resource_to_backpack(
            program_id,
            market_authority,
            seller,
            backpack,
            backpack_program,
            &resource_record,
        )?;
    } else if source_kind == SOURCE_ASSET {
        if accounts.len() != 3 {
            return Err(NicechunkMarketError::InvalidAccountCount.into());
        }
        let asset = next_account_info(account_info_iter)?;
        require_key_eq(
            asset.key,
            &source_inventory,
            NicechunkMarketError::InvalidEscrowInventory,
        )?;
        if !asset.is_writable {
            return Err(NicechunkMarketError::InvalidWritableAccount.into());
        }
        require_key_eq(
            asset.owner,
            program_id,
            NicechunkMarketError::InvalidListingOwner,
        )?;
        let mut asset_data = asset.try_borrow_mut_data()?;
        AssetAccount::unlock_to_owner(&mut asset_data, seller.key, listing.key, clock.slot)?;
    } else if accounts.len() != 2 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }

    {
        let mut data = listing.try_borrow_mut_data()?;
        data[state::ListingAccount::STATE_OFFSET] = state::LISTING_STATE_CANCELED;
        data.fill(0);
    }

    let listing_lamports = listing.lamports();
    **seller.try_borrow_mut_lamports()? = seller
        .lamports()
        .checked_add(listing_lamports)
        .ok_or(NicechunkMarketError::InvalidListingData)?;
    **listing.try_borrow_mut_lamports()? = 0;

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
    let source_kind = ListingAccount::source_kind(&data)?;
    let source_inventory = ListingAccount::source_inventory(&data)?;
    let resource_record = ListingAccount::resource_record(&data)?;
    drop(data);

    let base_account_count = match currency {
        state::CURRENCY_SOL => 5,
        state::CURRENCY_NCK => 8,
        _ => return Err(NicechunkMarketError::UnsupportedCurrency.into()),
    };
    let expected_account_count = base_account_count
        + match source_kind {
            SOURCE_BACKPACK => 3,
            SOURCE_ASSET => 1,
            _ => return Err(NicechunkMarketError::InvalidSourceKind.into()),
        };
    if accounts.len() != expected_account_count {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }

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
            let (seller_amount, fee_amount) = split_market_payment(price_base_units)?;
            if seller_amount > 0 {
                let seller_payment =
                    system_instruction::transfer(buyer.key, seller.key, seller_amount);
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
            let (seller_amount, fee_amount) = split_market_payment(price_base_units)?;
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
        _ => return Err(NicechunkMarketError::UnsupportedCurrency.into()),
    }

    if source_kind == SOURCE_BACKPACK {
        let buyer_backpack = next_account_info(account_info_iter)?;
        let backpack_program = next_account_info(account_info_iter)?;
        let market_authority = next_account_info(account_info_iter)?;
        append_market_resource_to_backpack(
            program_id,
            market_authority,
            buyer,
            buyer_backpack,
            backpack_program,
            &resource_record,
        )?;
    } else if source_kind == SOURCE_ASSET {
        let asset = next_account_info(account_info_iter)?;
        require_key_eq(
            asset.key,
            &source_inventory,
            NicechunkMarketError::InvalidEscrowInventory,
        )?;
        if !asset.is_writable {
            return Err(NicechunkMarketError::InvalidWritableAccount.into());
        }
        require_key_eq(
            asset.owner,
            program_id,
            NicechunkMarketError::InvalidListingOwner,
        )?;
        let clock = Clock::get()?;
        let mut asset_data = asset.try_borrow_mut_data()?;
        AssetAccount::unlock_to_owner(&mut asset_data, buyer.key, listing.key, clock.slot)?;
    }

    let clock = Clock::get()?;
    let mut data = listing.try_borrow_mut_data()?;
    ListingAccount::mark_sold(&mut data, buyer.key, clock.slot, clock.unix_timestamp)
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

fn validate_backpack_resource(
    backpack: &AccountInfo,
    owner: &Pubkey,
    source_index: u16,
    expected_record: &BackpackResourceRecord,
) -> ProgramResult {
    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    let data = backpack.try_borrow_data()?;
    if data.len() < BACKPACK_HEADER_LEN {
        return Err(NicechunkMarketError::InvalidBackpackData.into());
    }
    if &data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32] != owner.as_ref() {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    let item_count = data[BACKPACK_ITEM_COUNT_OFFSET];
    if source_index > u8::MAX as u16 || source_index as u8 >= item_count {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    let record_len = backpack_record_len(&data)?;
    let offset = BACKPACK_HEADER_LEN + source_index as usize * record_len;
    if offset + record_len > data.len() {
        return Err(NicechunkMarketError::InvalidBackpackData.into());
    }
    let actual = backpack_resource_record_at(&data[offset..offset + record_len], record_len)?;
    if actual.world_x != expected_record.world_x
        || actual.world_y != expected_record.world_y
        || actual.world_z != expected_record.world_z
    {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    Ok(())
}

fn backpack_record_len(data: &[u8]) -> Result<usize, solana_program::program_error::ProgramError> {
    if data.len() >= 10 {
        let version = u16::from_le_bytes([data[8], data[9]]);
        if version == BACKPACK_LEGACY_VERSION {
            return Ok(BACKPACK_LEGACY_RECORD_LEN);
        }
        if version == BACKPACK_VERSION {
            return Ok(BACKPACK_SLOT_RECORD_LEN);
        }
    }
    Err(NicechunkMarketError::InvalidBackpackData.into())
}

fn backpack_resource_record_at(
    data: &[u8],
    record_len: usize,
) -> Result<BackpackResourceRecord, solana_program::program_error::ProgramError> {
    if record_len == BACKPACK_LEGACY_RECORD_LEN {
        return BackpackResourceRecord::unpack(data).map_err(|error| error.into());
    }
    if data.len() != BACKPACK_SLOT_RECORD_LEN || data[0] != BACKPACK_SLOT_KIND_BLOCK {
        return Err(NicechunkMarketError::InvalidEscrowInventory.into());
    }
    BackpackResourceRecord::unpack(&data[8..18]).map_err(|error| error.into())
}

fn validate_asset_for_listing(
    asset: &AccountInfo,
    owner: &Pubkey,
    category: u8,
    quantity: u32,
    item_hash: &[u8; 32],
) -> ProgramResult {
    require_key_eq(
        asset.owner,
        &crate::ID,
        NicechunkMarketError::InvalidListingOwner,
    )?;
    if !asset.is_writable {
        return Err(NicechunkMarketError::InvalidWritableAccount.into());
    }
    let data = asset.try_borrow_data()?;
    AssetAccount::validate_active_owner(&data, owner, category, quantity, item_hash)
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
    invoke(&ix, &[seller.clone(), backpack.clone()])
}

fn append_market_resource_to_backpack<'a>(
    program_id: &Pubkey,
    market_authority: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    backpack: &AccountInfo<'a>,
    backpack_program: &AccountInfo<'a>,
    record: &BackpackResourceRecord,
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
    let mut data = Vec::with_capacity(11);
    data.push(3);
    data.extend_from_slice(&record.world_x.to_le_bytes());
    data.extend_from_slice(&record.world_y.to_le_bytes());
    data.extend_from_slice(&record.world_z.to_le_bytes());
    let data = backpack_cpi_data(&data);
    let ix = solana_program::instruction::Instruction {
        program_id: NICECHUNK_BACKPACK_PROGRAM_ID,
        accounts: vec![
            solana_program::instruction::AccountMeta::new_readonly(*market_authority.key, true),
            solana_program::instruction::AccountMeta::new_readonly(*owner.key, false),
            solana_program::instruction::AccountMeta::new(*backpack.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[market_authority.clone(), owner.clone(), backpack.clone()],
        &[&[MARKET_AUTHORITY_SEED, &[bump]]],
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

fn validate_asset_pda(
    program_id: &Pubkey,
    asset: &Pubkey,
    owner: &Pubkey,
    asset_id: u64,
) -> Result<u8, solana_program::program_error::ProgramError> {
    let asset_id_bytes = asset_id.to_le_bytes();
    let (expected_asset, bump) =
        Pubkey::find_program_address(&[ASSET_SEED, owner.as_ref(), &asset_id_bytes], program_id);
    require_key_eq(
        asset,
        &expected_asset,
        NicechunkMarketError::InvalidAssetPda,
    )?;
    Ok(bump)
}

fn create_asset_pda<'a>(
    owner: &AccountInfo<'a>,
    asset: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    program_id: &Pubkey,
    asset_id: u64,
    bump: u8,
) -> ProgramResult {
    let asset_id_bytes = asset_id.to_le_bytes();
    let seeds = &[ASSET_SEED, owner.key.as_ref(), &asset_id_bytes, &[bump]];
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(AssetAccount::LEN);
    let create = system_instruction::create_account(
        owner.key,
        asset.key,
        lamports,
        AssetAccount::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create,
        &[owner.clone(), asset.clone(), system_program_account.clone()],
        &[seeds],
    )
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
