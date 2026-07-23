use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkPlayerError;

pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_VERSION: u16 = 7;
pub const PLAYER_PROFILE_SEED: &[u8] = b"player-v7";
pub const PLAYER_APPEARANCE_MAGIC: [u8; 8] = *b"NCKAPP01";
pub const PLAYER_APPEARANCE_VERSION: u16 = 1;
pub const PLAYER_APPEARANCE_SEED: &[u8] = b"appearance-v1";
pub const PLAYER_EQUIPMENT_MAGIC: [u8; 8] = *b"NCKEQP01";
pub const PLAYER_EQUIPMENT_VERSION: u16 = 1;
pub const PLAYER_EQUIPMENT_SEED: &[u8] = b"player-equipment-v1";
pub const PLAYER_EQUIPMENT_HEADER_LEN: usize = 128;
pub const PLAYER_EQUIPMENT_SLOT_LEN: usize = 768;
pub const PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES: usize = 640;
pub const PLAYER_EQUIPMENT_FLAG_MODEL: u8 = 1 << 0;
pub const PLAYER_EQUIPMENT_FLAG_CUSTODY: u8 = 1 << 1;
const NCF1_LEGACY_VERSION: u8 = 14;
const NCF1_VERSION: u8 = 15;
pub const EQUIPMENT_TRANSFER_AUTHORITY_SEED: &[u8] = b"equipment-transfer-v1";
pub const APPEARANCE_MODEL_CODE_MAX_BYTES: usize = 2048;
pub const APPEARANCE_TITLE_MAX_BYTES: usize = 96;
pub const APPEARANCE_EQUIPMENT_SLOT_COUNT: usize = 12;
pub const APPEARANCE_EQUIPMENT_CODE_MAX_BYTES: usize = 512;
pub const APPEARANCE_EQUIPMENT_SLOT_LEN: usize = 576;
pub const PLAYER_SESSION_MAGIC: [u8; 8] = *b"NCKSES01";
pub const PLAYER_SESSION_VERSION: u16 = 1;
pub const PLAYER_SESSION_SEED: &[u8] = b"session";
pub const USERNAME_INDEX_MAGIC: [u8; 8] = *b"NCKNAM01";
pub const USERNAME_INDEX_VERSION: u16 = 1;
pub const USERNAME_INDEX_SEED: &[u8] = b"player-name-v1";
pub const USERNAME_INDEX_NAME_MAX_BYTES: usize = 96;
pub const INVITE_INDEX_MAGIC: [u8; 8] = *b"NCKINV01";
pub const INVITE_INDEX_VERSION: u16 = 1;
pub const INVITE_INDEX_SEED: &[u8] = b"invite-index-v1";
pub const INVITE_INDEX_CAPACITY: usize = 64;
pub const INVITE_INDEX_HEADER_LEN: usize = 128;
pub const INVITE_INDEX_RECORD_LEN: usize = 40;
pub const SESSION_ACTION_BREAK_BLOCK: u16 = 1 << 1;
pub const SESSION_ACTION_PLACE_BLOCK: u16 = 1 << 2;
pub const EQUIPMENT_SLOT_COUNT: usize = 9;
pub const DEFAULT_HEALTH: u16 = 100;
pub const DEFAULT_ENERGY: u16 = 100;
pub const DEFAULT_STAMINA: u16 = 100;
pub const DEFAULT_MINING_POWER: u16 = 1;
pub const DEFAULT_BUILD_POWER: u16 = 1;
pub const DEFAULT_DEFENSE: u16 = 0;
pub const DEFAULT_BACKPACK_STYLE: u8 = 0;
pub const PLAYER_NAME_MAX_CHARS: usize = 32;
pub const PLAYER_NAME_MAX_BYTES: usize = 300;
pub const CHARACTER_MODEL_KIND_MALE: u8 = 1;
pub const CHARACTER_MODEL_KIND_FEMALE: u8 = 2;

pub const GLOBAL_CONFIG_LEN: usize = 293;
pub const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
pub const GLOBAL_CONFIG_WORLD_ID_OFFSET: usize = 85;
pub const GLOBAL_CONFIG_MIN_BUILD_Y_OFFSET: usize = 263;
pub const GLOBAL_CONFIG_MAX_BUILD_Y_OFFSET: usize = 265;

pub const BACKPACK_LEN: usize = 8048;
pub const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
pub const BACKPACK_VERSION: u16 = 4;
pub const BACKPACK_SEED: &[u8] = b"backpack";
pub const BACKPACK_HEADER_LEN: usize = 128;
pub const BACKPACK_SLOT_RECORD_LEN: usize = 80;
pub const BACKPACK_ID_OFFSET: usize = 12;
pub const BACKPACK_OWNER_OFFSET: usize = 20;
pub const BACKPACK_ITEM_COUNT_OFFSET: usize = 53;
pub const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
pub const BACKPACK_SLOT_KIND_ITEM: u8 = 2;
pub const BACKPACK_ITEM_CATEGORY_FORGED: u8 = 2;
pub const BACKPACK_FORGED_ITEM_CODE: u16 = 8;
pub const BACKPACK_SLOT_QUANTITY_OFFSET: usize = 4;
pub const BACKPACK_SLOT_ITEM_ID_OFFSET: usize = 20;
pub const BACKPACK_SLOT_ITEM_PDA_OFFSET: usize = 28;
pub const BACKPACK_SLOT_DURABILITY_CURRENT_OFFSET: usize = 64;
pub const BACKPACK_SLOT_DURABILITY_MAX_OFFSET: usize = 68;
pub const BACKPACK_SLOT_GRADE_OFFSET: usize = 72;
pub const BACKPACK_SLOT_ITEM_LEVEL_OFFSET: usize = 73;
pub const BACKPACK_SLOT_QUALITY_BPS_OFFSET: usize = 74;
pub const DURABILITY_BPS_DENOMINATOR: u16 = 10_000;

pub struct GlobalConfigView {
    pub world_id: u16,
    pub min_build_y: i16,
    pub max_build_y: i16,
}

impl GlobalConfigView {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkPlayerError> {
        if data.len() != GLOBAL_CONFIG_LEN {
            return Err(NicechunkPlayerError::InvalidGlobalConfigData);
        }
        if data[0..8] != GLOBAL_CONFIG_MAGIC {
            return Err(NicechunkPlayerError::InvalidGlobalConfigData);
        }
        Ok(Self {
            world_id: read_u16(data, GLOBAL_CONFIG_WORLD_ID_OFFSET),
            min_build_y: read_i16(data, GLOBAL_CONFIG_MIN_BUILD_Y_OFFSET),
            max_build_y: read_i16(data, GLOBAL_CONFIG_MAX_BUILD_Y_OFFSET),
        })
    }
}

/// Public player profile.
///
/// This is intentionally not the player inventory. Equipment is public because
/// nearby players and gameplay systems need to know what is visibly equipped.
/// Backpack contents are kept out of this account; future inventory accounts
/// can be loaded only by the owner workflow when the player opens a backpack.
pub struct PlayerProfile;

impl PlayerProfile {
    pub const LEN: usize = 773;
    pub const OWNER_OFFSET: usize = 12;
    pub const GLOBAL_CONFIG_OFFSET: usize = 44;
    pub const WORLD_ID_OFFSET: usize = 76;
    pub const POSITION_OFFSET: usize = 78;
    pub const EQUIPMENT_OFFSET: usize = 103;
    pub const BACKPACK_STYLE_OFFSET: usize = 391;
    pub const BACKPACK_FLAGS_OFFSET: usize = 392;
    pub const EQUIPPED_BACKPACK_OFFSET: usize = 393;
    pub const CREATED_SLOT_OFFSET: usize = 425;
    pub const UPDATED_SLOT_OFFSET: usize = 433;
    pub const CREATED_AT_OFFSET: usize = 441;
    pub const FORGING_XP_OFFSET: usize = 449;
    pub const FORGED_ITEM_COUNT_OFFSET: usize = 457;
    pub const BEST_FORGED_GRADE_OFFSET: usize = 461;
    pub const BEST_FORGED_ITEM_LEVEL_OFFSET: usize = 462;
    pub const NAME_LENGTH_OFFSET: usize = 463;
    pub const NAME_BYTES_OFFSET: usize = 465;
    pub const RESERVED_OFFSET: usize = 765;

    pub fn pack_default(
        dst: &mut [u8],
        bump: u8,
        owner: &Pubkey,
        global_config: &Pubkey,
        world_id: u16,
        player_name: &str,
        created_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        Self::validate_name(player_name.as_bytes())?;

        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&PLAYER_PROFILE_MAGIC)?;
        writer.u16(PLAYER_PROFILE_VERSION)?;
        writer.u8(bump)?;
        writer.u8(1)?;
        writer.pubkey(owner)?;
        writer.pubkey(global_config)?;
        writer.u16(world_id)?;
        writer.i32(0)?;
        writer.i32(0)?;
        writer.i32(0)?;
        writer.u16(DEFAULT_HEALTH)?;
        writer.u16(DEFAULT_ENERGY)?;
        writer.u16(DEFAULT_STAMINA)?;
        writer.u16(DEFAULT_MINING_POWER)?;
        writer.u16(DEFAULT_BUILD_POWER)?;
        writer.u16(DEFAULT_DEFENSE)?;
        writer.u8(EQUIPMENT_SLOT_COUNT as u8)?;
        for _ in 0..EQUIPMENT_SLOT_COUNT {
            writer.pubkey(&Pubkey::default())?;
        }
        writer.u8(DEFAULT_BACKPACK_STYLE)?;
        writer.u8(0)?;
        writer.pubkey(&Pubkey::default())?;
        writer.u64(created_slot)?;
        writer.u64(created_slot)?;
        writer.i64(created_at)?;
        writer.u64(0)?;
        writer.u32(0)?;
        writer.u8(0)?;
        writer.u8(0)?;
        let player_name_bytes = player_name.as_bytes();
        writer.u16(player_name_bytes.len() as u16)?;
        writer.bytes(player_name_bytes)?;
        writer.bytes(
            &[0_u8; PLAYER_NAME_MAX_BYTES][..PLAYER_NAME_MAX_BYTES - player_name_bytes.len()],
        )?;
        writer.bytes(&[0_u8; 8])?;

        if writer.offset != Self::LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_name(name: &[u8]) -> Result<&str, NicechunkPlayerError> {
        if name.len() > PLAYER_NAME_MAX_BYTES {
            return Err(NicechunkPlayerError::InvalidPlayerName);
        }
        let value =
            core::str::from_utf8(name).map_err(|_| NicechunkPlayerError::InvalidPlayerName)?;
        let mut char_count = 0usize;
        for ch in value.chars() {
            char_count += 1;
            if char_count > PLAYER_NAME_MAX_CHARS {
                return Err(NicechunkPlayerError::InvalidPlayerName);
            }
            let valid =
                ch == '_' || ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(&ch);
            if !valid {
                return Err(NicechunkPlayerError::InvalidPlayerName);
            }
        }
        Ok(value)
    }

    pub fn validate_owner_and_config(
        data: &[u8],
        owner: &Pubkey,
        global_config: &Pubkey,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidGlobalConfig.into());
        }
        Ok(())
    }

    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
        }
        Ok(())
    }

    pub fn has_equipped_backpack(data: &[u8]) -> Result<bool, NicechunkPlayerError> {
        if data.len() != Self::LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData);
        }
        Ok(
            data[Self::EQUIPPED_BACKPACK_OFFSET..Self::EQUIPPED_BACKPACK_OFFSET + 32]
                .iter()
                .any(|byte| *byte != 0),
        )
    }

    pub fn write_position(
        dst: &mut [u8],
        x: i32,
        y: i32,
        z: i32,
        updated_slot: u64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        dst[Self::POSITION_OFFSET..Self::POSITION_OFFSET + 4].copy_from_slice(&x.to_le_bytes());
        dst[Self::POSITION_OFFSET + 4..Self::POSITION_OFFSET + 8].copy_from_slice(&y.to_le_bytes());
        dst[Self::POSITION_OFFSET + 8..Self::POSITION_OFFSET + 12]
            .copy_from_slice(&z.to_le_bytes());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn write_equipment_slot(
        dst: &mut [u8],
        slot: u8,
        item: &Pubkey,
        updated_slot: u64,
    ) -> ProgramResult {
        if slot as usize >= EQUIPMENT_SLOT_COUNT {
            return Err(NicechunkPlayerError::InvalidEquipmentSlot.into());
        }
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        let offset = Self::EQUIPMENT_OFFSET + slot as usize * 32;
        dst[offset..offset + 32].copy_from_slice(item.as_ref());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn write_backpack_style(
        dst: &mut [u8],
        backpack_style: u8,
        updated_slot: u64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        dst[Self::BACKPACK_STYLE_OFFSET] = backpack_style;
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn write_equipped_backpack(
        dst: &mut [u8],
        backpack: &Pubkey,
        updated_slot: u64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        dst[Self::EQUIPPED_BACKPACK_OFFSET..Self::EQUIPPED_BACKPACK_OFFSET + 32]
            .copy_from_slice(backpack.as_ref());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn write_name(dst: &mut [u8], player_name: &str, updated_slot: u64) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        Self::validate_name(player_name.as_bytes())?;
        dst[8..10].copy_from_slice(&PLAYER_PROFILE_VERSION.to_le_bytes());
        dst[Self::NAME_LENGTH_OFFSET..Self::NAME_LENGTH_OFFSET + 2]
            .copy_from_slice(&(player_name.as_bytes().len() as u16).to_le_bytes());
        dst[Self::NAME_BYTES_OFFSET..Self::NAME_BYTES_OFFSET + PLAYER_NAME_MAX_BYTES].fill(0);
        dst[Self::NAME_BYTES_OFFSET..Self::NAME_BYTES_OFFSET + player_name.as_bytes().len()]
            .copy_from_slice(player_name.as_bytes());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn add_forging_result(
        dst: &mut [u8],
        owner: &Pubkey,
        gained_xp: u64,
        grade: u8,
        item_level: u8,
        updated_slot: u64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        Self::validate_owner(dst, owner)?;
        let next_xp = read_u64(dst, Self::FORGING_XP_OFFSET).saturating_add(gained_xp);
        let next_count = read_u32(dst, Self::FORGED_ITEM_COUNT_OFFSET).saturating_add(1);
        dst[8..10].copy_from_slice(&PLAYER_PROFILE_VERSION.to_le_bytes());
        dst[Self::FORGING_XP_OFFSET..Self::FORGING_XP_OFFSET + 8]
            .copy_from_slice(&next_xp.to_le_bytes());
        dst[Self::FORGED_ITEM_COUNT_OFFSET..Self::FORGED_ITEM_COUNT_OFFSET + 4]
            .copy_from_slice(&next_count.to_le_bytes());
        dst[Self::BEST_FORGED_GRADE_OFFSET] = dst[Self::BEST_FORGED_GRADE_OFFSET].max(grade);
        dst[Self::BEST_FORGED_ITEM_LEVEL_OFFSET] =
            dst[Self::BEST_FORGED_ITEM_LEVEL_OFFSET].max(item_level);
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

pub struct PlayerEquipmentInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub player_profile: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub created_slot: u64,
}

/// Public, authoritative hotbar equipment state.
///
/// Each record owns the exact item moved out of Backpack and, for forged items,
/// the verified NCF1 model bytes needed by a fresh client to render it. Legacy
/// records without the custody flag remain readable for one-time migration.
pub struct PlayerEquipment;

impl PlayerEquipment {
    pub const LEN: usize =
        PLAYER_EQUIPMENT_HEADER_LEN + EQUIPMENT_SLOT_COUNT * PLAYER_EQUIPMENT_SLOT_LEN;
    pub const OWNER_OFFSET: usize = 12;
    pub const PLAYER_PROFILE_OFFSET: usize = 44;
    pub const GLOBAL_CONFIG_OFFSET: usize = 76;
    pub const SLOT_COUNT_OFFSET: usize = 108;
    pub const CREATED_SLOT_OFFSET: usize = 112;
    pub const UPDATED_SLOT_OFFSET: usize = 120;
    pub const SLOTS_OFFSET: usize = PLAYER_EQUIPMENT_HEADER_LEN;

    pub const RECORD_STATE_OFFSET: usize = 0;
    pub const RECORD_SLOT_OFFSET: usize = 1;
    pub const RECORD_BACKPACK_INDEX_OFFSET: usize = 2;
    pub const RECORD_FLAGS_OFFSET: usize = 3;
    pub const RECORD_MODEL_LENGTH_OFFSET: usize = 4;
    pub const RECORD_BACKPACK_OFFSET: usize = 8;
    pub const RECORD_BACKPACK_SLOT_OFFSET: usize = 40;
    pub const RECORD_MODEL_CODE_OFFSET: usize = 120;

    pub fn pack_empty(dst: &mut [u8], args: &PlayerEquipmentInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentData.into());
        }
        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&PLAYER_EQUIPMENT_MAGIC)?;
        writer.u16(PLAYER_EQUIPMENT_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.pubkey(args.owner)?;
        writer.pubkey(args.player_profile)?;
        writer.pubkey(args.global_config)?;
        writer.u8(EQUIPMENT_SLOT_COUNT as u8)?;
        writer.bytes(&[0_u8; 3])?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        if writer.offset != PLAYER_EQUIPMENT_HEADER_LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        for slot in 0..EQUIPMENT_SLOT_COUNT {
            let offset = Self::slot_offset(slot as u8)?;
            writer.dst[offset + Self::RECORD_SLOT_OFFSET] = slot as u8;
            writer.dst[offset + Self::RECORD_BACKPACK_INDEX_OFFSET] = u8::MAX;
        }
        Ok(())
    }

    pub fn validate_owner_and_config(
        data: &[u8],
        owner: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
    ) -> ProgramResult {
        if data.len() != Self::LEN
            || data[0..8] != PLAYER_EQUIPMENT_MAGIC
            || read_u16(data, 8) != PLAYER_EQUIPMENT_VERSION
            || data[Self::SLOT_COUNT_OFFSET] as usize != EQUIPMENT_SLOT_COUNT
        {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentData.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
        }
        if &data[Self::PLAYER_PROFILE_OFFSET..Self::PLAYER_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidGlobalConfig.into());
        }
        Ok(())
    }

    pub fn clear_slot(dst: &mut [u8], slot: u8, updated_slot: u64) -> ProgramResult {
        let offset = Self::slot_offset(slot)?;
        if dst.len() != Self::LEN || dst[0..8] != PLAYER_EQUIPMENT_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentData.into());
        }
        dst[offset..offset + PLAYER_EQUIPMENT_SLOT_LEN].fill(0);
        dst[offset + Self::RECORD_SLOT_OFFSET] = slot;
        dst[offset + Self::RECORD_BACKPACK_INDEX_OFFSET] = u8::MAX;
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn write_slot(
        dst: &mut [u8],
        slot: u8,
        backpack_index: u8,
        backpack: &Pubkey,
        backpack_record: &[u8; BACKPACK_SLOT_RECORD_LEN],
        model_code: &[u8],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::write_slot_internal(
            dst,
            slot,
            backpack_index,
            backpack,
            backpack_record,
            model_code,
            updated_slot,
            false,
        )
    }

    pub fn write_custodied_slot(
        dst: &mut [u8],
        slot: u8,
        backpack_index: u8,
        backpack: &Pubkey,
        backpack_record: &[u8; BACKPACK_SLOT_RECORD_LEN],
        model_code: &[u8],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::write_slot_internal(
            dst,
            slot,
            backpack_index,
            backpack,
            backpack_record,
            model_code,
            updated_slot,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write_slot_internal(
        dst: &mut [u8],
        slot: u8,
        backpack_index: u8,
        backpack: &Pubkey,
        backpack_record: &[u8; BACKPACK_SLOT_RECORD_LEN],
        model_code: &[u8],
        updated_slot: u64,
        custodied: bool,
    ) -> ProgramResult {
        let offset = Self::slot_offset(slot)?;
        if dst.len() != Self::LEN || dst[0..8] != PLAYER_EQUIPMENT_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentData.into());
        }
        Self::validate_model_code(backpack_record, model_code)?;

        let mut preserved_code = [0_u8; PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES];
        let preserved_len = if model_code.is_empty() {
            Self::find_model_code(dst, backpack, backpack_record, &mut preserved_code)
        } else {
            0
        };
        let effective_code = if model_code.is_empty() && preserved_len > 0 {
            &preserved_code[..preserved_len]
        } else {
            model_code
        };

        dst[offset..offset + PLAYER_EQUIPMENT_SLOT_LEN].fill(0);
        dst[offset + Self::RECORD_STATE_OFFSET] = 1;
        dst[offset + Self::RECORD_SLOT_OFFSET] = slot;
        dst[offset + Self::RECORD_BACKPACK_INDEX_OFFSET] = backpack_index;
        dst[offset + Self::RECORD_FLAGS_OFFSET] = if effective_code.is_empty() {
            0
        } else {
            PLAYER_EQUIPMENT_FLAG_MODEL
        } | if custodied {
            PLAYER_EQUIPMENT_FLAG_CUSTODY
        } else {
            0
        };
        dst[offset + Self::RECORD_MODEL_LENGTH_OFFSET
            ..offset + Self::RECORD_MODEL_LENGTH_OFFSET + 2]
            .copy_from_slice(&(effective_code.len() as u16).to_le_bytes());
        dst[offset + Self::RECORD_BACKPACK_OFFSET..offset + Self::RECORD_BACKPACK_OFFSET + 32]
            .copy_from_slice(backpack.as_ref());
        dst[offset + Self::RECORD_BACKPACK_SLOT_OFFSET
            ..offset + Self::RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN]
            .copy_from_slice(backpack_record);
        dst[offset + Self::RECORD_MODEL_CODE_OFFSET
            ..offset + Self::RECORD_MODEL_CODE_OFFSET + effective_code.len()]
            .copy_from_slice(effective_code);
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn slot_is_equipped(data: &[u8], slot: u8) -> Result<bool, NicechunkPlayerError> {
        let offset =
            Self::slot_offset(slot).map_err(|_| NicechunkPlayerError::InvalidEquipmentSlot)?;
        Self::validate_layout(data)?;
        Ok(data[offset + Self::RECORD_STATE_OFFSET] == 1)
    }

    pub fn slot_is_custodied(data: &[u8], slot: u8) -> Result<bool, NicechunkPlayerError> {
        let offset =
            Self::slot_offset(slot).map_err(|_| NicechunkPlayerError::InvalidEquipmentSlot)?;
        Self::validate_layout(data)?;
        Ok(data[offset + Self::RECORD_STATE_OFFSET] == 1
            && data[offset + Self::RECORD_FLAGS_OFFSET] & PLAYER_EQUIPMENT_FLAG_CUSTODY != 0)
    }

    pub fn slot_record(
        data: &[u8],
        slot: u8,
    ) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], NicechunkPlayerError> {
        let offset =
            Self::slot_offset(slot).map_err(|_| NicechunkPlayerError::InvalidEquipmentSlot)?;
        Self::validate_layout(data)?;
        if data[offset + Self::RECORD_STATE_OFFSET] != 1 {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        data[offset + Self::RECORD_BACKPACK_SLOT_OFFSET
            ..offset + Self::RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN]
            .try_into()
            .map_err(|_| NicechunkPlayerError::InvalidPlayerEquipmentData)
    }

    pub fn slot_identity(data: &[u8], slot: u8) -> Result<Pubkey, NicechunkPlayerError> {
        let offset =
            Self::slot_offset(slot).map_err(|_| NicechunkPlayerError::InvalidEquipmentSlot)?;
        Self::validate_layout(data)?;
        if data[offset + Self::RECORD_STATE_OFFSET] != 1 {
            return Ok(Pubkey::default());
        }
        Ok(Pubkey::new_from_array(
            solana_program::hash::hashv(&[
                b"equipment-v2",
                &data[offset + Self::RECORD_BACKPACK_OFFSET
                    ..offset + Self::RECORD_BACKPACK_OFFSET + 32],
                &data[offset + Self::RECORD_BACKPACK_SLOT_OFFSET
                    ..offset + Self::RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN],
            ])
            .to_bytes(),
        ))
    }

    pub fn consume_forged_durability(
        dst: &mut [u8],
        slot: u8,
        amount: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        if amount == 0 {
            return Err(NicechunkPlayerError::InvalidDurabilityAmount.into());
        }
        let offset = Self::slot_offset(slot)?;
        Self::validate_layout(dst)?;
        let record_offset = offset + Self::RECORD_BACKPACK_SLOT_OFFSET;
        if dst[offset + Self::RECORD_STATE_OFFSET] != 1
            || dst[offset + Self::RECORD_FLAGS_OFFSET] & PLAYER_EQUIPMENT_FLAG_CUSTODY == 0
            || dst[record_offset] != BACKPACK_SLOT_KIND_ITEM
            || dst[record_offset + 1] != BACKPACK_ITEM_CATEGORY_FORGED
            || read_u16(dst, record_offset + 18) != BACKPACK_FORGED_ITEM_CODE
        {
            return Err(NicechunkPlayerError::EquipmentNotCustodied.into());
        }
        let durability_offset = record_offset + BACKPACK_SLOT_DURABILITY_CURRENT_OFFSET;
        let max_offset = record_offset + BACKPACK_SLOT_DURABILITY_MAX_OFFSET;
        let current = read_u32(dst, durability_offset);
        let maximum = read_u32(dst, max_offset);
        if current == 0 || maximum == 0 || current > maximum || amount > current {
            return Err(NicechunkPlayerError::EquipmentBroken.into());
        }
        dst[durability_offset..durability_offset + 4]
            .copy_from_slice(&(current - amount).to_le_bytes());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn swap_slots(
        dst: &mut [u8],
        from_slot: u8,
        to_slot: u8,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_layout(dst)?;
        if from_slot == to_slot {
            return Err(NicechunkPlayerError::InvalidEquipmentSlot.into());
        }
        let from_offset = Self::slot_offset(from_slot)?;
        let to_offset = Self::slot_offset(to_slot)?;
        let mut from = [0_u8; PLAYER_EQUIPMENT_SLOT_LEN];
        let mut to = [0_u8; PLAYER_EQUIPMENT_SLOT_LEN];
        from.copy_from_slice(&dst[from_offset..from_offset + PLAYER_EQUIPMENT_SLOT_LEN]);
        to.copy_from_slice(&dst[to_offset..to_offset + PLAYER_EQUIPMENT_SLOT_LEN]);
        dst[from_offset..from_offset + PLAYER_EQUIPMENT_SLOT_LEN].copy_from_slice(&to);
        dst[to_offset..to_offset + PLAYER_EQUIPMENT_SLOT_LEN].copy_from_slice(&from);
        dst[from_offset + Self::RECORD_SLOT_OFFSET] = from_slot;
        dst[to_offset + Self::RECORD_SLOT_OFFSET] = to_slot;
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    fn validate_layout(data: &[u8]) -> Result<(), NicechunkPlayerError> {
        if data.len() != Self::LEN
            || data[0..8] != PLAYER_EQUIPMENT_MAGIC
            || read_u16(data, 8) != PLAYER_EQUIPMENT_VERSION
            || data[Self::SLOT_COUNT_OFFSET] as usize != EQUIPMENT_SLOT_COUNT
        {
            return Err(NicechunkPlayerError::InvalidPlayerEquipmentData);
        }
        Ok(())
    }

    fn slot_offset(slot: u8) -> Result<usize, solana_program::program_error::ProgramError> {
        if slot as usize >= EQUIPMENT_SLOT_COUNT {
            return Err(NicechunkPlayerError::InvalidEquipmentSlot.into());
        }
        Ok(Self::SLOTS_OFFSET + slot as usize * PLAYER_EQUIPMENT_SLOT_LEN)
    }

    fn validate_model_code(
        backpack_record: &[u8; BACKPACK_SLOT_RECORD_LEN],
        model_code: &[u8],
    ) -> ProgramResult {
        if model_code.len() > PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES {
            return Err(NicechunkPlayerError::InvalidEquipmentModel.into());
        }
        let forged = backpack_record[0] == BACKPACK_SLOT_KIND_ITEM
            && backpack_record[1] == BACKPACK_ITEM_CATEGORY_FORGED
            && read_u16(backpack_record, 18) == BACKPACK_FORGED_ITEM_CODE;
        if model_code.is_empty() {
            return Ok(());
        }
        let model_version = model_code[0] >> 4;
        if !forged
            || model_code.len() < 14
            || (model_version != NCF1_LEGACY_VERSION && model_version != NCF1_VERSION)
            || fnv1a32(model_code) != read_u32(backpack_record, 76)
        {
            return Err(NicechunkPlayerError::InvalidEquipmentModel.into());
        }
        Ok(())
    }

    fn find_model_code(
        data: &[u8],
        backpack: &Pubkey,
        backpack_record: &[u8; BACKPACK_SLOT_RECORD_LEN],
        out: &mut [u8; PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES],
    ) -> usize {
        for slot in 0..EQUIPMENT_SLOT_COUNT {
            let offset = Self::SLOTS_OFFSET + slot * PLAYER_EQUIPMENT_SLOT_LEN;
            if data[offset + Self::RECORD_STATE_OFFSET] != 1
                || &data[offset + Self::RECORD_BACKPACK_OFFSET
                    ..offset + Self::RECORD_BACKPACK_OFFSET + 32]
                    != backpack.as_ref()
            {
                continue;
            }
            let stored_record = &data[offset + Self::RECORD_BACKPACK_SLOT_OFFSET
                ..offset + Self::RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN];
            if !same_backpack_item(stored_record, backpack_record) {
                continue;
            }
            let len = read_u16(data, offset + Self::RECORD_MODEL_LENGTH_OFFSET) as usize;
            if len == 0 || len > PLAYER_EQUIPMENT_MODEL_CODE_MAX_BYTES {
                return 0;
            }
            out[..len].copy_from_slice(
                &data[offset + Self::RECORD_MODEL_CODE_OFFSET
                    ..offset + Self::RECORD_MODEL_CODE_OFFSET + len],
            );
            return len;
        }
        0
    }
}

pub struct PlayerAppearanceInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub player_profile: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub treasury_authority: &'a Pubkey,
    pub model_kind: u8,
    pub display_name: &'a str,
    pub title: &'a [u8],
    pub model_code: &'a [u8],
    pub created_slot: u64,
    pub updated_slot: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Public on-chain appearance.
///
/// The wallet funds the account rent, but the close/reclaim authority is the
/// protocol treasury. Nearby clients can load this PDA once to render the
/// player name, title, body model and visible equipment without reading the
/// private backpack workflow.
pub struct PlayerAppearance;

impl PlayerAppearance {
    pub const HEADER_LEN: usize = 256;
    pub const DISPLAY_NAME_OFFSET: usize = Self::HEADER_LEN;
    pub const TITLE_OFFSET: usize = Self::DISPLAY_NAME_OFFSET + PLAYER_NAME_MAX_BYTES;
    pub const MODEL_CODE_OFFSET: usize = Self::TITLE_OFFSET + APPEARANCE_TITLE_MAX_BYTES;
    pub const EQUIPMENT_OFFSET: usize = Self::MODEL_CODE_OFFSET + APPEARANCE_MODEL_CODE_MAX_BYTES;
    pub const LEN: usize =
        Self::EQUIPMENT_OFFSET + APPEARANCE_EQUIPMENT_SLOT_COUNT * APPEARANCE_EQUIPMENT_SLOT_LEN;
    pub const OWNER_OFFSET: usize = 12;
    pub const PLAYER_PROFILE_OFFSET: usize = 44;
    pub const GLOBAL_CONFIG_OFFSET: usize = 76;
    pub const TREASURY_AUTHORITY_OFFSET: usize = 108;
    pub const MODEL_KIND_OFFSET: usize = 140;
    pub const FLAGS_OFFSET: usize = 141;
    pub const DISPLAY_NAME_LENGTH_OFFSET: usize = 143;
    pub const TITLE_LENGTH_OFFSET: usize = 145;
    pub const MODEL_CODE_LENGTH_OFFSET: usize = 147;
    pub const EQUIPMENT_SLOT_COUNT_OFFSET: usize = 149;
    pub const CREATED_SLOT_OFFSET: usize = 150;
    pub const UPDATED_SLOT_OFFSET: usize = 158;
    pub const CREATED_AT_OFFSET: usize = 166;
    pub const UPDATED_AT_OFFSET: usize = 174;
    pub const RESERVED_OFFSET: usize = 182;

    pub fn pack(dst: &mut [u8], args: &PlayerAppearanceInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidAppearanceData.into());
        }
        Self::validate_model_kind(args.model_kind)?;
        PlayerProfile::validate_name(args.display_name.as_bytes())?;
        Self::validate_title(args.title)?;
        Self::validate_model_code(args.model_code)?;

        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&PLAYER_APPEARANCE_MAGIC)?;
        writer.u16(PLAYER_APPEARANCE_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.pubkey(args.owner)?;
        writer.pubkey(args.player_profile)?;
        writer.pubkey(args.global_config)?;
        writer.pubkey(args.treasury_authority)?;
        writer.u8(args.model_kind)?;
        writer.u16(0)?;
        writer.u16(args.display_name.as_bytes().len() as u16)?;
        writer.u16(args.title.len() as u16)?;
        writer.u16(args.model_code.len() as u16)?;
        writer.u8(APPEARANCE_EQUIPMENT_SLOT_COUNT as u8)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.updated_slot)?;
        writer.i64(args.created_at)?;
        writer.i64(args.updated_at)?;
        writer.bytes(&[0_u8; Self::HEADER_LEN - Self::RESERVED_OFFSET])?;

        let display_name_bytes = args.display_name.as_bytes();
        writer.bytes(display_name_bytes)?;
        writer.bytes(
            &[0_u8; PLAYER_NAME_MAX_BYTES][..PLAYER_NAME_MAX_BYTES - display_name_bytes.len()],
        )?;
        writer.bytes(args.title)?;
        writer.bytes(
            &[0_u8; APPEARANCE_TITLE_MAX_BYTES][..APPEARANCE_TITLE_MAX_BYTES - args.title.len()],
        )?;
        writer.bytes(args.model_code)?;
        writer.bytes(
            &[0_u8; APPEARANCE_MODEL_CODE_MAX_BYTES]
                [..APPEARANCE_MODEL_CODE_MAX_BYTES - args.model_code.len()],
        )?;
        for slot in 0..APPEARANCE_EQUIPMENT_SLOT_COUNT {
            writer.u8(0)?;
            writer.u8(slot as u8)?;
            writer.bytes(&[0_u8; APPEARANCE_EQUIPMENT_SLOT_LEN - 2])?;
        }

        if writer.offset != Self::LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_model_kind(model_kind: u8) -> Result<(), NicechunkPlayerError> {
        if model_kind != CHARACTER_MODEL_KIND_MALE && model_kind != CHARACTER_MODEL_KIND_FEMALE {
            return Err(NicechunkPlayerError::InvalidCharacterModelKind);
        }
        Ok(())
    }

    pub fn validate_title(title: &[u8]) -> Result<&str, NicechunkPlayerError> {
        if title.len() > APPEARANCE_TITLE_MAX_BYTES {
            return Err(NicechunkPlayerError::InvalidAppearanceTitle);
        }
        core::str::from_utf8(title).map_err(|_| NicechunkPlayerError::InvalidAppearanceTitle)
    }

    pub fn validate_model_code(code: &[u8]) -> Result<&str, NicechunkPlayerError> {
        if code.is_empty() || code.len() > APPEARANCE_MODEL_CODE_MAX_BYTES {
            return Err(NicechunkPlayerError::InvalidCharacterCode);
        }
        let value =
            core::str::from_utf8(code).map_err(|_| NicechunkPlayerError::InvalidCharacterCode)?;
        if !value.starts_with("NCM") {
            return Err(NicechunkPlayerError::InvalidCharacterCode);
        }
        Ok(value)
    }

    pub fn validate_owner_and_config(
        data: &[u8],
        owner: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != PLAYER_APPEARANCE_MAGIC {
            return Err(NicechunkPlayerError::InvalidAppearanceData.into());
        }
        if read_u16(data, 8) != PLAYER_APPEARANCE_VERSION {
            return Err(NicechunkPlayerError::InvalidAppearanceData.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
        }
        if &data[Self::PLAYER_PROFILE_OFFSET..Self::PLAYER_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidGlobalConfig.into());
        }
        Ok(())
    }

    pub fn validate_treasury_authority(data: &[u8], treasury_authority: &Pubkey) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != PLAYER_APPEARANCE_MAGIC {
            return Err(NicechunkPlayerError::InvalidAppearanceData.into());
        }
        if &data[Self::TREASURY_AUTHORITY_OFFSET..Self::TREASURY_AUTHORITY_OFFSET + 32]
            != treasury_authority.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidTreasuryAuthority.into());
        }
        Ok(())
    }

    pub fn owner(data: &[u8]) -> Result<Pubkey, NicechunkPlayerError> {
        if data.len() != Self::LEN || data[0..8] != PLAYER_APPEARANCE_MAGIC {
            return Err(NicechunkPlayerError::InvalidAppearanceData);
        }
        Ok(Pubkey::new_from_array(
            data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkPlayerError::InvalidAppearanceData)?,
        ))
    }

    pub fn created_metadata(data: &[u8]) -> Option<(u64, i64)> {
        if data.len() != Self::LEN || data.get(0..8)? != PLAYER_APPEARANCE_MAGIC {
            return None;
        }
        Some((
            read_u64(data, Self::CREATED_SLOT_OFFSET),
            i64::from_le_bytes(
                data[Self::CREATED_AT_OFFSET..Self::CREATED_AT_OFFSET + 8]
                    .try_into()
                    .ok()?,
            ),
        ))
    }
}

pub struct BackpackAccountView;

impl BackpackAccountView {
    pub fn validate_pda_and_owner(
        data: &[u8],
        backpack: &Pubkey,
        backpack_program: &Pubkey,
        owner: &Pubkey,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let backpack_id = read_u64(data, BACKPACK_ID_OFFSET);
        let backpack_id_bytes = backpack_id.to_le_bytes();
        let (expected_backpack, _) = Pubkey::find_program_address(
            &[BACKPACK_SEED, owner.as_ref(), &backpack_id_bytes],
            backpack_program,
        );
        if backpack != &expected_backpack {
            return Err(NicechunkPlayerError::InvalidForgingAuthority.into());
        }
        Ok(())
    }

    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        if data.len() != BACKPACK_LEN || data[0..8] != BACKPACK_MAGIC {
            return Err(NicechunkPlayerError::InvalidBackpackData.into());
        }
        if read_u16(data, 8) != BACKPACK_VERSION {
            return Err(NicechunkPlayerError::InvalidBackpackData.into());
        }
        if &data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn equippable_item_pda_at(
        data: &[u8],
        owner: &Pubkey,
        index: u8,
    ) -> Result<Pubkey, NicechunkPlayerError> {
        Self::validate_owner(data, owner).map_err(|_| NicechunkPlayerError::InvalidBackpackData)?;
        if index >= data[BACKPACK_ITEM_COUNT_OFFSET] {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        let offset = BACKPACK_HEADER_LEN + index as usize * BACKPACK_SLOT_RECORD_LEN;
        if offset + BACKPACK_SLOT_RECORD_LEN > data.len() {
            return Err(NicechunkPlayerError::InvalidBackpackData);
        }
        if data[offset] != BACKPACK_SLOT_KIND_ITEM
            || data[offset + 1] != BACKPACK_ITEM_CATEGORY_FORGED
        {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        if read_u32(data, offset + BACKPACK_SLOT_QUANTITY_OFFSET) == 0
            || read_u64(data, offset + BACKPACK_SLOT_ITEM_ID_OFFSET) == 0
        {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        let item = Pubkey::new_from_array(
            data[offset + BACKPACK_SLOT_ITEM_PDA_OFFSET
                ..offset + BACKPACK_SLOT_ITEM_PDA_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkPlayerError::InvalidBackpackItem)?,
        );
        let durability_current = read_u32(data, offset + BACKPACK_SLOT_DURABILITY_CURRENT_OFFSET);
        let durability_max = read_u32(data, offset + BACKPACK_SLOT_DURABILITY_MAX_OFFSET);
        let grade = data[offset + BACKPACK_SLOT_GRADE_OFFSET];
        let item_level = data[offset + BACKPACK_SLOT_ITEM_LEVEL_OFFSET];
        let quality_bps = read_u16(data, offset + BACKPACK_SLOT_QUALITY_BPS_OFFSET);
        if item == Pubkey::default()
            || durability_current == 0
            || durability_max == 0
            || durability_current > durability_max
            || grade == 0
            || grade > 10
            || item_level == 0
            || item_level > 100
            || quality_bps == 0
            || quality_bps > DURABILITY_BPS_DENOMINATOR
        {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        Ok(item)
    }

    pub fn equipment_record_at(
        data: &[u8],
        owner: &Pubkey,
        index: u8,
    ) -> Result<[u8; BACKPACK_SLOT_RECORD_LEN], NicechunkPlayerError> {
        Self::validate_owner(data, owner).map_err(|_| NicechunkPlayerError::InvalidBackpackData)?;
        if index >= data[BACKPACK_ITEM_COUNT_OFFSET] {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        let offset = BACKPACK_HEADER_LEN + index as usize * BACKPACK_SLOT_RECORD_LEN;
        if offset + BACKPACK_SLOT_RECORD_LEN > data.len() {
            return Err(NicechunkPlayerError::InvalidBackpackData);
        }
        let kind = data[offset];
        if (kind != BACKPACK_SLOT_KIND_BLOCK && kind != BACKPACK_SLOT_KIND_ITEM)
            || read_u32(data, offset + BACKPACK_SLOT_QUANTITY_OFFSET) == 0
        {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        if kind == BACKPACK_SLOT_KIND_ITEM
            && (read_u64(data, offset + BACKPACK_SLOT_ITEM_ID_OFFSET) == 0
                || data[offset + BACKPACK_SLOT_ITEM_PDA_OFFSET
                    ..offset + BACKPACK_SLOT_ITEM_PDA_OFFSET + 32]
                    .iter()
                    .all(|byte| *byte == 0))
        {
            return Err(NicechunkPlayerError::InvalidBackpackItem);
        }
        data[offset..offset + BACKPACK_SLOT_RECORD_LEN]
            .try_into()
            .map_err(|_| NicechunkPlayerError::InvalidBackpackData)
    }
}

pub struct PlayerSessionInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub session_authority: &'a Pubkey,
    pub player_profile: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub world_id: u16,
    pub allowed_actions: u16,
    pub expires_at: i64,
    pub max_actions: u32,
    pub created_slot: u64,
    pub created_at: i64,
}

/// Public session authorization account.
///
/// A player wallet creates this PDA once per short gameplay session. High
/// frequency gameplay transactions can then be signed by the temporary
/// `session_authority` key while other programs verify the owner, world,
/// action mask and expiry from this fixed layout.
pub struct PlayerSession;

impl PlayerSession {
    pub const LEN: usize = 184;
    pub const OWNER_OFFSET: usize = 12;
    pub const SESSION_AUTHORITY_OFFSET: usize = 44;
    pub const PLAYER_PROFILE_OFFSET: usize = 76;
    pub const GLOBAL_CONFIG_OFFSET: usize = 108;
    pub const WORLD_ID_OFFSET: usize = 140;
    pub const ALLOWED_ACTIONS_OFFSET: usize = 142;
    pub const EXPIRES_AT_OFFSET: usize = 144;
    pub const UPDATED_SLOT_OFFSET: usize = 160;
    pub const MAX_ACTIONS_OFFSET: usize = 176;
    pub const ACTION_COUNT_OFFSET: usize = 180;

    pub fn pack(dst: &mut [u8], args: &PlayerSessionInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerSessionData.into());
        }

        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&PLAYER_SESSION_MAGIC)?;
        writer.u16(PLAYER_SESSION_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.pubkey(args.owner)?;
        writer.pubkey(args.session_authority)?;
        writer.pubkey(args.player_profile)?;
        writer.pubkey(args.global_config)?;
        writer.u16(args.world_id)?;
        writer.u16(args.allowed_actions)?;
        writer.i64(args.expires_at)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.u32(args.max_actions)?;
        writer.u32(0)?;

        if writer.offset != Self::LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn refresh(
        dst: &mut [u8],
        owner: &Pubkey,
        session_authority: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
        allowed_actions: u16,
        expires_at: i64,
        max_actions: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner_and_config(
            dst,
            owner,
            session_authority,
            player_profile,
            global_config,
        )?;
        dst[Self::ALLOWED_ACTIONS_OFFSET..Self::ALLOWED_ACTIONS_OFFSET + 2]
            .copy_from_slice(&allowed_actions.to_le_bytes());
        dst[Self::EXPIRES_AT_OFFSET..Self::EXPIRES_AT_OFFSET + 8]
            .copy_from_slice(&expires_at.to_le_bytes());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        dst[Self::MAX_ACTIONS_OFFSET..Self::MAX_ACTIONS_OFFSET + 4]
            .copy_from_slice(&max_actions.to_le_bytes());
        dst[Self::ACTION_COUNT_OFFSET..Self::ACTION_COUNT_OFFSET + 4]
            .copy_from_slice(&0_u32.to_le_bytes());
        Ok(())
    }

    pub fn validate_owner_and_config(
        data: &[u8],
        owner: &Pubkey,
        session_authority: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != PLAYER_SESSION_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerSessionData.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
        }
        if &data[Self::SESSION_AUTHORITY_OFFSET..Self::SESSION_AUTHORITY_OFFSET + 32]
            != session_authority.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidSessionAuthority.into());
        }
        if &data[Self::PLAYER_PROFILE_OFFSET..Self::PLAYER_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidGlobalConfig.into());
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_action(
        data: &[u8],
        owner: &Pubkey,
        session_authority: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
        required_action: u16,
        now: i64,
    ) -> ProgramResult {
        Self::validate_owner_and_config(
            data,
            owner,
            session_authority,
            player_profile,
            global_config,
        )?;
        if read_i64(data, Self::EXPIRES_AT_OFFSET) <= now {
            return Err(NicechunkPlayerError::PlayerSessionExpired.into());
        }
        if read_u16(data, Self::ALLOWED_ACTIONS_OFFSET) & required_action != required_action {
            return Err(NicechunkPlayerError::PlayerSessionActionNotAllowed.into());
        }
        Ok(())
    }
}

pub struct UsernameIndexInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub player_profile: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub name_hash: &'a [u8; 32],
    pub display_name: &'a str,
    pub created_slot: u64,
}

/// One-name-one-owner index.
///
/// The account is addressed by a canonical name hash, so uniqueness checks are
/// O(1) regardless of total player count.
pub struct UsernameIndex;

impl UsernameIndex {
    pub const LEN: usize = 256;
    pub const OWNER_OFFSET: usize = 12;
    pub const PLAYER_PROFILE_OFFSET: usize = 44;
    pub const GLOBAL_CONFIG_OFFSET: usize = 76;
    pub const NAME_HASH_OFFSET: usize = 108;
    pub const NAME_LENGTH_OFFSET: usize = 140;
    pub const CREATED_SLOT_OFFSET: usize = 142;
    pub const UPDATED_SLOT_OFFSET: usize = 150;
    pub const NAME_BYTES_OFFSET: usize = 160;

    pub fn pack(dst: &mut [u8], args: &UsernameIndexInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidUsernameIndexData.into());
        }
        PlayerProfile::validate_name(args.display_name.as_bytes())?;
        let name_bytes = args.display_name.as_bytes();
        if name_bytes.len() > USERNAME_INDEX_NAME_MAX_BYTES {
            return Err(NicechunkPlayerError::InvalidPlayerName.into());
        }
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&USERNAME_INDEX_MAGIC)?;
        writer.u16(USERNAME_INDEX_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.pubkey(args.owner)?;
        writer.pubkey(args.player_profile)?;
        writer.pubkey(args.global_config)?;
        writer.bytes(args.name_hash)?;
        writer.u16(name_bytes.len() as u16)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.bytes(&[0_u8; Self::NAME_BYTES_OFFSET - 158])?;
        writer.bytes(name_bytes)?;
        writer.bytes(
            &[0_u8; USERNAME_INDEX_NAME_MAX_BYTES]
                [..USERNAME_INDEX_NAME_MAX_BYTES - name_bytes.len()],
        )?;
        if writer.offset != Self::LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_owner_or_available(
        data: &[u8],
        owner: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
        name_hash: &[u8; 32],
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != USERNAME_INDEX_MAGIC {
            return Err(NicechunkPlayerError::InvalidUsernameIndexData.into());
        }
        if read_u16(data, 8) != USERNAME_INDEX_VERSION {
            return Err(NicechunkPlayerError::InvalidUsernameIndexData.into());
        }
        if &data[Self::NAME_HASH_OFFSET..Self::NAME_HASH_OFFSET + 32] != name_hash {
            return Err(NicechunkPlayerError::InvalidUsernameIndexData.into());
        }
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidUsernameIndexData.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::UsernameAlreadyTaken.into());
        }
        if &data[Self::PLAYER_PROFILE_OFFSET..Self::PLAYER_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkPlayerError::UsernameAlreadyTaken.into());
        }
        Ok(())
    }
}

pub struct InviteIndexInitArgs<'a> {
    pub bump: u8,
    pub inviter: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub page_index: u32,
    pub created_slot: u64,
}

/// Public append-only invite page.
///
/// Each page stores only invited wallet keys and registration slots. Display
/// names are read from PlayerAppearance by wallet so invitation records stay
/// small and cheap to scan.
pub struct InviteIndex;

impl InviteIndex {
    pub const LEN: usize =
        INVITE_INDEX_HEADER_LEN + INVITE_INDEX_CAPACITY * INVITE_INDEX_RECORD_LEN;
    pub const INVITER_OFFSET: usize = 12;
    pub const GLOBAL_CONFIG_OFFSET: usize = 44;
    pub const PAGE_INDEX_OFFSET: usize = 76;
    pub const COUNT_OFFSET: usize = 80;
    pub const CAPACITY_OFFSET: usize = 82;
    pub const CREATED_SLOT_OFFSET: usize = 84;
    pub const UPDATED_SLOT_OFFSET: usize = 92;
    pub const RECORDS_OFFSET: usize = INVITE_INDEX_HEADER_LEN;

    pub fn pack_empty(dst: &mut [u8], args: &InviteIndexInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&INVITE_INDEX_MAGIC)?;
        writer.u16(INVITE_INDEX_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.pubkey(args.inviter)?;
        writer.pubkey(args.global_config)?;
        writer.u32(args.page_index)?;
        writer.u16(0)?;
        writer.u16(INVITE_INDEX_CAPACITY as u16)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.bytes(&[0_u8; INVITE_INDEX_HEADER_LEN - 100])?;
        writer.bytes(&[0_u8; INVITE_INDEX_CAPACITY * INVITE_INDEX_RECORD_LEN])?;
        if writer.offset != Self::LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate(
        data: &[u8],
        inviter: &Pubkey,
        global_config: &Pubkey,
        page_index: u32,
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != INVITE_INDEX_MAGIC {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        if read_u16(data, 8) != INVITE_INDEX_VERSION {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        if &data[Self::INVITER_OFFSET..Self::INVITER_OFFSET + 32] != inviter.as_ref() {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        if read_u32(data, Self::PAGE_INDEX_OFFSET) != page_index {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        if read_u16(data, Self::CAPACITY_OFFSET) as usize != INVITE_INDEX_CAPACITY {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        let count = read_u16(data, Self::COUNT_OFFSET) as usize;
        if count > INVITE_INDEX_CAPACITY {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        Ok(())
    }

    pub fn is_full(
        data: &[u8],
        inviter: &Pubkey,
        global_config: &Pubkey,
        page_index: u32,
    ) -> Result<bool, NicechunkPlayerError> {
        Self::validate(data, inviter, global_config, page_index)
            .map_err(|_| NicechunkPlayerError::InvalidInviteIndexData)?;
        Ok(read_u16(data, Self::COUNT_OFFSET) as usize >= INVITE_INDEX_CAPACITY)
    }

    pub fn append(dst: &mut [u8], invited: &Pubkey, updated_slot: u64) -> ProgramResult {
        if dst.len() != Self::LEN || dst[0..8] != INVITE_INDEX_MAGIC {
            return Err(NicechunkPlayerError::InvalidInviteIndexData.into());
        }
        let count = read_u16(dst, Self::COUNT_OFFSET) as usize;
        if count >= INVITE_INDEX_CAPACITY {
            return Err(NicechunkPlayerError::InviteIndexPageFull.into());
        }
        let offset = Self::RECORDS_OFFSET + count * INVITE_INDEX_RECORD_LEN;
        dst[offset..offset + 32].copy_from_slice(invited.as_ref());
        dst[offset + 32..offset + 40].copy_from_slice(&updated_slot.to_le_bytes());
        dst[Self::COUNT_OFFSET..Self::COUNT_OFFSET + 2]
            .copy_from_slice(&((count + 1) as u16).to_le_bytes());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
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
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
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

fn read_i64(data: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0_u8; 8]))
}

fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for byte in bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn same_backpack_item(left: &[u8], right: &[u8; BACKPACK_SLOT_RECORD_LEN]) -> bool {
    if left.len() != BACKPACK_SLOT_RECORD_LEN || left[0] != right[0] {
        return false;
    }
    if left[0] == BACKPACK_SLOT_KIND_ITEM {
        left[1] == right[1]
            && left[18..20] == right[18..20]
            && left[20..28] == right[20..28]
            && left[28..60] == right[28..60]
            && left[76..80] == right[76..80]
    } else {
        left[8..18] == right[8..18] && left[76..80] == right[76..80]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_profile_len_matches_pack() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = [0_u8; PlayerProfile::LEN];
        PlayerProfile::pack_default(
            &mut data,
            252,
            &owner,
            &global_config,
            1,
            "Jerry_Miner",
            123,
            456,
        )
        .unwrap();

        assert_eq!(&data[0..8], &PLAYER_PROFILE_MAGIC);
        assert_eq!(data[10], 252);
        assert_eq!(&data[12..44], owner.as_ref());
        assert_eq!(&data[44..76], global_config.as_ref());
        assert_eq!(data[102], EQUIPMENT_SLOT_COUNT as u8);
        assert_eq!(
            &data[PlayerProfile::EQUIPPED_BACKPACK_OFFSET
                ..PlayerProfile::EQUIPPED_BACKPACK_OFFSET + 32],
            Pubkey::default().as_ref()
        );
        assert_eq!(
            u64::from_le_bytes(
                data[PlayerProfile::CREATED_SLOT_OFFSET..PlayerProfile::CREATED_SLOT_OFFSET + 8]
                    .try_into()
                    .unwrap()
            ),
            123
        );
        assert_eq!(
            u64::from_le_bytes(
                data[PlayerProfile::UPDATED_SLOT_OFFSET..PlayerProfile::UPDATED_SLOT_OFFSET + 8]
                    .try_into()
                    .unwrap()
            ),
            123
        );
        assert_eq!(
            i64::from_le_bytes(
                data[PlayerProfile::CREATED_AT_OFFSET..PlayerProfile::CREATED_AT_OFFSET + 8]
                    .try_into()
                    .unwrap()
            ),
            456
        );
        let name_len = u16::from_le_bytes(
            data[PlayerProfile::NAME_LENGTH_OFFSET..PlayerProfile::NAME_LENGTH_OFFSET + 2]
                .try_into()
                .unwrap(),
        ) as usize;
        assert_eq!(
            core::str::from_utf8(
                &data
                    [PlayerProfile::NAME_BYTES_OFFSET..PlayerProfile::NAME_BYTES_OFFSET + name_len]
            )
            .unwrap(),
            "Jerry_Miner"
        );
    }

    #[test]
    fn player_profile_rejects_wrong_len() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = [0_u8; PlayerProfile::LEN - 1];
        assert!(PlayerProfile::pack_default(
            &mut data,
            252,
            &owner,
            &global_config,
            1,
            "",
            123,
            456
        )
        .is_err());
    }

    #[test]
    fn player_equipment_persists_verified_backpack_identity_and_model() {
        let owner = Pubkey::new_unique();
        let profile = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let backpack = Pubkey::new_unique();
        let item_pda = Pubkey::new_unique();
        let mut data = vec![0_u8; PlayerEquipment::LEN];
        PlayerEquipment::pack_empty(
            &mut data,
            &PlayerEquipmentInitArgs {
                bump: 250,
                owner: &owner,
                player_profile: &profile,
                global_config: &global_config,
                created_slot: 100,
            },
        )
        .unwrap();

        let mut record = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record[0] = BACKPACK_SLOT_KIND_ITEM;
        record[1] = BACKPACK_ITEM_CATEGORY_FORGED;
        record[4..8].copy_from_slice(&1_u32.to_le_bytes());
        record[18..20].copy_from_slice(&BACKPACK_FORGED_ITEM_CODE.to_le_bytes());
        record[20..28].copy_from_slice(&77_u64.to_le_bytes());
        record[28..60].copy_from_slice(item_pda.as_ref());
        let model = [0xe0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
        record[76..80].copy_from_slice(&fnv1a32(&model).to_le_bytes());

        PlayerEquipment::write_slot(&mut data, 7, 4, &backpack, &record, &model, 101).unwrap();
        PlayerEquipment::validate_owner_and_config(&data, &owner, &profile, &global_config)
            .unwrap();
        let offset = PlayerEquipment::SLOTS_OFFSET + 7 * PLAYER_EQUIPMENT_SLOT_LEN;
        assert_eq!(data[offset], 1);
        assert_eq!(data[offset + 1], 7);
        assert_eq!(data[offset + 2], 4);
        assert_eq!(&data[offset + 8..offset + 40], backpack.as_ref());
        assert_eq!(&data[offset + 40..offset + 120], &record);
        assert_eq!(&data[offset + 120..offset + 134], &model);

        let mut invalid_record = record;
        invalid_record[76..80].copy_from_slice(&123_u32.to_le_bytes());
        assert!(PlayerEquipment::write_slot(
            &mut data,
            6,
            5,
            &backpack,
            &invalid_record,
            &model,
            102,
        )
        .is_err());
        PlayerEquipment::clear_slot(&mut data, 7, 103).unwrap();
        assert_eq!(data[offset], 0);
        assert_eq!(data[offset + 2], u8::MAX);
    }

    #[test]
    fn player_equipment_accepts_legacy_and_current_ncf1_models() {
        let mut record = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record[0] = BACKPACK_SLOT_KIND_ITEM;
        record[1] = BACKPACK_ITEM_CATEGORY_FORGED;
        record[18..20].copy_from_slice(&BACKPACK_FORGED_ITEM_CODE.to_le_bytes());

        for version in [NCF1_LEGACY_VERSION, NCF1_VERSION] {
            let mut model = [0_u8; 14];
            model[0] = version << 4;
            record[76..80].copy_from_slice(&fnv1a32(&model).to_le_bytes());
            PlayerEquipment::validate_model_code(&record, &model).unwrap();
        }

        let mut retired_model = [0_u8; 14];
        retired_model[0] = 13 << 4;
        record[76..80].copy_from_slice(&fnv1a32(&retired_model).to_le_bytes());
        assert!(PlayerEquipment::validate_model_code(&record, &retired_model).is_err());
    }

    #[test]
    fn player_equipment_custody_and_slot_swaps_preserve_embedded_items() {
        let owner = Pubkey::new_unique();
        let profile = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let backpack = Pubkey::new_unique();
        let mut data = vec![0_u8; PlayerEquipment::LEN];
        PlayerEquipment::pack_empty(
            &mut data,
            &PlayerEquipmentInitArgs {
                bump: 250,
                owner: &owner,
                player_profile: &profile,
                global_config: &global_config,
                created_slot: 100,
            },
        )
        .unwrap();

        let mut first = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        first[0] = BACKPACK_SLOT_KIND_BLOCK;
        first[4..8].copy_from_slice(&1_u32.to_le_bytes());
        first[8..12].copy_from_slice(&101_i32.to_le_bytes());
        let mut second = first;
        second[8..12].copy_from_slice(&202_i32.to_le_bytes());
        PlayerEquipment::write_custodied_slot(&mut data, 2, 4, &backpack, &first, &[], 101)
            .unwrap();
        PlayerEquipment::write_custodied_slot(&mut data, 6, 5, &backpack, &second, &[], 102)
            .unwrap();

        assert!(PlayerEquipment::slot_is_custodied(&data, 2).ok().unwrap());
        assert!(PlayerEquipment::slot_is_custodied(&data, 6).ok().unwrap());
        assert_eq!(PlayerEquipment::slot_record(&data, 2).ok().unwrap(), first);
        assert_eq!(PlayerEquipment::slot_record(&data, 6).ok().unwrap(), second);
        let first_identity = PlayerEquipment::slot_identity(&data, 2).ok().unwrap();
        let second_identity = PlayerEquipment::slot_identity(&data, 6).ok().unwrap();

        PlayerEquipment::swap_slots(&mut data, 2, 6, 103).unwrap();

        assert!(PlayerEquipment::slot_is_custodied(&data, 2).ok().unwrap());
        assert!(PlayerEquipment::slot_is_custodied(&data, 6).ok().unwrap());
        assert_eq!(PlayerEquipment::slot_record(&data, 2).ok().unwrap(), second);
        assert_eq!(PlayerEquipment::slot_record(&data, 6).ok().unwrap(), first);
        assert_eq!(
            PlayerEquipment::slot_identity(&data, 2).ok().unwrap(),
            second_identity
        );
        assert_eq!(
            PlayerEquipment::slot_identity(&data, 6).ok().unwrap(),
            first_identity
        );
    }

    #[test]
    fn player_equipment_durability_is_consumed_in_custodied_record() {
        let owner = Pubkey::new_unique();
        let profile = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let backpack = Pubkey::new_unique();
        let item_pda = Pubkey::new_unique();
        let mut data = vec![0_u8; PlayerEquipment::LEN];
        PlayerEquipment::pack_empty(
            &mut data,
            &PlayerEquipmentInitArgs {
                bump: 250,
                owner: &owner,
                player_profile: &profile,
                global_config: &global_config,
                created_slot: 100,
            },
        )
        .unwrap();
        let mut record = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record[0] = BACKPACK_SLOT_KIND_ITEM;
        record[1] = BACKPACK_ITEM_CATEGORY_FORGED;
        record[4..8].copy_from_slice(&1_u32.to_le_bytes());
        record[18..20].copy_from_slice(&BACKPACK_FORGED_ITEM_CODE.to_le_bytes());
        record[20..28].copy_from_slice(&77_u64.to_le_bytes());
        record[28..60].copy_from_slice(item_pda.as_ref());
        record
            [BACKPACK_SLOT_DURABILITY_CURRENT_OFFSET..BACKPACK_SLOT_DURABILITY_CURRENT_OFFSET + 4]
            .copy_from_slice(&120_u32.to_le_bytes());
        record[BACKPACK_SLOT_DURABILITY_MAX_OFFSET..BACKPACK_SLOT_DURABILITY_MAX_OFFSET + 4]
            .copy_from_slice(&150_u32.to_le_bytes());
        PlayerEquipment::write_custodied_slot(&mut data, 3, 4, &backpack, &record, &[], 101)
            .unwrap();

        PlayerEquipment::consume_forged_durability(&mut data, 3, 7, 102).unwrap();
        let stored = PlayerEquipment::slot_record(&data, 3).ok().unwrap();
        assert_eq!(
            read_u32(&stored, BACKPACK_SLOT_DURABILITY_CURRENT_OFFSET),
            113
        );
        assert_eq!(read_u64(&data, PlayerEquipment::UPDATED_SLOT_OFFSET), 102);
        assert!(PlayerEquipment::consume_forged_durability(&mut data, 3, 114, 103).is_err());
    }

    #[test]
    fn player_appearance_len_matches_pack() {
        let owner = Pubkey::new_unique();
        let player_profile = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let treasury_authority = Pubkey::new_unique();
        let mut data = vec![0_u8; PlayerAppearance::LEN];
        PlayerAppearance::pack(
            &mut data,
            &PlayerAppearanceInitArgs {
                bump: 251,
                owner: &owner,
                player_profile: &player_profile,
                global_config: &global_config,
                treasury_authority: &treasury_authority,
                model_kind: CHARACTER_MODEL_KIND_MALE,
                display_name: "Jerry_Miner",
                title: b"Genesis Miner",
                model_code: b"NCM2:test-model",
                created_slot: 123,
                updated_slot: 124,
                created_at: 456,
                updated_at: 457,
            },
        )
        .unwrap();

        assert_eq!(&data[0..8], &PLAYER_APPEARANCE_MAGIC);
        assert_eq!(u16::from_le_bytes(data[8..10].try_into().unwrap()), 1);
        assert_eq!(data[10], 251);
        assert_eq!(
            &data[PlayerAppearance::OWNER_OFFSET..PlayerAppearance::OWNER_OFFSET + 32],
            owner.as_ref()
        );
        assert_eq!(
            &data[PlayerAppearance::TREASURY_AUTHORITY_OFFSET
                ..PlayerAppearance::TREASURY_AUTHORITY_OFFSET + 32],
            treasury_authority.as_ref()
        );
        assert_eq!(
            data[PlayerAppearance::MODEL_KIND_OFFSET],
            CHARACTER_MODEL_KIND_MALE
        );
        assert_eq!(
            u16::from_le_bytes(
                data[PlayerAppearance::DISPLAY_NAME_LENGTH_OFFSET
                    ..PlayerAppearance::DISPLAY_NAME_LENGTH_OFFSET + 2]
                    .try_into()
                    .unwrap()
            ) as usize,
            "Jerry_Miner".as_bytes().len()
        );
        assert_eq!(
            core::str::from_utf8(
                &data[PlayerAppearance::DISPLAY_NAME_OFFSET
                    ..PlayerAppearance::DISPLAY_NAME_OFFSET + "Jerry_Miner".as_bytes().len()]
            )
            .unwrap(),
            "Jerry_Miner"
        );
        assert_eq!(
            core::str::from_utf8(
                &data[PlayerAppearance::TITLE_OFFSET
                    ..PlayerAppearance::TITLE_OFFSET + "Genesis Miner".len()]
            )
            .unwrap(),
            "Genesis Miner"
        );
        assert_eq!(
            core::str::from_utf8(
                &data[PlayerAppearance::MODEL_CODE_OFFSET
                    ..PlayerAppearance::MODEL_CODE_OFFSET + "NCM2:test-model".len()]
            )
            .unwrap(),
            "NCM2:test-model"
        );
        assert_eq!(
            data[PlayerAppearance::EQUIPMENT_OFFSET + APPEARANCE_EQUIPMENT_SLOT_LEN],
            0
        );
        assert_eq!(
            data[PlayerAppearance::EQUIPMENT_OFFSET + APPEARANCE_EQUIPMENT_SLOT_LEN + 1],
            1
        );
        assert_eq!(data.len(), PlayerAppearance::LEN);
    }

    #[test]
    fn player_session_len_matches_pack() {
        let owner = Pubkey::new_unique();
        let session_authority = Pubkey::new_unique();
        let player_profile = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = [0_u8; PlayerSession::LEN];
        PlayerSession::pack(
            &mut data,
            &PlayerSessionInitArgs {
                bump: 251,
                owner: &owner,
                session_authority: &session_authority,
                player_profile: &player_profile,
                global_config: &global_config,
                world_id: 1,
                allowed_actions: SESSION_ACTION_BREAK_BLOCK | SESSION_ACTION_PLACE_BLOCK,
                expires_at: 999,
                max_actions: 10,
                created_slot: 123,
                created_at: 456,
            },
        )
        .unwrap();

        assert_eq!(&data[0..8], &PLAYER_SESSION_MAGIC);
        assert_eq!(data[10], 251);
        assert_eq!(
            &data[PlayerSession::OWNER_OFFSET..PlayerSession::OWNER_OFFSET + 32],
            owner.as_ref()
        );
        assert_eq!(
            &data[PlayerSession::SESSION_AUTHORITY_OFFSET
                ..PlayerSession::SESSION_AUTHORITY_OFFSET + 32],
            session_authority.as_ref()
        );
        assert_eq!(u16::from_le_bytes(data[142..144].try_into().unwrap()), 6);
        assert_eq!(i64::from_le_bytes(data[144..152].try_into().unwrap()), 999);
        assert_eq!(u32::from_le_bytes(data[176..180].try_into().unwrap()), 10);
    }
}
