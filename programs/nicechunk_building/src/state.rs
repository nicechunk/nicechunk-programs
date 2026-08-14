use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkBuildingError;

pub const GLOBAL_CONFIG_LEN: usize = 293;
pub const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
pub const CANONICAL_CHUNK_SIZE: u16 = 16;
pub const CANONICAL_MIN_BUILD_Y: i16 = -32;
pub const CANONICAL_MAX_BUILD_Y: i16 = 320;

const PLAYER_PROFILE_LEN: usize = 773;
const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
const PLAYER_PROFILE_VERSION: u16 = 7;
const PLAYER_PROFILE_INITIALIZED_OFFSET: usize = 11;
const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;
const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET: usize = 102;
const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT: usize = 9;
const PLAYER_SESSION_LEN: usize = 184;
const PLAYER_SESSION_MAGIC: [u8; 8] = *b"NCKSES01";
const PLAYER_SESSION_VERSION: u16 = 1;
const PLAYER_SESSION_INITIALIZED_OFFSET: usize = 11;
const PLAYER_SESSION_OWNER_OFFSET: usize = 12;
const PLAYER_SESSION_AUTHORITY_OFFSET: usize = 44;
const PLAYER_SESSION_PROFILE_OFFSET: usize = 76;
const PLAYER_SESSION_GLOBAL_CONFIG_OFFSET: usize = 108;
const PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET: usize = 142;
const PLAYER_SESSION_EXPIRES_AT_OFFSET: usize = 144;

#[derive(Clone, Copy, Debug)]
pub struct GlobalConfigView {
    pub chunk_size: u16,
    pub min_build_y: i16,
    pub max_build_y: i16,
}

impl GlobalConfigView {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkBuildingError> {
        if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
            return Err(NicechunkBuildingError::InvalidGlobalConfigData);
        }
        Ok(Self {
            chunk_size: CANONICAL_CHUNK_SIZE,
            min_build_y: CANONICAL_MIN_BUILD_Y,
            max_build_y: CANONICAL_MAX_BUILD_Y,
        })
    }
}

pub struct PlayerProfileView;

impl PlayerProfileView {
    pub fn validate(data: &[u8], owner: &Pubkey, global_config: &Pubkey) -> ProgramResult {
        if data.len() != PLAYER_PROFILE_LEN
            || data[0..8] != PLAYER_PROFILE_MAGIC
            || read_u16(data, 8) != PLAYER_PROFILE_VERSION
            || data[PLAYER_PROFILE_INITIALIZED_OFFSET] != 1
            || data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] as usize
                != PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT
        {
            return Err(NicechunkBuildingError::InvalidPlayerProfile.into());
        }
        if &data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkBuildingError::InvalidPlayerAuthority.into());
        }
        if &data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkBuildingError::InvalidGlobalConfigData.into());
        }
        Ok(())
    }
}

pub struct PlayerSessionView {
    pub owner: Pubkey,
}

impl PlayerSessionView {
    pub fn validate(
        data: &[u8],
        session_authority: &Pubkey,
        player_profile: &Pubkey,
        global_config: &Pubkey,
        action: u8,
        now: i64,
    ) -> Result<Self, NicechunkBuildingError> {
        if data.len() != PLAYER_SESSION_LEN
            || data[0..8] != PLAYER_SESSION_MAGIC
            || read_u16(data, 8) != PLAYER_SESSION_VERSION
            || data[PLAYER_SESSION_INITIALIZED_OFFSET] != 1
        {
            return Err(NicechunkBuildingError::InvalidPlayerSession);
        }
        if &data[PLAYER_SESSION_AUTHORITY_OFFSET..PLAYER_SESSION_AUTHORITY_OFFSET + 32]
            != session_authority.as_ref()
        {
            return Err(NicechunkBuildingError::InvalidSessionAuthority);
        }
        if &data[PLAYER_SESSION_PROFILE_OFFSET..PLAYER_SESSION_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkBuildingError::InvalidPlayerProfile);
        }
        if &data[PLAYER_SESSION_GLOBAL_CONFIG_OFFSET..PLAYER_SESSION_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkBuildingError::InvalidGlobalConfigData);
        }
        if read_i64(data, PLAYER_SESSION_EXPIRES_AT_OFFSET) <= now {
            return Err(NicechunkBuildingError::PlayerSessionExpired);
        }
        let allowed_actions = read_u16(data, PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET);
        if action >= 16 || allowed_actions & (1_u16 << action) == 0 {
            return Err(NicechunkBuildingError::SessionActionNotAllowed);
        }
        let owner = Pubkey::new_from_array(
            data[PLAYER_SESSION_OWNER_OFFSET..PLAYER_SESSION_OWNER_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkBuildingError::InvalidPlayerSession)?,
        );
        Ok(Self { owner })
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_i64(data: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0; 8]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_views_require_final_initialized_layouts() {
        let owner = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let profile = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut profile_data = vec![0_u8; PLAYER_PROFILE_LEN];
        profile_data[0..8].copy_from_slice(&PLAYER_PROFILE_MAGIC);
        profile_data[8..10].copy_from_slice(&PLAYER_PROFILE_VERSION.to_le_bytes());
        profile_data[PLAYER_PROFILE_INITIALIZED_OFFSET] = 1;
        profile_data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        profile_data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        profile_data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] =
            PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT as u8;
        PlayerProfileView::validate(&profile_data, &owner, &global_config).unwrap();
        profile_data[8..10].copy_from_slice(&(PLAYER_PROFILE_VERSION - 1).to_le_bytes());
        assert!(PlayerProfileView::validate(&profile_data, &owner, &global_config).is_err());

        let mut session_data = vec![0_u8; PLAYER_SESSION_LEN];
        session_data[0..8].copy_from_slice(&PLAYER_SESSION_MAGIC);
        session_data[8..10].copy_from_slice(&PLAYER_SESSION_VERSION.to_le_bytes());
        session_data[PLAYER_SESSION_INITIALIZED_OFFSET] = 1;
        session_data[PLAYER_SESSION_OWNER_OFFSET..PLAYER_SESSION_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        session_data[PLAYER_SESSION_AUTHORITY_OFFSET..PLAYER_SESSION_AUTHORITY_OFFSET + 32]
            .copy_from_slice(authority.as_ref());
        session_data[PLAYER_SESSION_PROFILE_OFFSET..PLAYER_SESSION_PROFILE_OFFSET + 32]
            .copy_from_slice(profile.as_ref());
        session_data[PLAYER_SESSION_GLOBAL_CONFIG_OFFSET..PLAYER_SESSION_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        session_data
            [PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET..PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET + 2]
            .copy_from_slice(&1_u16.to_le_bytes());
        session_data[PLAYER_SESSION_EXPIRES_AT_OFFSET..PLAYER_SESSION_EXPIRES_AT_OFFSET + 8]
            .copy_from_slice(&200_i64.to_le_bytes());
        PlayerSessionView::validate(&session_data, &authority, &profile, &global_config, 0, 100)
            .unwrap();
        session_data[PLAYER_SESSION_INITIALIZED_OFFSET] = 0;
        assert!(PlayerSessionView::validate(
            &session_data,
            &authority,
            &profile,
            &global_config,
            0,
            100,
        )
        .is_err());
    }
}
