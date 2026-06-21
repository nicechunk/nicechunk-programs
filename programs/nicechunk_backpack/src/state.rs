use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkBackpackError;

pub const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
pub const BACKPACK_VERSION: u16 = 1;
pub const BACKPACK_SEED: &[u8] = b"backpack";
pub const BACKPACK_DEFAULT_CAPACITY: u8 = 50;
pub const BACKPACK_MAX_CAPACITY: u8 = 99;
pub const BACKPACK_HEADER_LEN: usize = 128;
pub const BACKPACK_RECORD_LEN: usize = 10;
pub const BACKPACK_LEN: usize =
    BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY as usize * BACKPACK_RECORD_LEN;
pub const BACKPACK_STATE_CARRIED: u8 = 1;
pub const SESSION_ACTION_BREAK_BLOCK: u8 = 1;

pub const LEGACY_PLAYER_PROFILE_LEN: usize = 417;
pub const PLAYER_PROFILE_LEN: usize = 449;
pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
pub const PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET: usize = 393;

pub const PLAYER_SESSION_LEN: usize = 184;
pub const PLAYER_SESSION_MAGIC: [u8; 8] = *b"NCKSES01";
pub const PLAYER_SESSION_OWNER_OFFSET: usize = 12;
pub const PLAYER_SESSION_AUTHORITY_OFFSET: usize = 44;
pub const PLAYER_SESSION_PROFILE_OFFSET: usize = 76;
pub const PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET: usize = 142;
pub const PLAYER_SESSION_EXPIRES_AT_OFFSET: usize = 144;

pub struct BackpackInitArgs<'a> {
    pub bump: u8,
    pub backpack_id: u64,
    pub owner: &'a Pubkey,
    pub capacity: u8,
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct BackpackAccount;

impl BackpackAccount {
    pub const LEN: usize = BACKPACK_LEN;
    pub const BACKPACK_ID_OFFSET: usize = 12;
    pub const OWNER_OFFSET: usize = 20;
    pub const CAPACITY_OFFSET: usize = 52;
    pub const ITEM_COUNT_OFFSET: usize = 53;
    pub const STATE_OFFSET: usize = 54;
    pub const FLAGS_OFFSET: usize = 55;
    pub const UPDATED_SLOT_OFFSET: usize = 74;
    pub const RECORDS_OFFSET: usize = BACKPACK_HEADER_LEN;

    pub fn pack_empty(dst: &mut [u8], args: &BackpackInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkBackpackError::InvalidBackpackData.into());
        }
        validate_capacity(args.capacity)?;
        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&BACKPACK_MAGIC)?;
        writer.u16(BACKPACK_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.u64(args.backpack_id)?;
        writer.pubkey(args.owner)?;
        writer.u8(args.capacity)?;
        writer.u8(0)?;
        writer.u8(BACKPACK_STATE_CARRIED)?;
        writer.u8(0)?;
        writer.i32(0)?;
        writer.i16(0)?;
        writer.i32(0)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.bytes(&[0_u8; 38])?;
        if writer.offset != BACKPACK_HEADER_LEN {
            return Err(NicechunkBackpackError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate(data: &[u8]) -> Result<(), NicechunkBackpackError> {
        if data.len() != Self::LEN || data[0..8] != BACKPACK_MAGIC {
            return Err(NicechunkBackpackError::InvalidBackpackData);
        }
        if read_u16(data, 8) != BACKPACK_VERSION || data[11] != 1 {
            return Err(NicechunkBackpackError::InvalidBackpackData);
        }
        validate_capacity(data[Self::CAPACITY_OFFSET])?;
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count > data[Self::CAPACITY_OFFSET] {
            return Err(NicechunkBackpackError::InvalidBackpackData);
        }
        Ok(())
    }

    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        Self::validate(data)?;
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkBackpackError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn append_resource(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackResourceRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let capacity = data[Self::CAPACITY_OFFSET];
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count >= capacity {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_RECORD_LEN;
        record.pack(&mut data[offset..offset + BACKPACK_RECORD_LEN])?;
        data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_add(1);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn remove_resource_at(
        data: &mut [u8],
        owner: &Pubkey,
        index: u8,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if index >= item_count {
            return Err(NicechunkBackpackError::InvalidResourceIndex.into());
        }

        let start = Self::RECORDS_OFFSET + index as usize * BACKPACK_RECORD_LEN;
        let end = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_RECORD_LEN;
        if start + BACKPACK_RECORD_LEN < end {
            data.copy_within(start + BACKPACK_RECORD_LEN..end, start);
        }
        let last_start = Self::RECORDS_OFFSET + (item_count - 1) as usize * BACKPACK_RECORD_LEN;
        data[last_start..last_start + BACKPACK_RECORD_LEN].fill(0);
        data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_sub(1);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

pub struct BackpackResourceRecord {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
}

impl BackpackResourceRecord {
    pub const LEN: usize = BACKPACK_RECORD_LEN;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkBackpackError> {
        if data.len() != Self::LEN {
            return Err(NicechunkBackpackError::InvalidInstruction);
        }
        Ok(Self {
            world_x: read_i32(data, 0),
            world_y: read_i16(data, 4),
            world_z: read_i32(data, 6),
        })
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkBackpackError::PackSizeMismatch.into());
        }
        dst[0..4].copy_from_slice(&self.world_x.to_le_bytes());
        dst[4..6].copy_from_slice(&self.world_y.to_le_bytes());
        dst[6..10].copy_from_slice(&self.world_z.to_le_bytes());
        Ok(())
    }
}

pub struct PlayerProfileView;

impl PlayerProfileView {
    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        if !is_supported_player_profile_len(data.len()) || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkBackpackError::InvalidPlayerProfile.into());
        }
        if &data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkBackpackError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn validate_owner_without_bound_backpack(data: &[u8], owner: &Pubkey) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if data.len() == PLAYER_PROFILE_LEN
            && data[PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET
                ..PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET + 32]
                .iter()
                .any(|byte| *byte != 0)
        {
            return Err(NicechunkBackpackError::PlayerBackpackAlreadyBound.into());
        }
        Ok(())
    }
}

fn is_supported_player_profile_len(len: usize) -> bool {
    len == PLAYER_PROFILE_LEN || len == LEGACY_PLAYER_PROFILE_LEN
}

pub struct PlayerSessionView {
    pub owner: Pubkey,
}

impl PlayerSessionView {
    pub fn validate(
        data: &[u8],
        session_authority: &Pubkey,
        player_profile: &Pubkey,
        action: u8,
        now: i64,
    ) -> Result<Self, NicechunkBackpackError> {
        if data.len() != PLAYER_SESSION_LEN || data[0..8] != PLAYER_SESSION_MAGIC {
            return Err(NicechunkBackpackError::InvalidPlayerSession);
        }
        if &data[PLAYER_SESSION_AUTHORITY_OFFSET..PLAYER_SESSION_AUTHORITY_OFFSET + 32]
            != session_authority.as_ref()
        {
            return Err(NicechunkBackpackError::InvalidSessionAuthority);
        }
        if &data[PLAYER_SESSION_PROFILE_OFFSET..PLAYER_SESSION_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkBackpackError::InvalidPlayerProfile);
        }
        let expires_at = read_i64(data, PLAYER_SESSION_EXPIRES_AT_OFFSET);
        if expires_at <= now {
            return Err(NicechunkBackpackError::PlayerSessionExpired);
        }
        let allowed_actions = read_u16(data, PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET);
        if action >= 16 || allowed_actions & (1_u16 << action) == 0 {
            return Err(NicechunkBackpackError::SessionActionNotAllowed);
        }
        Ok(Self {
            owner: Pubkey::new_from_array(
                data[PLAYER_SESSION_OWNER_OFFSET..PLAYER_SESSION_OWNER_OFFSET + 32]
                    .try_into()
                    .map_err(|_| NicechunkBackpackError::InvalidPlayerSession)?,
            ),
        })
    }
}

pub fn validate_capacity(capacity: u8) -> Result<(), NicechunkBackpackError> {
    if !(1..=BACKPACK_MAX_CAPACITY).contains(&capacity) {
        return Err(NicechunkBackpackError::InvalidBackpackCapacity);
    }
    Ok(())
}

struct ByteWriter<'a> {
    dst: &'a mut [u8],
    offset: usize,
}

impl ByteWriter<'_> {
    fn bytes(&mut self, bytes: &[u8]) -> ProgramResult {
        let end = self.offset + bytes.len();
        if end > self.dst.len() {
            return Err(NicechunkBackpackError::PackSizeMismatch.into());
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

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
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

fn read_i64(data: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes([
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
