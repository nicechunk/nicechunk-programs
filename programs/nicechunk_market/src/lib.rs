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

use cluster_config::{
    MARKET_TREASURY, NCK_MINT, NICECHUNK_BACKPACK_PROGRAM_ID, NICECHUNK_PLAYER_PROGRAM_ID,
};
use errors::{require_key_eq, NicechunkMarketError};
use state::{
    CreateListingArgs, ListingAccount, ListingInitArgs, LISTING_SEED, MARKET_AUTHORITY_SEED,
    SOURCE_BACKPACK, SOURCE_EQUIPMENT,
};

const NCK_DECIMALS: u8 = 6;
const TOKEN_ACCOUNT_MIN_LEN: usize = 165;
const TOKEN_ACCOUNT_MINT_OFFSET: usize = 0;
const TOKEN_ACCOUNT_OWNER_OFFSET: usize = 32;
const BACKPACK_HEADER_LEN: usize = 128;
const BACKPACK_LEN: usize = 8048;
const BACKPACK_VERSION: u16 = 3;
const BACKPACK_SEED: &[u8] = b"backpack";
const BACKPACK_ID_OFFSET: usize = 12;
const BACKPACK_SLOT_RECORD_LEN: usize = 80;
const BACKPACK_OWNER_OFFSET: usize = 20;
const BACKPACK_CAPACITY_OFFSET: usize = 52;
const BACKPACK_ITEM_COUNT_OFFSET: usize = 53;
const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
const BACKPACK_SLOT_KIND_ITEM: u8 = 2;
const BACKPACK_SLOT_ITEM_PDA_OFFSET: usize = 28;
const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
const PLAYER_PROFILE_LEN: usize = 773;
const PLAYER_PROFILE_SEED: &[u8] = b"player-v7";
const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;
const PLAYER_PROFILE_EQUIPMENT_OFFSET: usize = 103;
const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT: usize = 9;
const CLEAR_EQUIPMENT_BACKPACK_INDEX: u8 = u8::MAX;
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
        3 => Err(NicechunkMarketError::InvalidInstruction.into()),
        _ => Err(NicechunkMarketError::InvalidInstruction.into()),
    }
}

fn create_listing(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    let args = CreateListingArgs::unpack(payload)?;
    if args.source_type != SOURCE_BACKPACK && args.source_type != SOURCE_EQUIPMENT {
        return Err(NicechunkMarketError::InvalidInstruction.into());
    }
    if accounts.len() != 8 {
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
    let backpack = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    require_key_eq(
        backpack_program.key,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    let player_profile = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    let player_program = next_account_info(account_info_iter)?;
    require_key_eq(
        player_program.key,
        &NICECHUNK_PLAYER_PROGRAM_ID,
        NicechunkMarketError::InvalidPlayerProgram,
    )?;

    let bump = validate_listing_pda(program_id, listing.key, seller.key, args.listing_id)?;
    if listing.owner == program_id {
        return Err(NicechunkMarketError::ListingAlreadyInitialized.into());
    }
    if listing.owner != &system_program::ID || listing.data_len() != 0 {
        return Err(NicechunkMarketError::InvalidSystemAccount.into());
    }

    let source_slot = match args.source_type {
        SOURCE_BACKPACK => {
            let source_slot =
                read_backpack_slot_for_listing(backpack, seller.key, args.source_index)?;
            remove_backpack_resource(seller, backpack, backpack_program, args.source_index as u16)?;
            if let Some(equipment_slot) = read_matching_equipment_slot_for_listing(
                player_profile,
                seller.key,
                player_program.key,
                global_config.key,
                &source_slot,
            )? {
                clear_player_equipment_slot(
                    seller,
                    player_profile,
                    global_config,
                    player_program,
                    equipment_slot,
                )?;
            }
            source_slot
        }
        SOURCE_EQUIPMENT => {
            let (source_slot, backpack_index) = read_equipment_slot_for_listing(
                player_profile,
                backpack,
                seller.key,
                args.source_index,
                player_program.key,
                global_config.key,
            )?;
            remove_backpack_resource(seller, backpack, backpack_program, backpack_index as u16)?;
            clear_player_equipment_slot(
                seller,
                player_profile,
                global_config,
                player_program,
                args.source_index,
            )?;
            source_slot
        }
        _ => return Err(NicechunkMarketError::InvalidInstruction.into()),
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
    Ok(())
}

fn cancel_listing(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() != 7 {
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
    append_market_slot_to_backpack(
        program_id,
        market_authority,
        seller,
        backpack,
        backpack_program,
        material_physics,
        global_config,
        &source_slot,
    )?;

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
    let source_slot = ListingAccount::source_slot(&data)?;
    drop(data);

    let base_account_count = match currency {
        state::CURRENCY_SOL => 5,
        state::CURRENCY_NCK => 8,
        _ => return Err(NicechunkMarketError::UnsupportedCurrency.into()),
    };
    let expected_account_count = base_account_count + 5;
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

    let buyer_backpack = next_account_info(account_info_iter)?;
    let backpack_program = next_account_info(account_info_iter)?;
    let market_authority = next_account_info(account_info_iter)?;
    let material_physics = next_account_info(account_info_iter)?;
    let global_config = next_account_info(account_info_iter)?;
    append_market_slot_to_backpack(
        program_id,
        market_authority,
        buyer,
        buyer_backpack,
        backpack_program,
        material_physics,
        global_config,
        &source_slot,
    )?;

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
    backpack: &AccountInfo,
    owner: &Pubkey,
    equipment_slot: u8,
    player_program: &Pubkey,
    global_config: &Pubkey,
) -> Result<([u8; BACKPACK_SLOT_RECORD_LEN], u8), solana_program::program_error::ProgramError> {
    require_key_eq(
        player_profile.owner,
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
    let equipment_offset = PLAYER_PROFILE_EQUIPMENT_OFFSET + equipment_slot as usize * 32;
    let equipped_item = Pubkey::new_from_array(
        profile_data[equipment_offset..equipment_offset + 32]
            .try_into()
            .map_err(|_| NicechunkMarketError::InvalidPlayerProfile)?,
    );
    drop(profile_data);
    if equipped_item == Pubkey::default() {
        return Err(NicechunkMarketError::InvalidEquipmentSource.into());
    }

    require_key_eq(
        backpack.owner,
        &NICECHUNK_BACKPACK_PROGRAM_ID,
        NicechunkMarketError::InvalidBackpackProgram,
    )?;
    let backpack_data = backpack.try_borrow_data()?;
    validate_backpack_data_and_pda(backpack.key, &backpack_data, owner)?;
    find_backpack_item_slot_by_pda(&backpack_data, &equipped_item)
}

fn read_matching_equipment_slot_for_listing(
    player_profile: &AccountInfo,
    owner: &Pubkey,
    player_program: &Pubkey,
    global_config: &Pubkey,
    source_slot: &[u8; BACKPACK_SLOT_RECORD_LEN],
) -> Result<Option<u8>, solana_program::program_error::ProgramError> {
    let Some(source_item) = item_pda_from_backpack_slot(source_slot)? else {
        return Ok(None);
    };
    require_key_eq(
        player_profile.owner,
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
    for equipment_slot in 0..PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT {
        let offset = PLAYER_PROFILE_EQUIPMENT_OFFSET + equipment_slot * 32;
        if &profile_data[offset..offset + 32] == source_item.as_ref() {
            return Ok(Some(equipment_slot as u8));
        }
    }
    Ok(None)
}

fn validate_backpack_data_and_pda(backpack: &Pubkey, data: &[u8], owner: &Pubkey) -> ProgramResult {
    if data.len() != BACKPACK_LEN
        || data[0..8] != *b"NCKBPK01"
        || u16::from_le_bytes([data[8], data[9]]) != BACKPACK_VERSION
        || data[11] != 1
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
    if data.len() != PLAYER_PROFILE_LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
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

fn find_backpack_item_slot_by_pda(
    data: &[u8],
    item: &Pubkey,
) -> Result<([u8; BACKPACK_SLOT_RECORD_LEN], u8), solana_program::program_error::ProgramError> {
    let item_count = data[BACKPACK_ITEM_COUNT_OFFSET];
    for index in 0..item_count {
        let offset = BACKPACK_HEADER_LEN + index as usize * BACKPACK_SLOT_RECORD_LEN;
        let slot = &data[offset..offset + BACKPACK_SLOT_RECORD_LEN];
        if slot[0] == BACKPACK_SLOT_KIND_ITEM
            && &slot[BACKPACK_SLOT_ITEM_PDA_OFFSET..BACKPACK_SLOT_ITEM_PDA_OFFSET + 32]
                == item.as_ref()
        {
            return Ok((copy_valid_backpack_slot(slot)?, index));
        }
    }
    Err(NicechunkMarketError::InvalidEquipmentSource.into())
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

fn item_pda_from_backpack_slot(
    slot: &[u8; BACKPACK_SLOT_RECORD_LEN],
) -> Result<Option<Pubkey>, solana_program::program_error::ProgramError> {
    if slot[0] != BACKPACK_SLOT_KIND_ITEM {
        return Ok(None);
    }
    let item = Pubkey::new_from_array(
        slot[BACKPACK_SLOT_ITEM_PDA_OFFSET..BACKPACK_SLOT_ITEM_PDA_OFFSET + 32]
            .try_into()
            .map_err(|_| NicechunkMarketError::InvalidBackpackData)?,
    );
    if item == Pubkey::default() {
        return Ok(None);
    }
    Ok(Some(item))
}

fn clear_player_equipment_slot<'a>(
    seller: &AccountInfo<'a>,
    player_profile: &AccountInfo<'a>,
    global_config: &AccountInfo<'a>,
    player_program: &AccountInfo<'a>,
    equipment_slot: u8,
) -> ProgramResult {
    let ix = solana_program::instruction::Instruction {
        program_id: NICECHUNK_PLAYER_PROGRAM_ID,
        accounts: vec![
            solana_program::instruction::AccountMeta::new_readonly(*seller.key, true),
            solana_program::instruction::AccountMeta::new(*player_profile.key, false),
            solana_program::instruction::AccountMeta::new_readonly(*global_config.key, false),
        ],
        data: vec![2, equipment_slot, CLEAR_EQUIPMENT_BACKPACK_INDEX],
    };
    invoke(
        &ix,
        &[
            seller.clone(),
            player_profile.clone(),
            global_config.clone(),
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
    global_config: &AccountInfo<'a>,
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
            solana_program::instruction::AccountMeta::new_readonly(*global_config.key, false),
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
            global_config.clone(),
            backpack_program.clone(),
        ],
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
