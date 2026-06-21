use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkMarketError;

pub const LISTING_MAGIC: [u8; 8] = *b"NCKMKT01";
pub const LISTING_VERSION: u16 = 2;
pub const LISTING_SEED: &[u8] = b"listing";
pub const LISTING_LEN: usize = 176;

pub const LISTING_STATE_ACTIVE: u8 = 1;
pub const LISTING_STATE_CANCELED: u8 = 2;
pub const LISTING_STATE_SOLD: u8 = 3;

pub const CATEGORY_RAW: u8 = 1;
pub const CATEGORY_EQUIPMENT: u8 = 2;
pub const CATEGORY_BUILDING: u8 = 3;
pub const CATEGORY_CLOTHING: u8 = 4;
pub const CATEGORY_VEGETATION: u8 = 5;

pub const CURRENCY_NCK: u8 = 1;
pub const CURRENCY_SOL: u8 = 2;

pub const SOURCE_BACKPACK: u8 = 1;
pub const SOURCE_HOTBAR: u8 = 2;

pub struct ListingInitArgs<'a> {
    pub bump: u8,
    pub seller: &'a Pubkey,
    pub listing_id: u64,
    pub category: u8,
    pub currency: u8,
    pub source_kind: u8,
    pub source_index: u16,
    pub quantity: u32,
    pub price_base_units: u64,
    pub item_hash: [u8; 32],
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct ListingAccount;

impl ListingAccount {
    pub const LEN: usize = LISTING_LEN;
    pub const STATE_OFFSET: usize = 11;
    pub const SELLER_OFFSET: usize = 12;
    pub const LISTING_ID_OFFSET: usize = 44;
    pub const CATEGORY_OFFSET: usize = 52;
    pub const CURRENCY_OFFSET: usize = 53;
    pub const SOURCE_KIND_OFFSET: usize = 54;
    pub const SOURCE_INDEX_OFFSET: usize = 55;
    pub const QUANTITY_OFFSET: usize = 57;
    pub const PRICE_OFFSET: usize = 61;
    pub const ITEM_HASH_OFFSET: usize = 69;
    pub const CREATED_SLOT_OFFSET: usize = 101;
    pub const UPDATED_SLOT_OFFSET: usize = 109;
    pub const CREATED_AT_OFFSET: usize = 117;
    pub const BUYER_OFFSET: usize = 125;
    pub const SOLD_SLOT_OFFSET: usize = 157;
    pub const SOLD_AT_OFFSET: usize = 165;

    pub fn pack(dst: &mut [u8], args: &ListingInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData.into());
        }
        validate_category(args.category)?;
        validate_currency(args.currency)?;
        validate_source_kind(args.source_kind)?;
        if args.price_base_units == 0 {
            return Err(NicechunkMarketError::InvalidPrice.into());
        }
        if args.quantity == 0 {
            return Err(NicechunkMarketError::InvalidQuantity.into());
        }

        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&LISTING_MAGIC)?;
        writer.u16(LISTING_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(LISTING_STATE_ACTIVE)?;
        writer.pubkey(args.seller)?;
        writer.u64(args.listing_id)?;
        writer.u8(args.category)?;
        writer.u8(args.currency)?;
        writer.u8(args.source_kind)?;
        writer.u16(args.source_index)?;
        writer.u32(args.quantity)?;
        writer.u64(args.price_base_units)?;
        writer.bytes(&args.item_hash)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.bytes(&[0_u8; 32])?;
        writer.u64(0)?;
        writer.i64(0)?;
        writer.bytes(&[0_u8; 3])?;
        if writer.offset != Self::LEN {
            return Err(NicechunkMarketError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_active_seller(data: &[u8], seller: &Pubkey) -> ProgramResult {
        Self::validate_active(data)?;
        if &data[Self::SELLER_OFFSET..Self::SELLER_OFFSET + 32] != seller.as_ref() {
            return Err(NicechunkMarketError::UnauthorizedSeller.into());
        }
        Ok(())
    }

    pub fn validate_active(data: &[u8]) -> ProgramResult {
        Self::validate(data)?;
        if data[Self::STATE_OFFSET] != LISTING_STATE_ACTIVE {
            return Err(NicechunkMarketError::ListingNotActive.into());
        }
        Ok(())
    }

    pub fn validate(data: &[u8]) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != LISTING_MAGIC {
            return Err(NicechunkMarketError::InvalidListingData.into());
        }
        if read_u16(data, 8) != LISTING_VERSION {
            return Err(NicechunkMarketError::InvalidListingData.into());
        }
        validate_category(data[Self::CATEGORY_OFFSET])?;
        validate_currency(data[Self::CURRENCY_OFFSET])?;
        validate_source_kind(data[Self::SOURCE_KIND_OFFSET])?;
        validate_state(data[Self::STATE_OFFSET])?;
        if read_u64(data, Self::PRICE_OFFSET) == 0 {
            return Err(NicechunkMarketError::InvalidPrice.into());
        }
        if read_u32(data, Self::QUANTITY_OFFSET) == 0 {
            return Err(NicechunkMarketError::InvalidQuantity.into());
        }
        Ok(())
    }

    pub fn listing_id(data: &[u8]) -> Result<u64, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        Ok(read_u64(data, Self::LISTING_ID_OFFSET))
    }

    pub fn seller(data: &[u8]) -> Result<Pubkey, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        let mut seller = [0_u8; 32];
        seller.copy_from_slice(&data[Self::SELLER_OFFSET..Self::SELLER_OFFSET + 32]);
        Ok(Pubkey::new_from_array(seller))
    }

    pub fn currency(data: &[u8]) -> Result<u8, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        Ok(data[Self::CURRENCY_OFFSET])
    }

    pub fn price_base_units(data: &[u8]) -> Result<u64, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        Ok(read_u64(data, Self::PRICE_OFFSET))
    }

    pub fn mark_sold(data: &mut [u8], buyer: &Pubkey, sold_slot: u64, sold_at: i64) -> ProgramResult {
        Self::validate_active(data)?;
        data[Self::STATE_OFFSET] = LISTING_STATE_SOLD;
        data[Self::BUYER_OFFSET..Self::BUYER_OFFSET + 32].copy_from_slice(buyer.as_ref());
        data[Self::SOLD_SLOT_OFFSET..Self::SOLD_SLOT_OFFSET + 8].copy_from_slice(&sold_slot.to_le_bytes());
        data[Self::SOLD_AT_OFFSET..Self::SOLD_AT_OFFSET + 8].copy_from_slice(&sold_at.to_le_bytes());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8].copy_from_slice(&sold_slot.to_le_bytes());
        Ok(())
    }
}

pub struct CreateListingArgs {
    pub listing_id: u64,
    pub category: u8,
    pub currency: u8,
    pub source_kind: u8,
    pub source_index: u16,
    pub quantity: u32,
    pub price_base_units: u64,
    pub item_hash: [u8; 32],
}

impl CreateListingArgs {
    pub const LEN: usize = 57;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let mut item_hash = [0_u8; 32];
        item_hash.copy_from_slice(&data[25..57]);
        Ok(Self {
            listing_id: read_u64(data, 0),
            category: data[8],
            currency: data[9],
            source_kind: data[10],
            source_index: read_u16(data, 11),
            quantity: read_u32(data, 13),
            price_base_units: read_u64(data, 17),
            item_hash,
        })
    }
}

fn validate_category(category: u8) -> Result<(), NicechunkMarketError> {
    match category {
        CATEGORY_RAW
        | CATEGORY_EQUIPMENT
        | CATEGORY_BUILDING
        | CATEGORY_CLOTHING
        | CATEGORY_VEGETATION => Ok(()),
        _ => Err(NicechunkMarketError::InvalidCategory),
    }
}

fn validate_currency(currency: u8) -> Result<(), NicechunkMarketError> {
    match currency {
        CURRENCY_NCK | CURRENCY_SOL => Ok(()),
        _ => Err(NicechunkMarketError::InvalidCurrency),
    }
}

fn validate_source_kind(source_kind: u8) -> Result<(), NicechunkMarketError> {
    match source_kind {
        SOURCE_BACKPACK | SOURCE_HOTBAR => Ok(()),
        _ => Err(NicechunkMarketError::InvalidSourceKind),
    }
}

fn validate_state(state: u8) -> Result<(), NicechunkMarketError> {
    match state {
        LISTING_STATE_ACTIVE | LISTING_STATE_CANCELED | LISTING_STATE_SOLD => Ok(()),
        _ => Err(NicechunkMarketError::InvalidListingData),
    }
}

struct ByteWriter<'a> {
    dst: &'a mut [u8],
    offset: usize,
}

impl ByteWriter<'_> {
    fn bytes(&mut self, bytes: &[u8]) -> ProgramResult {
        let end = self.offset + bytes.len();
        if end > self.dst.len() {
            return Err(NicechunkMarketError::PackSizeMismatch.into());
        }
        self.dst[self.offset..end].copy_from_slice(bytes);
        self.offset = end;
        Ok(())
    }

    fn pubkey(&mut self, key: &Pubkey) -> ProgramResult {
        self.bytes(key.as_ref())
    }

    fn u8(&mut self, value: u8) -> ProgramResult {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
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
