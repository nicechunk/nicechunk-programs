use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkChunkError;

pub const CHUNK_MAGIC: [u8; 8] = *b"NCKCHK01";
pub const CHUNK_VERSION: u16 = 1;
pub const CHUNK_SEED: &[u8] = b"chunk";
pub const MAX_BLOCK_DELTAS: usize = 128;
pub const BLOCK_DELTA_LEN: usize = 64;

pub const CHUNK_BROKEN_MAGIC: [u8; 4] = *b"NCBK";
pub const CHUNK_BROKEN_VERSION: u8 = 1;
pub const CHUNK_BROKEN_SEED: &[u8] = b"chunk-broken";
pub const CHUNK_BROKEN_HEADER_LEN: usize = 16;
pub const CHUNK_BROKEN_RECORD_LEN: usize = 3;
pub const CHUNK_BROKEN_INITIAL_CAPACITY: u16 = 64;
pub const CHUNK_BROKEN_GROW_BY: u16 = 64;
pub const CHUNK_BROKEN_MAX_CAPACITY: u16 = 2048;
pub const CHUNK_BROKEN_PACKED_Y_BITS: i32 = 9;
pub const CHUNK_BROKEN_MAX_Y_OFFSET: i32 = (1_i32 << CHUNK_BROKEN_PACKED_Y_BITS) - 1;

pub const GLOBAL_CONFIG_LEN: usize = 293;
pub const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
pub const GLOBAL_CONFIG_WORLD_ID_OFFSET: usize = 85;
pub const GLOBAL_CONFIG_WORLD_SEED_OFFSET: usize = 87;
pub const GLOBAL_CONFIG_CHUNK_SIZE_OFFSET: usize = 259;
pub const GLOBAL_CONFIG_MIN_BUILD_Y_OFFSET: usize = 263;
pub const GLOBAL_CONFIG_MAX_BUILD_Y_OFFSET: usize = 265;
pub const GLOBAL_CONFIG_MAX_TERRAIN_HEIGHT_OFFSET: usize = 267;
pub const GLOBAL_CONFIG_SEA_LEVEL_OFFSET: usize = 269;

pub const BLOCK_AIR: u16 = 0;
pub const BLOCK_GRASS: u16 = 1;
pub const BLOCK_DIRT: u16 = 2;
pub const BLOCK_STONE: u16 = 3;
pub const BLOCK_DEEP_STONE: u16 = 4;
pub const BLOCK_BEDROCK: u16 = 16;
pub const BLOCK_WATER: u16 = 17;

pub const LEGACY_PLAYER_PROFILE_LEN: usize = 417;
pub const PLAYER_PROFILE_LEN: usize = 449;
pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
pub const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;

pub const PLAYER_SESSION_LEN: usize = 184;
pub const PLAYER_SESSION_MAGIC: [u8; 8] = *b"NCKSES01";
pub const PLAYER_SESSION_OWNER_OFFSET: usize = 12;
pub const PLAYER_SESSION_AUTHORITY_OFFSET: usize = 44;
pub const PLAYER_SESSION_PROFILE_OFFSET: usize = 76;
pub const PLAYER_SESSION_GLOBAL_CONFIG_OFFSET: usize = 108;
pub const PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET: usize = 142;
pub const PLAYER_SESSION_EXPIRES_AT_OFFSET: usize = 144;

pub struct GlobalConfigView {
    pub world_id: u16,
    pub world_seed: [u8; 32],
    pub chunk_size: u16,
    pub min_build_y: i16,
    pub max_build_y: i16,
    pub max_terrain_height: i16,
    pub sea_level: i16,
}

impl GlobalConfigView {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() != GLOBAL_CONFIG_LEN || data[0..8] != GLOBAL_CONFIG_MAGIC {
            return Err(NicechunkChunkError::InvalidGlobalConfigData);
        }
        Ok(Self {
            world_id: read_u16(data, GLOBAL_CONFIG_WORLD_ID_OFFSET),
            world_seed: data[GLOBAL_CONFIG_WORLD_SEED_OFFSET..GLOBAL_CONFIG_WORLD_SEED_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkChunkError::InvalidGlobalConfigData)?,
            chunk_size: read_u16(data, GLOBAL_CONFIG_CHUNK_SIZE_OFFSET),
            min_build_y: read_i16(data, GLOBAL_CONFIG_MIN_BUILD_Y_OFFSET),
            max_build_y: read_i16(data, GLOBAL_CONFIG_MAX_BUILD_Y_OFFSET),
            max_terrain_height: read_i16(data, GLOBAL_CONFIG_MAX_TERRAIN_HEIGHT_OFFSET),
            sea_level: read_i16(data, GLOBAL_CONFIG_SEA_LEVEL_OFFSET),
        })
    }
}

#[derive(Clone, Copy)]
pub struct GeneratedBlockArgs {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub expected_block_id: u16,
}

impl GeneratedBlockArgs {
    pub const LEN: usize = 14;
    pub const INSPECT_ONLY_EXPECTED_BLOCK_ID: u16 = u16::MAX;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() != Self::LEN {
            return Err(NicechunkChunkError::InvalidInstruction);
        }
        Ok(Self {
            chunk_x: read_i32(data, 0),
            chunk_z: read_i32(data, 4),
            local_x: data[8],
            y: read_i16(data, 9),
            local_z: data[11],
            expected_block_id: read_u16(data, 12),
        })
    }

    pub fn validate(&self, global_config: &GlobalConfigView) -> ProgramResult {
        if self.local_x as u16 >= global_config.chunk_size
            || self.local_z as u16 >= global_config.chunk_size
            || self.y < global_config.min_build_y
            || self.y > global_config.max_build_y
        {
            return Err(NicechunkChunkError::InvalidBlockCoordinate.into());
        }
        Ok(())
    }

    pub fn world_x(&self, global_config: &GlobalConfigView) -> i32 {
        self.chunk_x
            .saturating_mul(global_config.chunk_size as i32)
            .saturating_add(self.local_x as i32)
    }

    pub fn world_z(&self, global_config: &GlobalConfigView) -> i32 {
        self.chunk_z
            .saturating_mul(global_config.chunk_size as i32)
            .saturating_add(self.local_z as i32)
    }
}

/// Minimal integer-only base terrain verifier.
///
/// This intentionally does not mirror the full browser renderer yet. It proves
/// that the program can derive a stable block id from GlobalConfig.world_seed
/// and one coordinate without generating a whole chunk or using floating point.
pub fn generated_block_id_at(global_config: &GlobalConfigView, args: &GeneratedBlockArgs) -> u16 {
    let world_x = args.world_x(global_config);
    let world_z = args.world_z(global_config);
    let surface = generated_surface_height(global_config, world_x, world_z);

    if args.y <= global_config.min_build_y {
        return BLOCK_BEDROCK;
    }
    if args.y > surface {
        if args.y <= global_config.sea_level {
            return BLOCK_WATER;
        }
        return BLOCK_AIR;
    }
    if args.y == surface {
        return BLOCK_GRASS;
    }
    if args.y >= surface.saturating_sub(3) {
        return BLOCK_DIRT;
    }
    if args.y < global_config.min_build_y.saturating_add(12) {
        return BLOCK_DEEP_STONE;
    }
    BLOCK_STONE
}

pub fn generated_surface_height(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
) -> i16 {
    let min_surface = global_config.min_build_y.saturating_add(8);
    let max_surface = global_config
        .max_terrain_height
        .min(global_config.max_build_y.saturating_sub(1))
        .max(min_surface);
    let span = (max_surface as i32 - min_surface as i32 + 1).max(1) as u32;
    let base = hash_coord(&global_config.world_seed, world_x, world_z, 0) % span;
    let detail =
        (hash_coord(&global_config.world_seed, world_x >> 2, world_z >> 2, 1) % 9) as i32 - 4;
    (min_surface as i32 + base as i32 + detail).clamp(min_surface as i32, max_surface as i32) as i16
}

fn hash_coord(seed: &[u8; 32], x: i32, z: i32, salt: u32) -> u32 {
    let mut hash = 0x811c_9dc5_u32 ^ salt;
    for byte in seed {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    for byte in x.to_le_bytes().iter().chain(z.to_le_bytes().iter()) {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    hash ^ (hash >> 16)
}

pub struct PlayerProfileView;

impl PlayerProfileView {
    pub fn validate(data: &[u8], authority: &Pubkey, global_config: &Pubkey) -> ProgramResult {
        if !is_supported_player_profile_len(data.len()) || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkChunkError::InvalidPlayerProfile.into());
        }
        if &data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
            != authority.as_ref()
        {
            return Err(NicechunkChunkError::InvalidPlayerAuthority.into());
        }
        if &data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkChunkError::InvalidGlobalConfig.into());
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
        global_config: &Pubkey,
        action: u8,
        now: i64,
    ) -> Result<Self, NicechunkChunkError> {
        if data.len() != PLAYER_SESSION_LEN || data[0..8] != PLAYER_SESSION_MAGIC {
            return Err(NicechunkChunkError::InvalidPlayerSession);
        }
        if &data[PLAYER_SESSION_AUTHORITY_OFFSET..PLAYER_SESSION_AUTHORITY_OFFSET + 32]
            != session_authority.as_ref()
        {
            return Err(NicechunkChunkError::InvalidSessionAuthority);
        }
        if &data[PLAYER_SESSION_PROFILE_OFFSET..PLAYER_SESSION_PROFILE_OFFSET + 32]
            != player_profile.as_ref()
        {
            return Err(NicechunkChunkError::InvalidPlayerProfile);
        }
        if &data[PLAYER_SESSION_GLOBAL_CONFIG_OFFSET..PLAYER_SESSION_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkChunkError::InvalidGlobalConfig);
        }
        let expires_at = read_i64(data, PLAYER_SESSION_EXPIRES_AT_OFFSET);
        if expires_at <= now {
            return Err(NicechunkChunkError::PlayerSessionExpired);
        }
        let allowed_actions = read_u16(data, PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET);
        if action >= 16 || allowed_actions & (1_u16 << action) == 0 {
            return Err(NicechunkChunkError::SessionActionNotAllowed);
        }
        Ok(Self {
            owner: Pubkey::new_from_array(
                data[PLAYER_SESSION_OWNER_OFFSET..PLAYER_SESSION_OWNER_OFFSET + 32]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidPlayerSession)?,
            ),
        })
    }
}

pub struct BlockChangeArgs {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub previous_block_id: u16,
    pub new_block_id: u16,
    pub action: u8,
    pub tool_slot: u8,
}

#[derive(Clone, Copy)]
pub struct MineBlockArgs {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
    pub expected_block_id: u16,
}

impl MineBlockArgs {
    pub const LEN: usize = 12;
    pub const INSPECT_ONLY_EXPECTED_BLOCK_ID: u16 = u16::MAX;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() != Self::LEN {
            return Err(NicechunkChunkError::InvalidInstruction);
        }
        Ok(Self {
            world_x: read_i32(data, 0),
            world_y: read_i16(data, 4),
            world_z: read_i32(data, 6),
            expected_block_id: read_u16(data, 10),
        })
    }

    pub fn chunk_coords(
        &self,
        global_config: &GlobalConfigView,
    ) -> Result<(i32, i32, u8, u8), NicechunkChunkError> {
        if global_config.chunk_size != 16 {
            return Err(NicechunkChunkError::InvalidGlobalConfigData);
        }
        if self.world_y < global_config.min_build_y || self.world_y > global_config.max_build_y {
            return Err(NicechunkChunkError::InvalidBlockCoordinate);
        }
        let chunk_size = global_config.chunk_size as i32;
        let chunk_x = self.world_x.div_euclid(chunk_size);
        let chunk_z = self.world_z.div_euclid(chunk_size);
        let local_x = self.world_x.rem_euclid(chunk_size) as u8;
        let local_z = self.world_z.rem_euclid(chunk_size) as u8;
        Ok((chunk_x, chunk_z, local_x, local_z))
    }

    pub fn generated_args(
        &self,
        global_config: &GlobalConfigView,
    ) -> Result<GeneratedBlockArgs, NicechunkChunkError> {
        let (chunk_x, chunk_z, local_x, local_z) = self.chunk_coords(global_config)?;
        Ok(GeneratedBlockArgs {
            chunk_x,
            chunk_z,
            local_x,
            y: self.world_y,
            local_z,
            expected_block_id: self.expected_block_id,
        })
    }
}

pub struct ChunkBrokenInitArgs {
    pub bump: u8,
    pub min_y: i16,
    pub capacity: u16,
}

/// Compact per-chunk mined block set.
///
/// This account stores only destroyed generated-block coordinates. Chunk X/Z
/// are not repeated in each record because they are encoded in the PDA seeds.
/// Each block coordinate is packed into three bytes:
/// local_x: 4 bits, local_z: 4 bits, y_offset from min_y: 9 bits, reserved: 7 bits.
pub struct ChunkBrokenState;

impl ChunkBrokenState {
    pub const HEADER_LEN: usize = CHUNK_BROKEN_HEADER_LEN;
    pub const RECORD_LEN: usize = CHUNK_BROKEN_RECORD_LEN;

    pub fn len_for_capacity(capacity: u16) -> usize {
        Self::HEADER_LEN + capacity as usize * Self::RECORD_LEN
    }

    pub fn pack_empty(dst: &mut [u8], args: &ChunkBrokenInitArgs) -> ProgramResult {
        if dst.len() != Self::len_for_capacity(args.capacity) {
            return Err(NicechunkChunkError::InvalidChunkBrokenData.into());
        }
        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&CHUNK_BROKEN_MAGIC)?;
        writer.u8(CHUNK_BROKEN_VERSION)?;
        writer.u8(args.bump)?;
        writer.u16(0)?;
        writer.u16(args.capacity)?;
        writer.i16(args.min_y)?;
        writer.u16(0)?;
        writer.u16(0)?;
        if writer.offset != Self::HEADER_LEN {
            return Err(NicechunkChunkError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_header(data: &[u8], min_y: i16) -> Result<(u16, u16), NicechunkChunkError> {
        if data.len() < Self::HEADER_LEN
            || data[0..4] != CHUNK_BROKEN_MAGIC
            || data[4] != CHUNK_BROKEN_VERSION
        {
            return Err(NicechunkChunkError::InvalidChunkBrokenData);
        }
        let count = read_u16(data, 6);
        let capacity = read_u16(data, 8);
        if read_i16(data, 10) != min_y
            || count > capacity
            || data.len() != Self::len_for_capacity(capacity)
        {
            return Err(NicechunkChunkError::InvalidChunkBrokenData);
        }
        Ok((count, capacity))
    }

    pub fn contains_packed(data: &[u8], packed: [u8; 3]) -> Result<bool, NicechunkChunkError> {
        let count = read_u16(data, 6) as usize;
        for index in 0..count {
            let offset = Self::HEADER_LEN + index * Self::RECORD_LEN;
            if data[offset..offset + Self::RECORD_LEN] == packed {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn append_packed(data: &mut [u8], min_y: i16, packed: [u8; 3]) -> ProgramResult {
        let (count, capacity) = Self::validate_header(data, min_y)?;
        if count >= capacity {
            return Err(NicechunkChunkError::ChunkBrokenCapacityExceeded.into());
        }
        let offset = Self::HEADER_LEN + count as usize * Self::RECORD_LEN;
        data[offset..offset + Self::RECORD_LEN].copy_from_slice(&packed);
        data[6..8].copy_from_slice(&count.saturating_add(1).to_le_bytes());
        Ok(())
    }
}

pub fn pack_broken_coord(
    local_x: u8,
    y: i16,
    local_z: u8,
    min_y: i16,
) -> Result<[u8; 3], NicechunkChunkError> {
    if local_x >= 16 || local_z >= 16 {
        return Err(NicechunkChunkError::InvalidPackedCoordinate);
    }
    let y_offset = y as i32 - min_y as i32;
    if !(0..=CHUNK_BROKEN_MAX_Y_OFFSET).contains(&y_offset) {
        return Err(NicechunkChunkError::InvalidPackedCoordinate);
    }
    let packed = (local_x as u32) | ((local_z as u32) << 4) | ((y_offset as u32) << 8);
    Ok([
        (packed & 0xff) as u8,
        ((packed >> 8) & 0xff) as u8,
        ((packed >> 16) & 0xff) as u8,
    ])
}

impl BlockChangeArgs {
    pub const LEN: usize = 18;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() != Self::LEN {
            return Err(NicechunkChunkError::InvalidInstruction);
        }
        Ok(Self {
            chunk_x: read_i32(data, 0),
            chunk_z: read_i32(data, 4),
            local_x: data[8],
            y: read_i16(data, 9),
            local_z: data[11],
            previous_block_id: read_u16(data, 12),
            new_block_id: read_u16(data, 14),
            action: data[16],
            tool_slot: data[17],
        })
    }

    pub fn validate(&self, global_config: &GlobalConfigView) -> ProgramResult {
        if self.local_x as u16 >= global_config.chunk_size
            || self.local_z as u16 >= global_config.chunk_size
            || self.y < global_config.min_build_y
            || self.y > global_config.max_build_y
        {
            return Err(NicechunkChunkError::InvalidBlockCoordinate.into());
        }
        if self.previous_block_id == self.new_block_id {
            return Err(NicechunkChunkError::InvalidBlockChange.into());
        }
        Ok(())
    }
}

pub struct ChunkInitArgs<'a> {
    pub bump: u8,
    pub global_config: &'a Pubkey,
    pub world_id: u16,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub created_slot: u64,
    pub created_at: i64,
}

/// Per-chunk public delta log.
///
/// The generated terrain itself stays off-chain. This account only records
/// player-authored block changes. The fixed-size ring buffer keeps the first
/// version compact; future settlement/indexer programs can archive or shard
/// history without changing the genesis world configuration.
pub struct ChunkState;

impl ChunkState {
    pub const HEADER_LEN: usize = 88;
    pub const LEN: usize = Self::HEADER_LEN + MAX_BLOCK_DELTAS * BLOCK_DELTA_LEN;

    pub fn pack_empty(dst: &mut [u8], args: &ChunkInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkChunkError::InvalidChunkData.into());
        }

        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&CHUNK_MAGIC)?;
        writer.u16(CHUNK_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.pubkey(args.global_config)?;
        writer.u16(args.world_id)?;
        writer.i32(args.chunk_x)?;
        writer.i32(args.chunk_z)?;
        writer.u32(0)?;
        writer.u16(0)?;
        writer.u16(0)?;
        writer.u16(MAX_BLOCK_DELTAS as u16)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;

        if writer.offset != Self::HEADER_LEN {
            return Err(NicechunkChunkError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_header(
        data: &[u8],
        global_config: &Pubkey,
        world_id: u16,
        chunk_x: i32,
        chunk_z: i32,
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != CHUNK_MAGIC {
            return Err(NicechunkChunkError::InvalidChunkData.into());
        }
        if &data[12..44] != global_config.as_ref() {
            return Err(NicechunkChunkError::InvalidGlobalConfig.into());
        }
        if read_u16(data, 44) != world_id
            || read_i32(data, 46) != chunk_x
            || read_i32(data, 50) != chunk_z
        {
            return Err(NicechunkChunkError::InvalidChunkData.into());
        }
        Ok(())
    }

    pub fn append_delta(
        data: &mut [u8],
        args: &BlockChangeArgs,
        authority: &Pubkey,
        slot: u64,
        timestamp: i64,
    ) -> ProgramResult {
        if data.len() != Self::LEN {
            return Err(NicechunkChunkError::InvalidChunkData.into());
        }
        let change_count = read_u32(data, 54);
        let stored_delta_count = read_u16(data, 58);
        let write_cursor = read_u16(data, 60);
        let max_deltas = read_u16(data, 62);
        if max_deltas as usize != MAX_BLOCK_DELTAS {
            return Err(NicechunkChunkError::InvalidChunkData.into());
        }

        let next_sequence = change_count
            .checked_add(1)
            .ok_or(NicechunkChunkError::InvalidChunkData)?;
        let delta_index = write_cursor as usize;
        let delta_offset = Self::HEADER_LEN + delta_index * BLOCK_DELTA_LEN;
        let next_cursor = ((write_cursor as usize + 1) % MAX_BLOCK_DELTAS) as u16;
        let next_stored_count = stored_delta_count
            .saturating_add(1)
            .min(MAX_BLOCK_DELTAS as u16);

        data[54..58].copy_from_slice(&next_sequence.to_le_bytes());
        data[58..60].copy_from_slice(&next_stored_count.to_le_bytes());
        data[60..62].copy_from_slice(&next_cursor.to_le_bytes());
        data[72..80].copy_from_slice(&slot.to_le_bytes());

        let mut writer = ByteWriter {
            dst: &mut data[delta_offset..delta_offset + BLOCK_DELTA_LEN],
            offset: 0,
        };
        writer.u32(next_sequence)?;
        writer.pubkey(authority)?;
        writer.u8(args.local_x)?;
        writer.i16(args.y)?;
        writer.u8(args.local_z)?;
        writer.u16(args.previous_block_id)?;
        writer.u16(args.new_block_id)?;
        writer.u8(args.action)?;
        writer.u8(args.tool_slot)?;
        writer.u64(slot)?;
        writer.i64(timestamp)?;
        writer.bytes(&[0_u8; 2])?;

        if writer.offset != BLOCK_DELTA_LEN {
            return Err(NicechunkChunkError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn current_block_id_at(
        data: &[u8],
        global_config: &GlobalConfigView,
        args: &BlockChangeArgs,
    ) -> Result<u16, NicechunkChunkError> {
        if data.len() != Self::LEN {
            return Err(NicechunkChunkError::InvalidChunkData);
        }

        let stored_delta_count = read_u16(data, 58) as usize;
        let write_cursor = read_u16(data, 60) as usize;
        let max_deltas = read_u16(data, 62) as usize;
        if max_deltas != MAX_BLOCK_DELTAS || stored_delta_count > MAX_BLOCK_DELTAS {
            return Err(NicechunkChunkError::InvalidChunkData);
        }

        for age in 0..stored_delta_count {
            let delta_index = (write_cursor + max_deltas - 1 - age) % max_deltas;
            let offset = Self::HEADER_LEN + delta_index * BLOCK_DELTA_LEN;
            let local_x = data[offset + 36];
            let y = read_i16(data, offset + 37);
            let local_z = data[offset + 39];
            if local_x == args.local_x && y == args.y && local_z == args.local_z {
                return Ok(read_u16(data, offset + 42));
            }
        }

        let generated_args = GeneratedBlockArgs {
            chunk_x: args.chunk_x,
            chunk_z: args.chunk_z,
            local_x: args.local_x,
            y: args.y,
            local_z: args.local_z,
            expected_block_id: GeneratedBlockArgs::INSPECT_ONLY_EXPECTED_BLOCK_ID,
        };
        Ok(generated_block_id_at(global_config, &generated_args))
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
            return Err(NicechunkChunkError::PackSizeMismatch.into());
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

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_i16(data: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([data[offset], data[offset + 1]])
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

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_global_config_view() -> GlobalConfigView {
        GlobalConfigView {
            world_id: 1,
            world_seed: [7_u8; 32],
            chunk_size: 16,
            min_build_y: -32,
            max_build_y: 256,
            max_terrain_height: 160,
            sea_level: 2,
        }
    }

    #[test]
    fn chunk_state_len_matches_pack() {
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; ChunkState::LEN];
        ChunkState::pack_empty(
            &mut data,
            &ChunkInitArgs {
                bump: 251,
                global_config: &global_config,
                world_id: 1,
                chunk_x: -2,
                chunk_z: 3,
                created_slot: 123,
                created_at: 456,
            },
        )
        .unwrap();

        assert_eq!(&data[0..8], &CHUNK_MAGIC);
        assert_eq!(data[10], 251);
        assert_eq!(&data[12..44], global_config.as_ref());
        assert_eq!(
            u16::from_le_bytes(data[62..64].try_into().unwrap()),
            MAX_BLOCK_DELTAS as u16
        );
        assert_eq!(u64::from_le_bytes(data[64..72].try_into().unwrap()), 123);
        assert_eq!(u64::from_le_bytes(data[72..80].try_into().unwrap()), 123);
    }

    #[test]
    fn append_delta_updates_ring_header() {
        let global_config = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mut data = vec![0_u8; ChunkState::LEN];
        ChunkState::pack_empty(
            &mut data,
            &ChunkInitArgs {
                bump: 251,
                global_config: &global_config,
                world_id: 1,
                chunk_x: 0,
                chunk_z: 0,
                created_slot: 123,
                created_at: 456,
            },
        )
        .unwrap();
        let args = BlockChangeArgs {
            chunk_x: 0,
            chunk_z: 0,
            local_x: 1,
            y: 2,
            local_z: 3,
            previous_block_id: 4,
            new_block_id: 0,
            action: 1,
            tool_slot: 0,
        };

        ChunkState::append_delta(&mut data, &args, &authority, 124, 457).unwrap();

        assert_eq!(u32::from_le_bytes(data[54..58].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(data[58..60].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(data[60..62].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(data[88..92].try_into().unwrap()), 1);
        assert_eq!(&data[92..124], authority.as_ref());
    }

    #[test]
    fn current_block_id_uses_latest_delta_before_generated_block() {
        let config = test_global_config_view();
        let global_config = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mut data = vec![0_u8; ChunkState::LEN];
        ChunkState::pack_empty(
            &mut data,
            &ChunkInitArgs {
                bump: 251,
                global_config: &global_config,
                world_id: 1,
                chunk_x: 0,
                chunk_z: 0,
                created_slot: 123,
                created_at: 456,
            },
        )
        .unwrap();
        let args = BlockChangeArgs {
            chunk_x: 0,
            chunk_z: 0,
            local_x: 1,
            y: 2,
            local_z: 3,
            previous_block_id: BLOCK_STONE,
            new_block_id: BLOCK_AIR,
            action: 1,
            tool_slot: 0,
        };

        ChunkState::append_delta(&mut data, &args, &authority, 124, 457).unwrap();

        assert_eq!(
            ChunkState::current_block_id_at(&data, &config, &args).unwrap(),
            BLOCK_AIR
        );
    }

    #[test]
    fn generated_surface_height_is_stable() {
        let config = test_global_config_view();
        let first = generated_surface_height(&config, 12, -34);
        let second = generated_surface_height(&config, 12, -34);
        assert_eq!(first, second);
        assert!(first >= config.min_build_y + 8);
        assert!(first <= config.max_terrain_height);
    }

    #[test]
    fn generated_block_id_matches_basic_layers() {
        let config = test_global_config_view();
        let surface = generated_surface_height(&config, 1, 2);
        let at_surface = GeneratedBlockArgs {
            chunk_x: 0,
            chunk_z: 0,
            local_x: 1,
            y: surface,
            local_z: 2,
            expected_block_id: BLOCK_GRASS,
        };
        let below_surface = GeneratedBlockArgs {
            y: surface - 1,
            expected_block_id: BLOCK_DIRT,
            ..at_surface
        };
        let deep = GeneratedBlockArgs {
            y: config.min_build_y + 1,
            expected_block_id: BLOCK_DEEP_STONE,
            ..at_surface
        };
        let bedrock = GeneratedBlockArgs {
            y: config.min_build_y,
            expected_block_id: BLOCK_BEDROCK,
            ..at_surface
        };

        assert_eq!(generated_block_id_at(&config, &at_surface), BLOCK_GRASS);
        assert_eq!(generated_block_id_at(&config, &below_surface), BLOCK_DIRT);
        assert_eq!(generated_block_id_at(&config, &deep), BLOCK_DEEP_STONE);
        assert_eq!(generated_block_id_at(&config, &bedrock), BLOCK_BEDROCK);
    }

    #[test]
    fn generated_block_args_rejects_out_of_range_local_coordinate() {
        let config = test_global_config_view();
        let args = GeneratedBlockArgs {
            chunk_x: 0,
            chunk_z: 0,
            local_x: 16,
            y: 0,
            local_z: 0,
            expected_block_id: BLOCK_AIR,
        };
        assert!(args.validate(&config).is_err());
    }

    #[test]
    fn mine_block_args_uses_floor_chunk_coords_for_negative_world_coords() {
        let config = test_global_config_view();
        let args = MineBlockArgs {
            world_x: -1,
            world_y: 8,
            world_z: -17,
            expected_block_id: BLOCK_STONE,
        };
        assert_eq!(args.chunk_coords(&config).unwrap(), (-1, -2, 15, 15));
    }

    #[test]
    fn chunk_broken_state_len_and_header_are_compact() {
        assert_eq!(ChunkBrokenState::len_for_capacity(0), 16);
        assert_eq!(ChunkBrokenState::len_for_capacity(64), 208);
        assert_eq!(ChunkBrokenState::len_for_capacity(128), 400);

        let mut data = vec![0_u8; ChunkBrokenState::len_for_capacity(64)];
        ChunkBrokenState::pack_empty(
            &mut data,
            &ChunkBrokenInitArgs {
                bump: 252,
                min_y: -32,
                capacity: 64,
            },
        )
        .unwrap();
        assert_eq!(&data[0..4], &CHUNK_BROKEN_MAGIC);
        assert_eq!(data[4], CHUNK_BROKEN_VERSION);
        assert_eq!(data[5], 252);
        assert_eq!(u16::from_le_bytes(data[6..8].try_into().unwrap()), 0);
        assert_eq!(u16::from_le_bytes(data[8..10].try_into().unwrap()), 64);
        assert_eq!(i16::from_le_bytes(data[10..12].try_into().unwrap()), -32);
        assert_eq!(
            ChunkBrokenState::validate_header(&data, -32).unwrap(),
            (0, 64)
        );
    }

    #[test]
    fn packed_broken_coord_is_three_bytes_and_detects_duplicates() {
        let mut data = vec![0_u8; ChunkBrokenState::len_for_capacity(2)];
        ChunkBrokenState::pack_empty(
            &mut data,
            &ChunkBrokenInitArgs {
                bump: 252,
                min_y: -32,
                capacity: 2,
            },
        )
        .unwrap();
        let packed = pack_broken_coord(15, -31, 7, -32).unwrap();
        assert_eq!(packed, [0x7f, 0x01, 0x00]);
        assert!(!ChunkBrokenState::contains_packed(&data, packed).unwrap());
        ChunkBrokenState::append_packed(&mut data, -32, packed).unwrap();
        assert!(ChunkBrokenState::contains_packed(&data, packed).unwrap());
        assert_eq!(u16::from_le_bytes(data[6..8].try_into().unwrap()), 1);
    }

    #[test]
    fn packed_broken_coord_rejects_out_of_range_y() {
        assert!(pack_broken_coord(0, -33, 0, -32).is_err());
        assert!(pack_broken_coord(0, 480, 0, -32).is_err());
    }
}
