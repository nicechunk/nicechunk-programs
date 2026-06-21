use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkPlayerError;

pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_VERSION: u16 = 1;
pub const PLAYER_PROFILE_SEED: &[u8] = b"player";
pub const PLAYER_SESSION_MAGIC: [u8; 8] = *b"NCKSES01";
pub const PLAYER_SESSION_VERSION: u16 = 1;
pub const PLAYER_SESSION_SEED: &[u8] = b"session";
pub const SESSION_ACTION_BREAK_BLOCK: u16 = 1 << 1;
pub const SESSION_ACTION_PLACE_BLOCK: u16 = 1 << 2;
pub const EQUIPMENT_SLOT_COUNT: usize = 9;
pub const LEGACY_PLAYER_PROFILE_LEN: usize = 417;
pub const DEFAULT_HEALTH: u16 = 100;
pub const DEFAULT_ENERGY: u16 = 100;
pub const DEFAULT_STAMINA: u16 = 100;
pub const DEFAULT_MINING_POWER: u16 = 1;
pub const DEFAULT_BUILD_POWER: u16 = 1;
pub const DEFAULT_DEFENSE: u16 = 0;
pub const DEFAULT_BACKPACK_STYLE: u8 = 0;

pub const GLOBAL_CONFIG_LEN: usize = 293;
pub const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
pub const GLOBAL_CONFIG_WORLD_ID_OFFSET: usize = 85;
pub const GLOBAL_CONFIG_MIN_BUILD_Y_OFFSET: usize = 263;
pub const GLOBAL_CONFIG_MAX_BUILD_Y_OFFSET: usize = 265;

pub const BACKPACK_LEN: usize = 1118;
pub const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
pub const BACKPACK_OWNER_OFFSET: usize = 20;

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
    pub const LEN: usize = 449;
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
    pub const LEGACY_CREATED_SLOT_OFFSET: usize = 393;
    pub const LEGACY_UPDATED_SLOT_OFFSET: usize = 401;
    pub const LEGACY_CREATED_AT_OFFSET: usize = 409;

    pub fn pack_default(
        dst: &mut [u8],
        bump: u8,
        owner: &Pubkey,
        global_config: &Pubkey,
        world_id: u16,
        created_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }

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

        if writer.offset != Self::LEN {
            return Err(NicechunkPlayerError::PackSizeMismatch.into());
        }
        Ok(())
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
        if !Self::is_supported_len(data.len()) || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidPlayerAuthority.into());
        }
        Ok(())
    }

    pub fn is_supported_len(len: usize) -> bool {
        len == Self::LEN || len == LEGACY_PLAYER_PROFILE_LEN
    }

    pub fn has_equipped_backpack(data: &[u8]) -> Result<bool, NicechunkPlayerError> {
        if !Self::is_supported_len(data.len()) || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData);
        }
        if data.len() == LEGACY_PLAYER_PROFILE_LEN {
            return Ok(false);
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
        if !Self::is_supported_len(dst.len()) {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        dst[Self::POSITION_OFFSET..Self::POSITION_OFFSET + 4].copy_from_slice(&x.to_le_bytes());
        dst[Self::POSITION_OFFSET + 4..Self::POSITION_OFFSET + 8].copy_from_slice(&y.to_le_bytes());
        dst[Self::POSITION_OFFSET + 8..Self::POSITION_OFFSET + 12]
            .copy_from_slice(&z.to_le_bytes());
        let updated_slot_offset = Self::updated_slot_offset(dst.len())?;
        dst[updated_slot_offset..updated_slot_offset + 8]
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
        if !Self::is_supported_len(dst.len()) {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        let offset = Self::EQUIPMENT_OFFSET + slot as usize * 32;
        dst[offset..offset + 32].copy_from_slice(item.as_ref());
        let updated_slot_offset = Self::updated_slot_offset(dst.len())?;
        dst[updated_slot_offset..updated_slot_offset + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn write_backpack_style(
        dst: &mut [u8],
        backpack_style: u8,
        updated_slot: u64,
    ) -> ProgramResult {
        if !Self::is_supported_len(dst.len()) {
            return Err(NicechunkPlayerError::InvalidPlayerProfileData.into());
        }
        dst[Self::BACKPACK_STYLE_OFFSET] = backpack_style;
        let updated_slot_offset = Self::updated_slot_offset(dst.len())?;
        dst[updated_slot_offset..updated_slot_offset + 8]
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

    fn updated_slot_offset(len: usize) -> Result<usize, NicechunkPlayerError> {
        if len == Self::LEN {
            Ok(Self::UPDATED_SLOT_OFFSET)
        } else if len == LEGACY_PLAYER_PROFILE_LEN {
            Ok(Self::LEGACY_UPDATED_SLOT_OFFSET)
        } else {
            Err(NicechunkPlayerError::InvalidPlayerProfileData)
        }
    }
}

pub struct BackpackAccountView;

impl BackpackAccountView {
    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        if data.len() != BACKPACK_LEN || data[0..8] != BACKPACK_MAGIC {
            return Err(NicechunkPlayerError::InvalidBackpackData.into());
        }
        if &data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkPlayerError::InvalidBackpackOwner.into());
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_profile_len_matches_pack() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = [0_u8; PlayerProfile::LEN];
        PlayerProfile::pack_default(&mut data, 252, &owner, &global_config, 1, 123, 456).unwrap();

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
    }

    #[test]
    fn player_profile_rejects_wrong_len() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = [0_u8; PlayerProfile::LEN - 1];
        assert!(
            PlayerProfile::pack_default(&mut data, 252, &owner, &global_config, 1, 123, 456)
                .is_err()
        );
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
