use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkMarketError;

pub const LISTING_MAGIC: [u8; 8] = *b"NCKMKT01";
pub const ASSET_MAGIC: [u8; 8] = *b"NCKAST01";
pub const LISTING_VERSION: u16 = 3;
pub const ASSET_VERSION: u16 = 2;
pub const LISTING_SEED: &[u8] = b"listing";
pub const ASSET_SEED: &[u8] = b"asset";
pub const MARKET_AUTHORITY_SEED: &[u8] = b"market-authority";
pub const LISTING_LEN: usize = 216;
pub const ASSET_PAYLOAD_LEN: usize = 96;
pub const ASSET_LEN: usize = 256;

pub const LISTING_STATE_ACTIVE: u8 = 1;
pub const LISTING_STATE_CANCELED: u8 = 2;
pub const LISTING_STATE_SOLD: u8 = 3;
pub const ASSET_STATE_ACTIVE: u8 = 1;
pub const ASSET_STATE_LISTED: u8 = 2;

pub const CATEGORY_RAW: u8 = 1;
pub const CATEGORY_EQUIPMENT: u8 = 2;
pub const CATEGORY_BUILDING: u8 = 3;
pub const CATEGORY_CLOTHING: u8 = 4;

pub const CURRENCY_NCK: u8 = 1;
pub const CURRENCY_SOL: u8 = 2;

pub const SOURCE_BACKPACK: u8 = 1;
pub const SOURCE_ASSET: u8 = 2;

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
    pub source_inventory: &'a Pubkey,
    pub resource_record: BackpackResourceRecord,
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
    pub const SOURCE_INVENTORY_OFFSET: usize = 101;
    pub const RESOURCE_X_OFFSET: usize = 133;
    pub const RESOURCE_Y_OFFSET: usize = 137;
    pub const RESOURCE_Z_OFFSET: usize = 139;
    pub const CREATED_SLOT_OFFSET: usize = 143;
    pub const UPDATED_SLOT_OFFSET: usize = 151;
    pub const CREATED_AT_OFFSET: usize = 159;
    pub const BUYER_OFFSET: usize = 167;
    pub const SOLD_SLOT_OFFSET: usize = 199;
    pub const SOLD_AT_OFFSET: usize = 207;

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
        writer.pubkey(args.source_inventory)?;
        args.resource_record.pack(&mut writer)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.bytes(&[0_u8; 32])?;
        writer.u64(0)?;
        writer.i64(0)?;
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

    pub fn source_kind(data: &[u8]) -> Result<u8, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        Ok(data[Self::SOURCE_KIND_OFFSET])
    }

    pub fn source_inventory(data: &[u8]) -> Result<Pubkey, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        let mut source_inventory = [0_u8; 32];
        source_inventory.copy_from_slice(
            &data[Self::SOURCE_INVENTORY_OFFSET..Self::SOURCE_INVENTORY_OFFSET + 32],
        );
        Ok(Pubkey::new_from_array(source_inventory))
    }

    pub fn resource_record(data: &[u8]) -> Result<BackpackResourceRecord, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidListingData);
        }
        Ok(BackpackResourceRecord {
            world_x: read_i32(data, Self::RESOURCE_X_OFFSET),
            world_y: read_i16(data, Self::RESOURCE_Y_OFFSET),
            world_z: read_i32(data, Self::RESOURCE_Z_OFFSET),
        })
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

pub struct AssetInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub asset_id: u64,
    pub category: u8,
    pub quantity: u32,
    pub item_hash: [u8; 32],
    pub item_code: u16,
    pub stack_count: u32,
    pub durability: u32,
    pub payload_len: u16,
    pub payload: [u8; ASSET_PAYLOAD_LEN],
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct AssetAccount;

impl AssetAccount {
    pub const LEN: usize = ASSET_LEN;
    pub const STATE_OFFSET: usize = 11;
    pub const OWNER_OFFSET: usize = 12;
    pub const ASSET_ID_OFFSET: usize = 44;
    pub const CATEGORY_OFFSET: usize = 52;
    pub const QUANTITY_OFFSET: usize = 53;
    pub const ITEM_HASH_OFFSET: usize = 57;
    pub const LISTING_OFFSET: usize = 89;
    pub const CREATED_SLOT_OFFSET: usize = 121;
    pub const UPDATED_SLOT_OFFSET: usize = 129;
    pub const CREATED_AT_OFFSET: usize = 137;
    pub const ITEM_CODE_OFFSET: usize = 145;
    pub const STACK_COUNT_OFFSET: usize = 147;
    pub const DURABILITY_OFFSET: usize = 151;
    pub const PAYLOAD_LEN_OFFSET: usize = 155;
    pub const PAYLOAD_OFFSET: usize = 157;

    pub fn pack(dst: &mut [u8], args: &AssetInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }
        validate_category(args.category)?;
        if args.quantity == 0 {
            return Err(NicechunkMarketError::InvalidQuantity.into());
        }
        if args.payload_len as usize > ASSET_PAYLOAD_LEN {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }

        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&ASSET_MAGIC)?;
        writer.u16(ASSET_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(ASSET_STATE_ACTIVE)?;
        writer.pubkey(args.owner)?;
        writer.u64(args.asset_id)?;
        writer.u8(args.category)?;
        writer.u32(args.quantity)?;
        writer.bytes(&args.item_hash)?;
        writer.bytes(&[0_u8; 32])?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.u16(args.item_code)?;
        writer.u32(args.stack_count)?;
        writer.u32(args.durability)?;
        writer.u16(args.payload_len)?;
        writer.bytes(&args.payload)?;
        writer.bytes(&[0_u8; 3])?;
        if writer.offset != Self::LEN {
            return Err(NicechunkMarketError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate(data: &[u8]) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != ASSET_MAGIC {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }
        if read_u16(data, 8) != ASSET_VERSION {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }
        validate_asset_state(data[Self::STATE_OFFSET])?;
        validate_category(data[Self::CATEGORY_OFFSET])?;
        if read_u32(data, Self::QUANTITY_OFFSET) == 0 {
            return Err(NicechunkMarketError::InvalidQuantity.into());
        }
        if read_u16(data, Self::PAYLOAD_LEN_OFFSET) as usize > ASSET_PAYLOAD_LEN {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }
        Ok(())
    }

    pub fn validate_active_owner(
        data: &[u8],
        owner: &Pubkey,
        category: u8,
        quantity: u32,
        item_hash: &[u8; 32],
    ) -> ProgramResult {
        Self::validate(data)?;
        if data[Self::STATE_OFFSET] != ASSET_STATE_ACTIVE {
            return Err(NicechunkMarketError::AssetNotActive.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkMarketError::InvalidAssetOwner.into());
        }
        if data[Self::CATEGORY_OFFSET] != category
            || read_u32(data, Self::QUANTITY_OFFSET) != quantity
            || &data[Self::ITEM_HASH_OFFSET..Self::ITEM_HASH_OFFSET + 32] != item_hash
        {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }
        Ok(())
    }

    pub fn lock_for_listing(data: &mut [u8], listing: &Pubkey, updated_slot: u64) -> ProgramResult {
        Self::validate(data)?;
        if data[Self::STATE_OFFSET] != ASSET_STATE_ACTIVE {
            return Err(NicechunkMarketError::AssetNotActive.into());
        }
        data[Self::STATE_OFFSET] = ASSET_STATE_LISTED;
        data[Self::LISTING_OFFSET..Self::LISTING_OFFSET + 32].copy_from_slice(listing.as_ref());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn unlock_to_owner(
        data: &mut [u8],
        owner: &Pubkey,
        listing: &Pubkey,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate(data)?;
        if data[Self::STATE_OFFSET] != ASSET_STATE_LISTED {
            return Err(NicechunkMarketError::AssetNotListed.into());
        }
        if &data[Self::LISTING_OFFSET..Self::LISTING_OFFSET + 32] != listing.as_ref() {
            return Err(NicechunkMarketError::InvalidAssetData.into());
        }
        data[Self::STATE_OFFSET] = ASSET_STATE_ACTIVE;
        data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32].copy_from_slice(owner.as_ref());
        data[Self::LISTING_OFFSET..Self::LISTING_OFFSET + 32].fill(0);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
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
    pub source_inventory: Pubkey,
    pub resource_record: BackpackResourceRecord,
}

pub struct CreateAssetArgs {
    pub asset_id: u64,
    pub category: u8,
    pub quantity: u32,
    pub item_hash: [u8; 32],
    pub item_code: u16,
    pub stack_count: u32,
    pub durability: u32,
    pub payload_len: u16,
    pub payload: [u8; ASSET_PAYLOAD_LEN],
}

impl CreateAssetArgs {
    pub const LEN: usize = 153;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let mut item_hash = [0_u8; 32];
        item_hash.copy_from_slice(&data[13..45]);
        let payload_len = read_u16(data, 55);
        if payload_len as usize > ASSET_PAYLOAD_LEN {
            return Err(NicechunkMarketError::InvalidAssetData);
        }
        let mut payload = [0_u8; ASSET_PAYLOAD_LEN];
        payload.copy_from_slice(&data[57..153]);
        Ok(Self {
            asset_id: read_u64(data, 0),
            category: data[8],
            quantity: read_u32(data, 9),
            item_hash,
            item_code: read_u16(data, 45),
            stack_count: read_u32(data, 47),
            durability: read_u32(data, 51),
            payload_len,
            payload,
        })
    }
}

impl CreateListingArgs {
    pub const LEN: usize = 99;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        let mut item_hash = [0_u8; 32];
        item_hash.copy_from_slice(&data[25..57]);
        let mut source_inventory = [0_u8; 32];
        source_inventory.copy_from_slice(&data[57..89]);
        Ok(Self {
            listing_id: read_u64(data, 0),
            category: data[8],
            currency: data[9],
            source_kind: data[10],
            source_index: read_u16(data, 11),
            quantity: read_u32(data, 13),
            price_base_units: read_u64(data, 17),
            item_hash,
            source_inventory: Pubkey::new_from_array(source_inventory),
            resource_record: BackpackResourceRecord::unpack(&data[89..99])?,
        })
    }
}

#[derive(Clone, Copy)]
pub struct BackpackResourceRecord {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
}

impl BackpackResourceRecord {
    pub const LEN: usize = 10;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidInstruction);
        }
        Ok(Self {
            world_x: read_i32(data, 0),
            world_y: read_i16(data, 4),
            world_z: read_i32(data, 6),
        })
    }

    fn pack(&self, writer: &mut ByteWriter) -> ProgramResult {
        writer.i32(self.world_x)?;
        writer.i16(self.world_y)?;
        writer.i32(self.world_z)
    }
}

fn validate_category(category: u8) -> Result<(), NicechunkMarketError> {
    match category {
        CATEGORY_RAW | CATEGORY_EQUIPMENT | CATEGORY_BUILDING | CATEGORY_CLOTHING => Ok(()),
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
        SOURCE_BACKPACK | SOURCE_ASSET => Ok(()),
        _ => Err(NicechunkMarketError::InvalidSourceKind),
    }
}

fn validate_asset_state(state: u8) -> Result<(), NicechunkMarketError> {
    match state {
        ASSET_STATE_ACTIVE | ASSET_STATE_LISTED => Ok(()),
        _ => Err(NicechunkMarketError::InvalidAssetData),
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

    fn i16(&mut self, value: i16) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn i32(&mut self, value: i32) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }
}

fn read_i16(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_asset_args_unpack_reads_v2_payload() {
        let mut data = [0_u8; CreateAssetArgs::LEN];
        let mut hash = [0_u8; 32];
        hash[0] = 9;
        hash[31] = 7;
        data[0..8].copy_from_slice(&42_u64.to_le_bytes());
        data[8] = CATEGORY_EQUIPMENT;
        data[9..13].copy_from_slice(&3_u32.to_le_bytes());
        data[13..45].copy_from_slice(&hash);
        data[45..47].copy_from_slice(&8_u16.to_le_bytes());
        data[47..51].copy_from_slice(&1_u32.to_le_bytes());
        data[51..55].copy_from_slice(&777_u32.to_le_bytes());
        data[55..57].copy_from_slice(&4_u16.to_le_bytes());
        data[57..61].copy_from_slice(&[1, 2, 3, 4]);

        let args = CreateAssetArgs::unpack(&data).unwrap();
        assert_eq!(args.asset_id, 42);
        assert_eq!(args.category, CATEGORY_EQUIPMENT);
        assert_eq!(args.quantity, 3);
        assert_eq!(args.item_hash, hash);
        assert_eq!(args.item_code, 8);
        assert_eq!(args.stack_count, 1);
        assert_eq!(args.durability, 777);
        assert_eq!(args.payload_len, 4);
        assert_eq!(&args.payload[..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn create_asset_args_rejects_oversized_payload_len() {
        let mut data = [0_u8; CreateAssetArgs::LEN];
        data[55..57].copy_from_slice(&((ASSET_PAYLOAD_LEN as u16) + 1).to_le_bytes());

        assert!(CreateAssetArgs::unpack(&data).is_err());
    }

    #[test]
    fn asset_account_pack_lock_and_unlock_preserve_metadata() {
        let owner = Pubkey::new_unique();
        let buyer = Pubkey::new_unique();
        let listing = Pubkey::new_unique();
        let mut hash = [0_u8; 32];
        hash[3] = 44;
        let mut payload = [0_u8; ASSET_PAYLOAD_LEN];
        payload[..6].copy_from_slice(b"gravel");
        let mut data = [0_u8; AssetAccount::LEN];

        AssetAccount::pack(
            &mut data,
            &AssetInitArgs {
                bump: 251,
                owner: &owner,
                asset_id: 9001,
                category: CATEGORY_RAW,
                quantity: 5,
                item_hash: hash,
                item_code: 65535,
                stack_count: 5,
                durability: 0,
                payload_len: 6,
                payload,
                created_slot: 123,
                created_at: 456,
            },
        )
        .unwrap();

        assert_eq!(&data[0..8], &ASSET_MAGIC);
        assert_eq!(read_u16(&data, 8), ASSET_VERSION);
        assert_eq!(data[AssetAccount::STATE_OFFSET], ASSET_STATE_ACTIVE);
        assert_eq!(
            &data[AssetAccount::OWNER_OFFSET..AssetAccount::OWNER_OFFSET + 32],
            owner.as_ref()
        );
        assert_eq!(read_u64(&data, AssetAccount::ASSET_ID_OFFSET), 9001);
        assert_eq!(data[AssetAccount::CATEGORY_OFFSET], CATEGORY_RAW);
        assert_eq!(read_u32(&data, AssetAccount::QUANTITY_OFFSET), 5);
        assert_eq!(
            &data[AssetAccount::ITEM_HASH_OFFSET..AssetAccount::ITEM_HASH_OFFSET + 32],
            &hash
        );
        assert_eq!(read_u16(&data, AssetAccount::ITEM_CODE_OFFSET), 65535);
        assert_eq!(read_u32(&data, AssetAccount::STACK_COUNT_OFFSET), 5);
        assert_eq!(read_u32(&data, AssetAccount::DURABILITY_OFFSET), 0);
        assert_eq!(read_u16(&data, AssetAccount::PAYLOAD_LEN_OFFSET), 6);
        assert_eq!(
            &data[AssetAccount::PAYLOAD_OFFSET..AssetAccount::PAYLOAD_OFFSET + 6],
            b"gravel"
        );

        AssetAccount::validate_active_owner(&data, &owner, CATEGORY_RAW, 5, &hash).unwrap();
        AssetAccount::lock_for_listing(&mut data, &listing, 200).unwrap();
        assert_eq!(data[AssetAccount::STATE_OFFSET], ASSET_STATE_LISTED);
        assert_eq!(
            &data[AssetAccount::LISTING_OFFSET..AssetAccount::LISTING_OFFSET + 32],
            listing.as_ref()
        );
        assert!(
            AssetAccount::validate_active_owner(&data, &owner, CATEGORY_RAW, 5, &hash).is_err()
        );

        AssetAccount::unlock_to_owner(&mut data, &buyer, &listing, 300).unwrap();
        assert_eq!(data[AssetAccount::STATE_OFFSET], ASSET_STATE_ACTIVE);
        assert_eq!(
            &data[AssetAccount::OWNER_OFFSET..AssetAccount::OWNER_OFFSET + 32],
            buyer.as_ref()
        );
        assert!(
            data[AssetAccount::LISTING_OFFSET..AssetAccount::LISTING_OFFSET + 32]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(read_u16(&data, AssetAccount::ITEM_CODE_OFFSET), 65535);
        assert_eq!(
            &data[AssetAccount::PAYLOAD_OFFSET..AssetAccount::PAYLOAD_OFFSET + 6],
            b"gravel"
        );
    }

    #[test]
    fn listing_account_mark_sold_sets_buyer_and_timestamps() {
        let seller = Pubkey::new_unique();
        let buyer = Pubkey::new_unique();
        let source_inventory = Pubkey::new_unique();
        let mut data = [0_u8; ListingAccount::LEN];
        let hash = [3_u8; 32];
        ListingAccount::pack(
            &mut data,
            &ListingInitArgs {
                bump: 1,
                seller: &seller,
                listing_id: 7,
                category: CATEGORY_BUILDING,
                currency: CURRENCY_SOL,
                source_kind: SOURCE_ASSET,
                source_index: 2,
                quantity: 1,
                price_base_units: 1_000_000,
                item_hash: hash,
                source_inventory: &source_inventory,
                resource_record: BackpackResourceRecord {
                    world_x: 0,
                    world_y: 0,
                    world_z: 0,
                },
                created_slot: 10,
                created_at: 20,
            },
        )
        .unwrap();

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
