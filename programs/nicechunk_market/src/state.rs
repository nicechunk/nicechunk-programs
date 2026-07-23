use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkMarketError;

pub const LISTING_MAGIC: [u8; 8] = *b"NCKMKT01";
pub const LISTING_VERSION: u16 = 5;
pub const LISTING_SEED: &[u8] = b"listing";
pub const MARKET_AUTHORITY_SEED: &[u8] = b"market-authority";
pub const LISTING_LEN: usize = 216;
pub const BACKPACK_SLOT_RECORD_LEN: usize = 80;

pub const LISTING_STATE_ACTIVE: u8 = 1;
pub const LISTING_STATE_CANCELED: u8 = 2;
pub const LISTING_STATE_SOLD: u8 = 3;

pub const CURRENCY_NCK: u8 = 1;
pub const CURRENCY_SOL: u8 = 2;
pub const SOURCE_BACKPACK: u8 = 1;
pub const SOURCE_EQUIPMENT: u8 = 2;

pub struct ListingInitArgs<'a> {
    pub bump: u8,
    pub seller: &'a Pubkey,
    pub listing_id: u64,
    pub currency: u8,
    pub source_type: u8,
    pub source_index: u8,
    pub price_base_units: u64,
    pub source_slot: [u8; BACKPACK_SLOT_RECORD_LEN],
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct ListingAccount;

impl ListingAccount {
    pub const LEN: usize = LISTING_LEN;
    pub const STATE_OFFSET: usize = 11;
    pub const SELLER_OFFSET: usize = 12;
    pub const LISTING_ID_OFFSET: usize = 44;
    pub const CURRENCY_OFFSET: usize = 52;
    pub const SOURCE_INDEX_OFFSET: usize = 53;
    pub const PRICE_OFFSET: usize = 54;
    pub const SOURCE_SLOT_OFFSET: usize = 62;
    pub const CREATED_SLOT_OFFSET: usize = 142;
    pub const UPDATED_SLOT_OFFSET: usize = 150;
    pub const CREATED_AT_OFFSET: usize = 158;
    pub const BUYER_OFFSET: usize = 166;
    pub const SOLD_SLOT_OFFSET: usize = 198;
    pub const SOLD_AT_OFFSET: usize = 206;
    pub const SOURCE_TYPE_OFFSET: usize = 214;

    pub fn pack(dst: &mut [u8], args: &ListingInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData.into());
        }
        validate_currency(args.currency)?;
        validate_source_type(args.source_type)?;
        if args.price_base_units == 0 {
            return Err(NicechunkMarketError::InvalidPrice.into());
        }

        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&LISTING_MAGIC)?;
        writer.u16(LISTING_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(LISTING_STATE_ACTIVE)?;
        writer.pubkey(args.seller)?;
        writer.u64(args.listing_id)?;
        writer.u8(args.currency)?;
        writer.u8(args.source_index)?;
        writer.u64(args.price_base_units)?;
        writer.bytes(&args.source_slot)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.bytes(&[0_u8; 32])?;
        writer.u64(0)?;
        writer.i64(0)?;
        writer.u8(args.source_type)?;
        writer.u8(0)?;
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
        validate_currency(data[Self::CURRENCY_OFFSET])?;
        validate_source_type(data[Self::SOURCE_TYPE_OFFSET])?;
        validate_state(data[Self::STATE_OFFSET])?;
        if read_u64(data, Self::PRICE_OFFSET) == 0 {
            return Err(NicechunkMarketError::InvalidPrice.into());
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

    pub fn source_slot(
        data: &[u8],
    ) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        let mut slot = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        slot.copy_from_slice(
            &data[Self::SOURCE_SLOT_OFFSET..Self::SOURCE_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN],
        );
        Ok(slot)
    }

    pub fn source_type(data: &[u8]) -> Result<u8, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        Ok(data[Self::SOURCE_TYPE_OFFSET])
    }

    pub fn mark_sold(
        data: &mut [u8],
        buyer: &Pubkey,
        sold_slot: u64,
        sold_at: i64,
    ) -> ProgramResult {
        Self::validate_active(data)?;
        data[Self::STATE_OFFSET] = LISTING_STATE_SOLD;
        data[Self::BUYER_OFFSET..Self::BUYER_OFFSET + 32].copy_from_slice(buyer.as_ref());
        data[Self::SOLD_SLOT_OFFSET..Self::SOLD_SLOT_OFFSET + 8]
            .copy_from_slice(&sold_slot.to_le_bytes());
        data[Self::SOLD_AT_OFFSET..Self::SOLD_AT_OFFSET + 8]
            .copy_from_slice(&sold_at.to_le_bytes());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&sold_slot.to_le_bytes());
        Ok(())
    }
}

pub struct CreateListingArgs {
    pub listing_id: u64,
    pub currency: u8,
    pub source_type: u8,
    pub source_index: u8,
    pub price_base_units: u64,
}

impl CreateListingArgs {
    pub const LEN: usize = 19;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        Ok(Self {
            listing_id: read_u64(data, 0),
            currency: data[8],
            source_type: data[9],
            source_index: data[10],
            price_base_units: read_u64(data, 11),
        })
    }
}

fn validate_currency(currency: u8) -> Result<(), NicechunkMarketError> {
    match currency {
        CURRENCY_NCK | CURRENCY_SOL => Ok(()),
        _ => Err(NicechunkMarketError::InvalidCurrency),
    }
}

fn validate_source_type(source_type: u8) -> Result<(), NicechunkMarketError> {
    match source_type {
        SOURCE_BACKPACK | SOURCE_EQUIPMENT => Ok(()),
        _ => Err(NicechunkMarketError::InvalidListingData),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_listing_args_reads_minimal_backpack_payload() {
        let mut data = [0_u8; CreateListingArgs::LEN];
        data[0..8].copy_from_slice(&42_u64.to_le_bytes());
        data[8] = CURRENCY_SOL;
        data[9] = SOURCE_BACKPACK;
        data[10] = 7;
        data[11..19].copy_from_slice(&1_000_000_u64.to_le_bytes());

        let args = CreateListingArgs::unpack(&data).unwrap();

        assert_eq!(args.listing_id, 42);
        assert_eq!(args.currency, CURRENCY_SOL);
        assert_eq!(args.source_type, SOURCE_BACKPACK);
        assert_eq!(args.source_index, 7);
        assert_eq!(args.price_base_units, 1_000_000);
    }

    #[test]
    fn listing_account_stores_source_slot_and_mark_sold_sets_buyer() {
        let seller = Pubkey::new_unique();
        let buyer = Pubkey::new_unique();
        let mut data = [0_u8; ListingAccount::LEN];
        let mut source_slot = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        source_slot[0] = 2;
        source_slot[1] = 2;
        source_slot[4..8].copy_from_slice(&1_u32.to_le_bytes());
        source_slot[18..20].copy_from_slice(&8_u16.to_le_bytes());
        source_slot[64..68].copy_from_slice(&900_u32.to_le_bytes());
        source_slot[68..72].copy_from_slice(&1200_u32.to_le_bytes());
        source_slot[72] = 4;
        source_slot[73] = 18;
        source_slot[74..76].copy_from_slice(&7_500_u16.to_le_bytes());
        ListingAccount::pack(
            &mut data,
            &ListingInitArgs {
                bump: 1,
                seller: &seller,
                listing_id: 7,
                currency: CURRENCY_SOL,
                source_type: SOURCE_EQUIPMENT,
                source_index: 2,
                price_base_units: 1_000_000,
                source_slot,
                created_slot: 10,
                created_at: 20,
            },
        )
        .unwrap();

        assert_eq!(ListingAccount::source_slot(&data).unwrap(), source_slot);
        assert_eq!(
            ListingAccount::source_type(&data).unwrap(),
            SOURCE_EQUIPMENT
        );
        ListingAccount::mark_sold(&mut data, &buyer, 30, 40).unwrap();

        assert_eq!(data[ListingAccount::STATE_OFFSET], LISTING_STATE_SOLD);
        assert_eq!(
            &data[ListingAccount::BUYER_OFFSET..ListingAccount::BUYER_OFFSET + 32],
            buyer.as_ref()
        );
        assert_eq!(read_u64(&data, ListingAccount::SOLD_SLOT_OFFSET), 30);
        assert_eq!(read_u64(&data, ListingAccount::UPDATED_SLOT_OFFSET), 30);
        assert_eq!(
            i64::from_le_bytes(
                data[ListingAccount::SOLD_AT_OFFSET..ListingAccount::SOLD_AT_OFFSET + 8]
                    .try_into()
                    .unwrap()
            ),
            40
        );
    }
}
