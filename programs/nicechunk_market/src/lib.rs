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

pub mod errors;
pub mod cluster_config;
pub mod state;

use cluster_config::NCK_MINT;
use errors::{require_key_eq, NicechunkMarketError};
use state::{CreateListingArgs, ListingAccount, ListingInitArgs, LISTING_SEED};

const NCK_DECIMALS: u8 = 6;
const TOKEN_ACCOUNT_MIN_LEN: usize = 165;
const TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;

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
        _ => Err(NicechunkMarketError::InvalidInstruction.into()),
    }
}

fn create_listing(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: &[u8],
) -> ProgramResult {
    if accounts.len() != 3 {
        return Err(NicechunkMarketError::InvalidAccountCount.into());
    }
    let args = CreateListingArgs::unpack(payload)?;

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

    create_listing_pda(
        seller,
        listing,
        system_program_account,
        program_id,
        args.listing_id,
        bump,
    )?;

    let clock = Clock::get()?;
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
            created_slot: clock.slot,
            created_at: clock.unix_timestamp,
        },
    )
}

fn cancel_listing(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 2 {
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
    drop(data);

    validate_listing_pda(program_id, listing.key, seller.key, listing_id)?;

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
    if accounts.len() != 4 && accounts.len() != 7 {
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
    drop(data);

    match currency {
        state::CURRENCY_SOL => {
            if accounts.len() != 4 {
                return Err(NicechunkMarketError::InvalidAccountCount.into());
            }
            let system_program_account = next_account_info(account_info_iter)?;
            require_key_eq(
                system_program_account.key,
                &system_program::ID,
                NicechunkMarketError::InvalidSystemProgram,
            )?;
            let payment = system_instruction::transfer(buyer.key, seller.key, price_base_units);
            invoke(
                &payment,
                &[
                    buyer.clone(),
                    seller.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        state::CURRENCY_NCK => {
            if accounts.len() != 7 {
                return Err(NicechunkMarketError::InvalidAccountCount.into());
            }
            let buyer_nck_token = next_account_info(account_info_iter)?;
            let seller_nck_token = next_account_info(account_info_iter)?;
            let nck_mint = next_account_info(account_info_iter)?;
            let token_program = next_account_info(account_info_iter)?;
            if !buyer_nck_token.is_writable || !seller_nck_token.is_writable {
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
            transfer_nck_to_seller(
                buyer_nck_token,
                seller_nck_token,
                nck_mint,
                buyer,
                token_program,
                price_base_units,
            )?;
        }
        _ => return Err(NicechunkMarketError::UnsupportedCurrency.into()),
    }

    let clock = Clock::get()?;
    let mut data = listing.try_borrow_mut_data()?;
    ListingAccount::mark_sold(&mut data, buyer.key, clock.slot, clock.unix_timestamp)
}

fn transfer_nck_to_seller<'a>(
    buyer_nck_token: &AccountInfo<'a>,
    seller_nck_token: &AccountInfo<'a>,
    nck_mint: &AccountInfo<'a>,
    buyer: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let ix = spl_token::instruction::transfer_checked(
        token_program.key,
        buyer_nck_token.key,
        nck_mint.key,
        seller_nck_token.key,
        buyer.key,
        &[],
        amount,
        NCK_DECIMALS,
    )
    .map_err(|_| NicechunkMarketError::InvalidInstruction)?;
    invoke(
        &ix,
        &[
            buyer_nck_token.clone(),
            nck_mint.clone(),
            seller_nck_token.clone(),
            buyer.clone(),
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
