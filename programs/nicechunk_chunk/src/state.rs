use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkChunkError;

pub const CHUNK_BROKEN_MAGIC: [u8; 4] = *b"NCBK";
pub const CHUNK_BROKEN_VERSION: u8 = 1;
pub const CHUNK_BROKEN_SEED: &[u8] = b"chunk-broken";
pub const CHUNK_BROKEN_HEADER_LEN: usize = 16;
pub const CHUNK_BROKEN_RECORD_LEN: usize = 3;
pub const CHUNK_BROKEN_INITIAL_CAPACITY: u16 = 64;
pub const CHUNK_BROKEN_GROW_BY: u16 = 64;
pub const CHUNK_BROKEN_MAX_CAPACITY: u16 = 2048;
pub const CHUNK_BROKEN_MAX_Y_OFFSET: i32 = 511;
pub const RESOURCE_DROP_TABLE_MAGIC: [u8; 8] = *b"NCKDRP02";
pub const RESOURCE_DROP_TABLE_VERSION: u8 = 2;
pub const RESOURCE_DROP_TABLE_SEED: &[u8] = b"resource-drops-v2";
pub const RESOURCE_DROP_TABLE_HEADER_LEN: usize = 16;
pub const RESOURCE_DROP_RULE_LEN: usize = 23;
pub const RESOURCE_DROP_RULE_MAX_COUNT: usize = 64;
pub const RESOURCE_DROP_CHANCE_DENOMINATOR: u32 = 10_000;
pub const SURFACE_DECORATION_TABLE_MAGIC: [u8; 8] = *b"NCKDEC01";
pub const SURFACE_DECORATION_TABLE_VERSION: u8 = 1;
pub const SURFACE_DECORATION_TABLE_SEED: &[u8] = b"surface-decor-v1";
pub const SURFACE_DECORATION_TABLE_HEADER_LEN: usize = 16;
pub const SURFACE_DECORATION_RULE_LEN: usize = 20;
pub const SURFACE_DECORATION_RULE_MAX_COUNT: usize = 128;
pub const SURFACE_DECORATION_TABLE_LEN: usize = SURFACE_DECORATION_TABLE_HEADER_LEN
    + SURFACE_DECORATION_RULE_MAX_COUNT * SURFACE_DECORATION_RULE_LEN;
pub const SURFACE_DECORATION_ROLL_DENOMINATOR: u32 = 10_000;
pub const SURFACE_DECORATION_FLAG_MINEABLE: u8 = 1 << 1;
pub const BACKPACK_PACKED_Y_BITS: i32 = 9;
pub const BACKPACK_PACKED_Y_MASK: i32 = (1 << BACKPACK_PACKED_Y_BITS) - 1;
pub const FOUNDATION_CHUNK_MAGIC: [u8; 8] = *b"NCKFCI01";
pub const FOUNDATION_CHUNK_VERSION: u8 = 1;
pub const FOUNDATION_CHUNK_SEED: &[u8] = b"foundation-chunk";
pub const FOUNDATION_CHUNK_HEADER_LEN: usize = 52;
pub const FOUNDATION_CHUNK_RECORD_LEN: usize = 52;
pub const FOUNDATION_CHUNK_CAPACITY: u16 = 32;
pub const FOUNDATION_CHUNK_LEN: usize =
    FOUNDATION_CHUNK_HEADER_LEN + FOUNDATION_CHUNK_CAPACITY as usize * FOUNDATION_CHUNK_RECORD_LEN;
pub const FOUNDATION_CHUNK_V2_MAGIC: [u8; 8] = *b"NCKFCI02";
pub const FOUNDATION_CHUNK_V2_VERSION: u8 = 2;
pub const FOUNDATION_CHUNK_V2_HEADER_LEN: usize = 56;
pub const FOUNDATION_CHUNK_V2_RECORD_LEN: usize = 58;
pub const FOUNDATION_CHUNK_V2_INITIAL_CAPACITY: u16 = 4;
pub const FOUNDATION_CHUNK_V2_GROWTH: u16 = 4;
pub const FOUNDATION_CHUNK_V2_MAX_CAPACITY: u16 = 64;
pub const FOUNDATION_MIN_SIZE: u8 = 2;
pub const FOUNDATION_MAX_SIZE: u8 = 16;
pub const GLOBAL_CONFIG_LEN: usize = 293;
pub const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
pub const GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET: usize = 53;
// These values are the current chunk.js canonical world. The legacy Core
// account remains an identity anchor only and never supplies terrain values.
pub const CANONICAL_WORLD_SEED: [u8; 32] = [
    110, 105, 99, 101, 99, 104, 117, 110, 107, 45, 109, 97, 105, 110, 110, 101, 116, 45, 48, 48,
    49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
pub const CANONICAL_CHUNK_SIZE: u16 = 16;
pub const CANONICAL_MIN_BUILD_Y: i16 = -32;
pub const CANONICAL_MAX_BUILD_Y: i16 = 320;
pub const CANONICAL_MAX_TERRAIN_HEIGHT: i16 = 240;
pub const CANONICAL_SEA_LEVEL: i16 = 96;

pub const BLOCK_AIR: u16 = 0;
pub const BLOCK_GRASS: u16 = 1;
pub const BLOCK_DIRT: u16 = 2;
pub const BLOCK_STONE: u16 = 3;
pub const BLOCK_DEEP_STONE: u16 = 4;
pub const BLOCK_SAND: u16 = 5;
pub const BLOCK_GRAVEL: u16 = 6;
pub const BLOCK_CLAY: u16 = 7;
pub const BLOCK_MUD: u16 = 8;
pub const BLOCK_DRY_DIRT: u16 = 9;
pub const BLOCK_SALT_FLAT: u16 = 10;
pub const BLOCK_SNOW: u16 = 11;
pub const BLOCK_FROZEN_SOIL: u16 = 13;
pub const BLOCK_BASALT: u16 = 14;
pub const BLOCK_ASH: u16 = 15;
pub const BLOCK_BEDROCK: u16 = 16;
pub const BLOCK_WATER: u16 = 17;
pub const BLOCK_QUICKSAND: u16 = 21;
pub const BLOCK_TRUNK: u16 = 22;
pub const BLOCK_LEAVES: u16 = 23;
pub const BLOCK_PINE_TRUNK: u16 = 24;
pub const BLOCK_PINE_LEAVES: u16 = 25;
pub const BLOCK_MOSS: u16 = 37;
pub const BLOCK_SHELL_BED: u16 = 46;
pub const BLOCK_COAL: u16 = 47;
const TREE_MAX_LEAF_RADIUS: i32 = 2;
const TREE_MAX_BLOCK_HEIGHT_ABOVE_SURFACE: i16 = 9;
const MAX_WATER_LEVEL_ABOVE_SEA: i16 = 6;
pub const TREE_FELL_MAX_CHUNKS: usize = 4;
pub const TREE_FELL_MAX_BLOCKS: usize = ((TREE_MAX_LEAF_RADIUS * 2 + 1) as usize)
    * ((TREE_MAX_LEAF_RADIUS * 2 + 1) as usize)
    * TREE_MAX_BLOCK_HEIGHT_ABOVE_SURFACE as usize;
// Canonical terrain verification is intentionally expensive. Two blocks keep
// one arbitrary-volume batch below Solana's transaction CU ceiling while the
// client can submit larger selections as a sequence of atomic batches.
pub const BATCH_MINE_MAX_BLOCKS: usize = 2;
pub const BATCH_MINE_MODE_DEBUG: u8 = 1;
pub const BATCH_MINE_BASE_DROP_CHANCE_BPS: u16 = 3_500;
pub const BATCH_MINE_DECORATION_DROP_CHANCE_BPS: u16 = 2_500;
pub const BATCH_MINE_EXTRA_DROP_CHANCE_BPS: u16 = 5_000;
pub const RANGE_MINE_MAX_BLOCKS: usize = 640;
pub const RANGE_MINE_MODE_DEBUG: u8 = 1;
pub const RANGE_MINE_MODE_EXPLOSIVE: u8 = 2;
pub const RANGE_MINE_BASE_DROP_CHANCE_BPS: u16 = 500;
pub const RANGE_MINE_SECONDARY_CANDIDATE_CHANCE_BPS: u16 = 100;
pub const RANGE_MINE_SECONDARY_PROOF_LIMIT: usize = 2;
pub const RANGE_MINE_MAX_REWARDS: usize = 16;

pub const PLAYER_PROFILE_LEN: usize = 773;
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
pub const PLAYER_PROGRESS_MAGIC: [u8; 8] = *b"NCKPRG01";
pub const PLAYER_PROGRESS_VERSION: u16 = 1;
pub const PLAYER_PROGRESS_SEED: &[u8] = b"player-progress";
pub const PLAYER_PROGRESS_LEN: usize = 128;
pub const PLAYER_PROGRESS_OWNER_OFFSET: usize = 12;
pub const PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET: usize = 44;
pub const PLAYER_PROGRESS_PRECISION_XP_OFFSET: usize = 76;
pub const PLAYER_PROGRESS_CREATED_SLOT_OFFSET: usize = 84;
pub const PLAYER_PROGRESS_UPDATED_SLOT_OFFSET: usize = 92;
pub const PLAYER_PROGRESS_CREATED_AT_OFFSET: usize = 100;
pub const PLAYER_PROGRESS_SMELTING_XP_OFFSET: usize = 108;
pub const PLAYER_PROGRESS_EXPLORATION_XP_OFFSET: usize = 116;
pub const PLAYER_PROGRESS_EXPLORED_CHUNK_COUNT_OFFSET: usize = 124;
pub const PRECISION_GATHERING_XP_PER_BLOCK: u64 = 1;
pub const EXPLORATION_XP_PER_EXTRA_DROP: u64 = 1;
pub const RESOURCE_BLOCK_VOLUME_MM3: u32 = 1_000_000;
pub const PRECISION_GATHERING_TOTAL_XP_BY_LEVEL: [u64; 11] = [
    0, 900, 3_481, 8_261, 15_663, 26_054, 39_764, 57_094, 78_323, 103_715, 133_517,
];
pub const EXPLORATION_TOTAL_XP_BY_LEVEL: [u64; 11] = [
    0, 1_250, 4_961, 11_975, 22_994, 38_636, 59_462, 85_991, 118_707, 158_068, 204_510,
];

pub struct GlobalConfigView {
    pub development_wallet: Pubkey,
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
        // Player/Profile/Backpack PDAs still bind to this account address, but
        // all generated block results come from the current chunk.js world.
        Ok(Self {
            development_wallet: Pubkey::new_from_array(
                data[GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET
                    ..GLOBAL_CONFIG_DEVELOPMENT_WALLET_OFFSET + 32]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidGlobalConfigData)?,
            ),
            world_seed: CANONICAL_WORLD_SEED,
            chunk_size: CANONICAL_CHUNK_SIZE,
            min_build_y: CANONICAL_MIN_BUILD_Y,
            max_build_y: CANONICAL_MAX_BUILD_Y,
            max_terrain_height: CANONICAL_MAX_TERRAIN_HEIGHT,
            sea_level: CANONICAL_SEA_LEVEL,
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
}

impl GeneratedBlockArgs {
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

pub fn generated_block_id_at(global_config: &GlobalConfigView, args: &GeneratedBlockArgs) -> u16 {
    let world_x = args.world_x(global_config);
    let world_z = args.world_z(global_config);
    let surface = generated_surface_height(global_config, world_x, world_z);

    if args.y <= global_config.min_build_y {
        return BLOCK_BEDROCK;
    }
    if args.y > global_config.max_build_y {
        return BLOCK_AIR;
    }
    if args.y > surface {
        if args.y
            <= global_config
                .sea_level
                .saturating_add(MAX_WATER_LEVEL_ABOVE_SEA)
        {
            let water_level = generated_water_level(global_config, world_x, world_z, surface);
            if water_level.map(|level| args.y <= level).unwrap_or(false) {
                return BLOCK_WATER;
            }
        }
        let tree_block = generated_tree_block_id_at(global_config, world_x, args.y, world_z);
        if tree_block != BLOCK_AIR {
            return tree_block;
        }
        return BLOCK_AIR;
    }
    if args.y == surface {
        return generated_surface_block_id(global_config, world_x, world_z, surface);
    }

    let depth = surface.saturating_sub(args.y);
    if depth <= 3 {
        return generated_subsurface_block_id(global_config, world_x, world_z, surface);
    }
    if depth >= 8 && generated_coal_seam_at(global_config, world_x, args.y, world_z, surface) {
        return BLOCK_COAL;
    }
    if args.y <= global_config.min_build_y.saturating_add(40) || depth >= 52 {
        return BLOCK_DEEP_STONE;
    }
    if generated_volcanic_at(global_config, world_x, world_z) > 238
        && hash_coord3(
            &global_config.world_seed,
            world_x,
            args.y as i32,
            world_z,
            601,
        ) > 210
    {
        return BLOCK_BASALT;
    }
    BLOCK_STONE
}

pub fn generated_block_id_at_world(
    global_config: &GlobalConfigView,
    world_x: i32,
    y: i16,
    world_z: i32,
) -> Result<u16, NicechunkChunkError> {
    if global_config.chunk_size != 16 {
        return Err(NicechunkChunkError::InvalidGlobalConfigData);
    }
    if y < global_config.min_build_y || y > global_config.max_build_y {
        return Err(NicechunkChunkError::InvalidBlockCoordinate);
    }
    let chunk_size = global_config.chunk_size as i32;
    let args = GeneratedBlockArgs {
        chunk_x: world_x.div_euclid(chunk_size),
        chunk_z: world_z.div_euclid(chunk_size),
        local_x: world_x.rem_euclid(chunk_size) as u8,
        y,
        local_z: world_z.rem_euclid(chunk_size) as u8,
    };
    Ok(generated_block_id_at(global_config, &args))
}

#[derive(Clone, Copy)]
pub struct TreeFellBlock {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
    pub block_id: u16,
}

pub fn is_tree_trunk_block(block_id: u16) -> bool {
    block_id == BLOCK_TRUNK || block_id == BLOCK_PINE_TRUNK
}

pub fn is_tree_leaf_block(block_id: u16) -> bool {
    block_id == BLOCK_LEAVES || block_id == BLOCK_PINE_LEAVES
}

pub fn generated_tree_fell_blocks(
    global_config: &GlobalConfigView,
    cut_x: i32,
    cut_y: i16,
    cut_z: i32,
) -> Result<Vec<TreeFellBlock>, NicechunkChunkError> {
    let surface = generated_surface_height(global_config, cut_x, cut_z);
    let tree = generated_tree_at(global_config, cut_x, cut_z, surface);
    if !tree.exists {
        return Err(NicechunkChunkError::GeneratedBlockMismatch);
    }
    if cut_y < tree.base_y || cut_y >= tree.base_y.saturating_add(tree.trunk_height) {
        return Err(NicechunkChunkError::GeneratedBlockMismatch);
    }

    let top_y = surface.saturating_add(TREE_MAX_BLOCK_HEIGHT_ABOVE_SURFACE);
    let mut blocks = Vec::with_capacity(TREE_FELL_MAX_BLOCKS);
    for y in cut_y..=top_y {
        for z in
            cut_z.saturating_sub(TREE_MAX_LEAF_RADIUS)..=cut_z.saturating_add(TREE_MAX_LEAF_RADIUS)
        {
            for x in cut_x.saturating_sub(TREE_MAX_LEAF_RADIUS)
                ..=cut_x.saturating_add(TREE_MAX_LEAF_RADIUS)
            {
                let tree_block = generated_tree_volume_block(global_config, &tree, x, y, z);
                if tree_block == BLOCK_AIR {
                    continue;
                }
                blocks.push(TreeFellBlock {
                    world_x: x,
                    world_y: y,
                    world_z: z,
                    block_id: tree_block,
                });
            }
        }
    }

    if blocks.is_empty() || blocks.len() > TREE_FELL_MAX_BLOCKS {
        return Err(NicechunkChunkError::InvalidInstruction);
    }
    Ok(blocks)
}

fn generated_coal_seam_at(
    global_config: &GlobalConfigView,
    world_x: i32,
    y: i16,
    world_z: i32,
    surface: i16,
) -> bool {
    let depth = surface.saturating_sub(y);
    if depth < 10 || y <= global_config.min_build_y.saturating_add(4) {
        return false;
    }
    if depth > 92 && y < global_config.min_build_y.saturating_add(12) {
        return false;
    }

    let layer_y = div_floor_i32((y - global_config.min_build_y) as i32, 6);
    let cell_x = div_floor_i32(world_x, 10);
    let cell_z = div_floor_i32(world_z, 10);
    let band = hash_coord3(&global_config.world_seed, cell_x, layer_y, cell_z, 301) & 255;
    if band < 214 {
        return false;
    }
    if !(18..=76).contains(&depth) {
        return false;
    }

    let lens = hash_coord3(
        &global_config.world_seed,
        world_x.saturating_add(layer_y.saturating_mul(17)),
        y as i32,
        world_z.saturating_sub(layer_y.saturating_mul(13)),
        302,
    ) & 255;
    let vein = hash_coord3(
        &global_config.world_seed,
        div_floor_i32(world_x.saturating_add((y as i32).saturating_mul(2)), 4),
        layer_y,
        div_floor_i32(world_z.saturating_sub((y as i32).saturating_mul(3)), 4),
        303,
    ) & 255;
    lens.saturating_add(vein / 2) >= 228
}

pub fn generated_surface_height(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
) -> i16 {
    let max_surface = global_config.min_build_y.saturating_add(8).max(
        global_config
            .max_terrain_height
            .min(global_config.max_build_y.saturating_sub(1)),
    );
    let desired_min_surface = global_config
        .min_build_y
        .saturating_add(8)
        .max(global_config.sea_level.saturating_sub(28));
    let min_surface = desired_min_surface.min(max_surface);

    let terrain = generated_terrain_factors(global_config, world_x, world_z);
    let wx = terrain.wx;
    let wz = terrain.wz;
    let shelf = terrain.shelf;
    let inland = terrain.inland;

    let ocean = global_config.sea_level as i32 - 16
        + ((value_noise2(&global_config.world_seed, wx, wz, 96, 24) as i32 - 128) * 5) / 128
        + ((value_noise2(&global_config.world_seed, wx, wz, 36, 25) as i32 - 128) * 2) / 128;
    let coast = global_config.sea_level as i32 - 3 + (shelf as i32 * 8) / 1024;
    let plains =
        ((value_noise2(&global_config.world_seed, wx, wz, 120, 26) as i32 - 128) * 4) / 128;
    let hills = ((value_noise2(&global_config.world_seed, wx, wz, 56, 27) as i32 - 128) * 7) / 128;
    let rolling =
        ((value_noise2(&global_config.world_seed, wx, wz, 28, 28) as i32 - 128) * 2) / 128;
    let roughness = smooth_range_fixed(
        (value_noise2(&global_config.world_seed, wx, wz, 180, 40) as i32 - 128).abs(),
        54,
        122,
    );

    let mountain_range = scale_by_fixed(
        smooth_range_fixed(
            value_noise2(&global_config.world_seed, wx, wz, 360, 30) as i32,
            136,
            226,
        ) as i32,
        inland,
    ) as u32;
    let highland = scale_by_fixed(
        34,
        scale_by_fixed(
            smooth_range_fixed(
                value_noise2(&global_config.world_seed, wx, wz, 620, 46) as i32,
                116,
                206,
            ) as i32,
            inland,
        ) as u32,
    );
    let ridge_line =
        128 - (value_noise2(&global_config.world_seed, wx, wz, 92, 29) as i32 - 128).abs();
    let ridge_lift = smooth_range_fixed(ridge_line, 44, 126);
    let peak_mask = scale_by_fixed(
        smooth_range_fixed(
            value_noise2(&global_config.world_seed, wx, wz, 176, 47) as i32,
            176,
            242,
        ) as i32,
        mountain_range,
    ) as u32;
    let crag = scale_by_fixed(
        smooth_range_fixed(
            (value_noise2(&global_config.world_seed, wx, wz, 52, 48) as i32 - 128).abs(),
            48,
            126,
        ) as i32,
        mountain_range,
    ) as u32;
    let mountain = highland
        + scale_by_fixed(
            24 + scale_by_fixed(72, ridge_lift) + scale_by_fixed(24, crag),
            mountain_range,
        )
        + scale_by_fixed(34, peak_mask);

    let land = global_config.sea_level as i32
        + 7
        + (inland as i32 * 8) / 1024
        + scale_by_fixed(plains + scale_by_fixed(hills + rolling, roughness), inland)
        + mountain;
    let mut shaped_land = coast.max(land);
    if terrain.floodplain_mask > 0 {
        let flat_noise =
            (value_noise2(&global_config.world_seed, wx, wz, 54, 38) as i32 - 128) / 128;
        let floodplain_lift = 2 + ((1024 - terrain.open_river as i32) * 2) / 1024;
        let floodplain_floor = global_config.sea_level as i32 + floodplain_lift + flat_noise;
        let floodplain_blend = ((terrain.floodplain_mask as i32
            * (720 + (terrain.open_river as i32 * 420) / 1024))
            / 1024)
            .min(1024) as u32;
        shaped_land = lerp_i32_fixed(
            shaped_land,
            shaped_land.min(floodplain_floor),
            floodplain_blend,
        );
    }
    if terrain.valley_mask > 0 {
        let bed_noise =
            ((value_noise2(&global_config.world_seed, wx, wz, 32, 39) as i32 - 128) * 2) / 128;
        let slope_noise = value_noise2(&global_config.world_seed, wx, wz, 150, 42) as i32;
        let open_flatten = scale_by_fixed(terrain.open_river as i32, 640);
        let canyon = scale_by_fixed(
            smooth_range_fixed(slope_noise, 190, 252) as i32,
            (1024 - open_flatten).max(0) as u32,
        );
        let gentle =
            1024 - scale_by_fixed(terrain.open_river as i32, (1024 - canyon).max(0) as u32);
        let slope_strength = 220
            + scale_by_fixed(360, canyon.max(0) as u32)
            + scale_by_fixed(90, gentle.max(0) as u32);
        let valley_blend = ((terrain.valley_mask as i32 * slope_strength) / 1024).min(1024) as u32;
        let bank_lift = 2 + ((255 - terrain.valley_softness as i32) * 4) / 255;
        let valley_cut = (terrain.valley_mask as i32 * (1 + slope_noise / 86)) / 1024;
        let bank_floor = global_config.sea_level as i32 + bank_lift - valley_cut + bed_noise;
        shaped_land = lerp_i32_fixed(shaped_land, shaped_land.min(bank_floor), valley_blend);

        let core_start = 84 + ((255 - slope_noise) * 172) / 255;
        let core_blend = smooth_range_fixed(terrain.water_mask as i32, core_start, 1024);
        if core_blend > 0 {
            let water_bed = global_config.sea_level as i32
                - 1
                - (terrain.water_mask as i32 * 4) / 1024
                - (terrain.lake as i32 * 3) / 1024
                + bed_noise;
            shaped_land = lerp_i32_fixed(shaped_land, water_bed, core_blend);
        }
    }

    lerp_i32_fixed(ocean, shaped_land, shelf).clamp(min_surface as i32, max_surface as i32) as i16
}

fn generated_surface_block_id(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> u16 {
    let water_level = generated_water_level(global_config, world_x, world_z, surface);
    let underwater = water_level.map(|level| surface < level).unwrap_or(false);
    let moisture = generated_moisture_at(global_config, world_x, world_z);
    let desert = generated_desert_score_at(global_config, world_x, world_z);
    let gravel_patch = value_noise2(&global_config.world_seed, world_x, world_z, 44, 103);
    let clay_patch = value_noise2(&global_config.world_seed, world_x, world_z, 52, 104);

    if underwater || surface <= global_config.sea_level.saturating_add(1) {
        if moisture > 190 && clay_patch > 148 {
            return BLOCK_CLAY;
        }
        if gravel_patch > 218 {
            return BLOCK_GRAVEL;
        }
        if value_noise2(&global_config.world_seed, world_x, world_z, 96, 105) > 236 {
            return BLOCK_SHELL_BED;
        }
        return BLOCK_SAND;
    }
    if generated_volcanic_at(global_config, world_x, world_z) > 246 {
        return if value_noise2(&global_config.world_seed, world_x, world_z, 64, 106) > 180 {
            BLOCK_BASALT
        } else {
            BLOCK_ASH
        };
    }
    if generated_cold_at(global_config, world_x, world_z, surface) {
        return if surface > global_config.sea_level.saturating_add(34)
            || value_noise2(&global_config.world_seed, world_x, world_z, 72, 107) > 164
        {
            BLOCK_SNOW
        } else {
            BLOCK_FROZEN_SOIL
        };
    }
    if desert > 178 {
        if desert > 226 && value_noise2(&global_config.world_seed, world_x, world_z, 88, 108) > 188
        {
            return BLOCK_SALT_FLAT;
        }
        return if desert > 204 {
            BLOCK_SAND
        } else {
            BLOCK_DRY_DIRT
        };
    }
    if moisture > 188 {
        return if moisture > 208 {
            BLOCK_MUD
        } else {
            BLOCK_GRASS
        };
    }
    if surface >= global_config.sea_level.saturating_add(36) {
        return BLOCK_STONE;
    }
    BLOCK_GRASS
}

fn generated_subsurface_block_id(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> u16 {
    match generated_surface_block_id(global_config, world_x, world_z, surface) {
        BLOCK_SAND | BLOCK_SALT_FLAT | BLOCK_QUICKSAND => BLOCK_SAND,
        BLOCK_MUD | BLOCK_CLAY | BLOCK_MOSS => {
            if hash_coord3(
                &global_config.world_seed,
                world_x,
                surface as i32 - 1,
                world_z,
                121,
            ) > 112
            {
                BLOCK_CLAY
            } else {
                BLOCK_MUD
            }
        }
        BLOCK_SNOW | BLOCK_FROZEN_SOIL => BLOCK_FROZEN_SOIL,
        BLOCK_BASALT | BLOCK_ASH => BLOCK_BASALT,
        BLOCK_STONE => BLOCK_STONE,
        _ => BLOCK_DIRT,
    }
}

fn generated_tree_block_id_at(
    global_config: &GlobalConfigView,
    world_x: i32,
    y: i16,
    world_z: i32,
) -> u16 {
    let mut best: Option<(i32, i32, u16)> = None;
    let tree_x_hash_base = seed_salt_hash(&global_config.world_seed, 401);
    let tree_z_hash_base = seed_salt_hash(&global_config.world_seed, 402);
    let tree_roll_hash_base = seed_salt_hash(&global_config.world_seed, 403);
    for cell_size in 6_i32..=14 {
        let min_cell_x = tree_candidate_min_cell(world_x, TREE_MAX_LEAF_RADIUS, cell_size);
        let max_cell_x = tree_candidate_max_cell(world_x, TREE_MAX_LEAF_RADIUS, cell_size);
        let min_cell_z = tree_candidate_min_cell(world_z, TREE_MAX_LEAF_RADIUS, cell_size);
        let max_cell_z = tree_candidate_max_cell(world_z, TREE_MAX_LEAF_RADIUS, cell_size);
        let inner = (cell_size - 2) as u32;
        for cell_z in min_cell_z..=max_cell_z {
            for cell_x in min_cell_x..=max_cell_x {
                let tree_x = cell_x
                    .saturating_mul(cell_size)
                    .saturating_add(1)
                    .saturating_add(
                        (hash_coord3_from_base(tree_x_hash_base, cell_x, 0, cell_z) % inner) as i32,
                    );
                let tree_z = cell_z
                    .saturating_mul(cell_size)
                    .saturating_add(1)
                    .saturating_add(
                        (hash_coord3_from_base(tree_z_hash_base, cell_x, 0, cell_z) % inner) as i32,
                    );
                if tree_x.saturating_sub(world_x).abs() > TREE_MAX_LEAF_RADIUS
                    || tree_z.saturating_sub(world_z).abs() > TREE_MAX_LEAF_RADIUS
                {
                    continue;
                }
                let roll = hash_coord3_from_base(tree_roll_hash_base, cell_x, 0, cell_z) & 255;
                if roll <= 128 {
                    continue;
                }
                let surface = generated_surface_height(global_config, tree_x, tree_z);
                if !tree_vertical_bounds_can_contain(surface, y)
                    || !generated_can_grow_tree(global_config, tree_x, tree_z, surface)
                {
                    continue;
                }
                let growth = generated_tree_growth_profile(global_config, tree_x, tree_z, surface);
                if growth.cell_size != cell_size || roll <= growth.density {
                    continue;
                }
                let tree = generated_tree_from_profile(
                    global_config,
                    tree_x,
                    tree_z,
                    surface,
                    growth.pine,
                );
                let block = generated_tree_volume_block(global_config, &tree, world_x, y, world_z);
                if block != BLOCK_AIR {
                    match best {
                        Some((best_z, best_x, _))
                            if tree.z > best_z || (tree.z == best_z && tree.x >= best_x) => {}
                        _ => best = Some((tree.z, tree.x, block)),
                    }
                }
            }
        }
    }
    best.map(|(_, _, block)| block).unwrap_or(BLOCK_AIR)
}

fn tree_vertical_bounds_can_contain(surface: i16, y: i16) -> bool {
    y >= surface.saturating_add(1)
        && y <= surface.saturating_add(TREE_MAX_BLOCK_HEIGHT_ABOVE_SURFACE)
}

fn tree_candidate_min_cell(world_coord: i32, radius: i32, cell_size: i32) -> i32 {
    div_floor_i32(
        world_coord
            .saturating_sub(radius)
            .saturating_sub(cell_size.saturating_sub(2)),
        cell_size,
    )
}

fn tree_candidate_max_cell(world_coord: i32, radius: i32, cell_size: i32) -> i32 {
    div_floor_i32(
        world_coord.saturating_add(radius).saturating_sub(1),
        cell_size,
    )
}

fn generated_can_grow_tree(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> bool {
    if surface <= global_config.sea_level.saturating_add(1) {
        return false;
    }
    let water_level = generated_water_level(global_config, world_x, world_z, surface);
    if water_level.map(|level| surface < level).unwrap_or(false) {
        return false;
    }
    !generated_desert_at(global_config, world_x, world_z)
        && generated_volcanic_at(global_config, world_x, world_z) <= 236
}

struct GeneratedTree {
    #[allow(dead_code)]
    exists: bool,
    x: i32,
    z: i32,
    base_y: i16,
    trunk_height: i16,
    pine: bool,
}

struct TreeGrowthProfile {
    cell_size: i32,
    density: u32,
    pine: bool,
}

#[allow(dead_code)]
fn generated_tree_at(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> GeneratedTree {
    let growth = generated_tree_growth_profile(global_config, world_x, world_z, surface);
    let density = growth.density;
    let cell_size = growth.cell_size;
    let cell_x = div_floor_i32(world_x, cell_size);
    let cell_z = div_floor_i32(world_z, cell_size);
    let inner = (cell_size - 2).max(1) as u32;
    let tree_x = cell_x
        .saturating_mul(cell_size)
        .saturating_add(1)
        .saturating_add(
            (hash_coord3(&global_config.world_seed, cell_x, 0, cell_z, 401) % inner) as i32,
        );
    let tree_z = cell_z
        .saturating_mul(cell_size)
        .saturating_add(1)
        .saturating_add(
            (hash_coord3(&global_config.world_seed, cell_x, 0, cell_z, 402) % inner) as i32,
        );
    let roll = hash_coord3(&global_config.world_seed, cell_x, 0, cell_z, 403) & 255;
    let tree = generated_tree_from_profile(global_config, world_x, world_z, surface, growth.pine);
    GeneratedTree {
        exists: world_x == tree_x && world_z == tree_z && roll > density,
        ..tree
    }
}

fn generated_tree_from_profile(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
    pine: bool,
) -> GeneratedTree {
    let trunk_height = (if pine { 5 } else { 4 })
        + (hash_coord3(
            &global_config.world_seed,
            world_x,
            surface as i32,
            world_z,
            405,
        ) % 3) as i16;
    GeneratedTree {
        exists: true,
        x: world_x,
        z: world_z,
        base_y: surface.saturating_add(1),
        trunk_height,
        pine,
    }
}

fn generated_tree_growth_profile(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> TreeGrowthProfile {
    let top = generated_surface_block_id(global_config, world_x, world_z, surface);
    if matches!(top, BLOCK_SAND | BLOCK_SALT_FLAT | BLOCK_ASH | BLOCK_BASALT) {
        return TreeGrowthProfile {
            cell_size: 14,
            density: 255,
            pine: false,
        };
    }
    let moisture = generated_moisture_at(global_config, world_x, world_z) as i32;
    let altitude = surface.saturating_sub(global_config.sea_level) as i32;
    let snow_line = generated_snow_line_at(global_config, world_x, world_z);
    let terrain = generated_terrain_factors(global_config, world_x, world_z);
    let (mut cell_size, mut density) = if moisture > 214 && altitude <= 44 {
        (6_i32, 136_i32)
    } else if moisture > 188 && altitude <= 54 {
        (6_i32, 154_i32)
    } else if moisture < 116 {
        (11_i32, 226_i32)
    } else if moisture < 150 {
        (9_i32, 210_i32)
    } else {
        (7_i32, 184_i32)
    };

    if altitude <= 6 {
        cell_size += 2;
        density += 22;
    } else if altitude <= 18 && terrain.floodplain_mask > 360 {
        cell_size += 1;
        density += 10;
    }
    if terrain.floodplain_mask > 620 && terrain.open_river > 520 {
        density += 10;
    }
    if altitude >= 36 {
        cell_size += 1;
        density += 12;
    }
    if altitude >= 54 {
        cell_size += 1;
        density += 14;
    }
    if surface >= snow_line.saturating_sub(10) {
        cell_size += 1;
        density += 12;
    }
    if surface >= snow_line {
        cell_size += 1;
        density += 12;
    }
    if matches!(top, BLOCK_STONE | BLOCK_GRAVEL) {
        cell_size += 1;
        density += 14;
    } else if matches!(top, BLOCK_FROZEN_SOIL | BLOCK_SNOW) {
        cell_size += 1;
        density += 8;
    } else if matches!(top, BLOCK_MUD | BLOCK_CLAY) {
        density -= 8;
    }

    let patch = value_noise2(&global_config.world_seed, world_x, world_z, 260, 406);
    if patch > 204 {
        cell_size -= 1;
        density -= 24;
    } else if patch < 58 {
        cell_size += 1;
        density += 14;
    }
    cell_size = cell_size.clamp(6, 14);
    density = density.clamp(128, 250);
    let pine = surface >= snow_line.saturating_sub(18)
        || altitude >= 46
        || (altitude >= 26 && moisture < 168)
        || (hash_coord3(
            &global_config.world_seed,
            world_x,
            surface as i32,
            world_z,
            404,
        ) & 255)
            > 218;
    TreeGrowthProfile {
        cell_size,
        density: density as u32,
        pine,
    }
}

fn generated_tree_volume_block(
    global_config: &GlobalConfigView,
    tree: &GeneratedTree,
    world_x: i32,
    y: i16,
    world_z: i32,
) -> u16 {
    let top = tree.base_y.saturating_add(tree.trunk_height);
    if world_x == tree.x && world_z == tree.z && y >= tree.base_y && y < top {
        return if tree.pine {
            BLOCK_PINE_TRUNK
        } else {
            BLOCK_TRUNK
        };
    }
    if tree.pine {
        let dy = y as i32 - top as i32;
        let layer = match dy {
            -4 => Some((2, 158, 501)),
            -3 => Some((2, 188, 502)),
            -2 => Some((1, 218, 503)),
            -1 => Some((1, 184, 504)),
            0 => Some((1, 138, 505)),
            _ => None,
        };
        if let Some((radius, density, salt)) = layer {
            if leaf_layer_contains_at_y(
                global_config,
                tree.x,
                tree.z,
                world_x,
                y,
                world_z,
                radius,
                density,
                salt,
            ) {
                return BLOCK_PINE_LEAVES;
            }
        } else if dy == 1 && world_x == tree.x && world_z == tree.z {
            return BLOCK_PINE_LEAVES;
        }
        return BLOCK_AIR;
    }
    let dy = y as i32 - top as i32;
    let layer = match dy {
        -2 => Some((2, 174, 511)),
        -1 => Some((2, 214, 512)),
        0 => Some((2, 148, 513)),
        1 => Some((1, 194, 514)),
        _ => None,
    };
    if let Some((radius, density, salt)) = layer {
        if leaf_layer_contains_at_y(
            global_config,
            tree.x,
            tree.z,
            world_x,
            y,
            world_z,
            radius,
            density,
            salt,
        ) {
            return BLOCK_LEAVES;
        }
    }
    BLOCK_AIR
}

#[allow(clippy::too_many_arguments)]
fn leaf_layer_contains_at_y(
    global_config: &GlobalConfigView,
    center_x: i32,
    center_z: i32,
    world_x: i32,
    y: i16,
    world_z: i32,
    radius: i32,
    density: u32,
    salt: u32,
) -> bool {
    let dx = world_x.saturating_sub(center_x);
    let dz = world_z.saturating_sub(center_z);
    if dx.abs() > radius || dz.abs() > radius || dx.abs().saturating_add(dz.abs()) > radius + 1 {
        return false;
    }
    let roll = hash_coord3(
        &global_config.world_seed,
        center_x.saturating_add(dx.saturating_mul(23)),
        y as i32,
        center_z.saturating_add(dz.saturating_mul(29)),
        salt,
    ) & 255;
    if dx.abs() == radius && dz.abs() == radius && roll < 178 {
        return false;
    }
    roll <= density
}

fn generated_cold_at(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> bool {
    let snow_line = generated_snow_line_at(global_config, world_x, world_z);
    surface >= snow_line
        || (surface >= snow_line.saturating_sub(7)
            && value_noise2(&global_config.world_seed, world_x, world_z, 160, 201) < 28)
}

fn generated_snow_line_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> i16 {
    let offset =
        ((value_noise2(&global_config.world_seed, world_x, world_z, 220, 202) as i32 - 128) * 8)
            / 128;
    (global_config.sea_level as i32 + 58 + offset).clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn generated_desert_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> bool {
    generated_desert_score_at(global_config, world_x, world_z) > 178
}

fn generated_volcanic_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> u32 {
    value_noise2(&global_config.world_seed, world_x, world_z, 192, 205)
}

struct GeneratedTerrainFactors {
    wx: i32,
    wz: i32,
    shelf: u32,
    inland: u32,
    water_mask: u32,
    lake: u32,
    valley_mask: u32,
    floodplain_mask: u32,
    valley_softness: u32,
    open_river: u32,
}

fn generated_terrain_factors(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
) -> GeneratedTerrainFactors {
    let warp_x =
        ((value_noise2(&global_config.world_seed, world_x, world_z, 160, 31) as i32 - 128) * 22)
            / 128;
    let warp_z =
        ((value_noise2(&global_config.world_seed, world_x, world_z, 160, 32) as i32 - 128) * 22)
            / 128;
    let wx = world_x.saturating_add(warp_x);
    let wz = world_z.saturating_add(warp_z);
    let continent = ((value_noise2(&global_config.world_seed, wx, wz, 520, 21) as i32 - 128) * 86)
        / 128
        + ((value_noise2(&global_config.world_seed, wx, wz, 220, 22) as i32 - 128) * 42) / 128
        + ((value_noise2(&global_config.world_seed, wx, wz, 96, 23) as i32 - 128) * 14) / 128
        + 46;
    let shelf = smooth_range_fixed(continent, -50, 34);
    let inland = smooth_range_fixed(continent, -8, 78);
    let river_warp_x =
        ((value_noise2(&global_config.world_seed, wx, wz, 128, 33) as i32 - 128) * 36) / 128;
    let river_warp_z =
        ((value_noise2(&global_config.world_seed, wx, wz, 128, 34) as i32 - 128) * 36) / 128;
    let river_line = 128
        - (value_noise2(
            &global_config.world_seed,
            wx.saturating_add(river_warp_x),
            wz.saturating_add(river_warp_z),
            104,
            35,
        ) as i32
            - 128)
            .abs();
    let lake_noise = value_noise2(&global_config.world_seed, wx, wz, 220, 37) as i32;
    let width_noise = ((value_noise2(&global_config.world_seed, wx, wz, 420, 43) as i32 * 2)
        + value_noise2(&global_config.world_seed, wx, wz, 96, 44) as i32)
        / 3;
    let canyon_noise = value_noise2(&global_config.world_seed, wx, wz, 340, 47) as i32;
    let broad_plain = smooth_range_fixed(
        value_noise2(&global_config.world_seed, wx, wz, 760, 49) as i32,
        144,
        224,
    );
    let river_width = 255.min(width_noise + (broad_plain as i32 * 64) / 1024);
    let lake_width = value_noise2(&global_config.world_seed, wx, wz, 520, 45) as i32;
    let open_river = scale_by_fixed(
        smooth_range_fixed(river_width, 72, 198) as i32,
        1024 - smooth_range_fixed(canyon_noise, 190, 252),
    ) as u32;
    let river_valley_start = 104 - (river_width * 88) / 255;
    let river_terrace_start = 0.max(river_valley_start - 64 - (river_width * 42) / 255);
    let river_floodplain_start = 0.max(river_terrace_start - 44 - (river_width * 32) / 255);
    let river_core_start = 122 - (river_width * 44) / 255;
    let river_terrace = scale_by_fixed(
        smooth_range_fixed(river_line, river_terrace_start, 128) as i32,
        (220 + (river_width * 430) / 255) as u32,
    );
    let river_floodplain = scale_by_fixed(
        smooth_range_fixed(river_line, river_floodplain_start, 128) as i32,
        open_river,
    );
    let river = scale_by_fixed(
        smooth_range_fixed(river_line, river_core_start, 128) as i32,
        inland,
    );
    let river_valley = scale_by_fixed(
        smooth_range_fixed(river_line, river_valley_start, 128).max(river_terrace as u32) as i32,
        inland,
    );
    let lake_core_start = 226 - (lake_width * 28) / 255;
    let lake = scale_by_fixed(
        smooth_range_fixed(lake_noise, lake_core_start, 242) as i32,
        inland,
    );
    let lake_valley_start = 194 - (lake_width * 74) / 255;
    let lake_terrace = scale_by_fixed(
        smooth_range_fixed(lake_noise, 0.max(lake_valley_start - 42), 242) as i32,
        (180 + (lake_width * 260) / 255) as u32,
    );
    let lake_valley = scale_by_fixed(
        smooth_range_fixed(lake_noise, lake_valley_start, 242).max(lake_terrace as u32) as i32,
        inland,
    );
    let floodplain_mask = scale_by_fixed(river_floodplain.max(lake_terrace) as i32, inland);
    GeneratedTerrainFactors {
        wx,
        wz,
        shelf,
        inland,
        water_mask: river.max(lake) as u32,
        lake: lake as u32,
        valley_mask: river_valley.max(lake_valley) as u32,
        floodplain_mask: floodplain_mask as u32,
        valley_softness: river_width.max(lake_width) as u32,
        open_river,
    }
}

fn generated_water_level(
    global_config: &GlobalConfigView,
    _world_x: i32,
    _world_z: i32,
    surface: i16,
) -> Option<i16> {
    if surface < global_config.sea_level {
        return Some(global_config.sea_level);
    }
    None
}

fn generated_moisture_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> u32 {
    (value_noise2(&global_config.world_seed, world_x, world_z, 176, 211) * 3
        + value_noise2(&global_config.world_seed, world_x, world_z, 72, 212))
        / 4
}

fn generated_desert_score_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> u32 {
    (value_noise2(&global_config.world_seed, world_x, world_z, 224, 213) * 3
        + (255 - generated_moisture_at(global_config, world_x, world_z)))
        / 4
}

fn value_noise2(seed: &[u8; 32], x: i32, z: i32, scale: i32, salt: u32) -> u32 {
    let cell_x = div_floor_i32(x, scale);
    let cell_z = div_floor_i32(z, scale);
    let local_x = positive_mod_i32(x, scale);
    let local_z = positive_mod_i32(z, scale);
    let tx = smooth_fixed(local_x, scale);
    let tz = smooth_fixed(local_z, scale);
    let base = seed_salt_hash(seed, salt);
    let a = hash_coord3_from_base(base, cell_x, 0, cell_z) & 255;
    let b = hash_coord3_from_base(base, cell_x.saturating_add(1), 0, cell_z) & 255;
    let c = hash_coord3_from_base(base, cell_x, 0, cell_z.saturating_add(1)) & 255;
    let d =
        hash_coord3_from_base(base, cell_x.saturating_add(1), 0, cell_z.saturating_add(1)) & 255;
    lerp_fixed(lerp_fixed(a, b, tx), lerp_fixed(c, d, tx), tz)
}

fn hash_coord3(seed: &[u8; 32], x: i32, y: i32, z: i32, salt: u32) -> u32 {
    hash_coord3_from_base(seed_salt_hash(seed, salt), x, y, z)
}

fn seed_salt_hash(seed: &[u8; 32], salt: u32) -> u32 {
    let mut hash = 0x811c_9dc5_u32 ^ salt;
    for byte in seed {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn hash_coord3_from_base(base: u32, x: i32, y: i32, z: i32) -> u32 {
    let mut hash = base;
    for byte in x
        .to_le_bytes()
        .iter()
        .chain(y.to_le_bytes().iter())
        .chain(z.to_le_bytes().iter())
    {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    hash ^ (hash >> 16)
}

fn div_floor_i32(value: i32, divisor: i32) -> i32 {
    let quotient = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && ((remainder < 0) != (divisor < 0)) {
        quotient - 1
    } else {
        quotient
    }
}

fn positive_mod_i32(value: i32, divisor: i32) -> i32 {
    let remainder = value % divisor;
    if remainder < 0 {
        remainder + divisor.abs()
    } else {
        remainder
    }
}

fn smooth_fixed(distance: i32, scale: i32) -> u32 {
    let fixed = (distance as i64 * 1024) / scale as i64;
    ((fixed * fixed * (3072 - fixed * 2)) / (1024 * 1024)) as u32
}

fn smooth_range_fixed(value: i32, edge0: i32, edge1: i32) -> u32 {
    if value <= edge0 {
        return 0;
    }
    if value >= edge1 {
        return 1024;
    }
    smooth_fixed(value - edge0, edge1 - edge0)
}

fn lerp_fixed(a: u32, b: u32, t: u32) -> u32 {
    ((a as u64 * (1024 - t) as u64 + b as u64 * t as u64 + 512) / 1024) as u32
}

fn lerp_i32_fixed(a: i32, b: i32, t: u32) -> i32 {
    ((a as i64 * (1024 - t) as i64 + b as i64 * t as i64 + 512) / 1024) as i32
}

fn scale_by_fixed(value: i32, fixed: u32) -> i32 {
    ((value as i64 * fixed as i64) / 1024) as i32
}

pub struct PlayerProfileView;

impl PlayerProfileView {
    pub fn validate(data: &[u8], authority: &Pubkey, global_config: &Pubkey) -> ProgramResult {
        if data.len() != PLAYER_PROFILE_LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
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
        if read_i64(data, PLAYER_SESSION_EXPIRES_AT_OFFSET) <= now {
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

pub struct PlayerProgressInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct PlayerProgressState {
    pub precision_gathering_xp: u64,
    pub smelting_xp: u64,
    pub exploration_xp: u64,
    pub explored_chunk_count: u32,
}

impl PlayerProgressState {
    pub fn pack_empty(dst: &mut [u8], args: &PlayerProgressInitArgs) -> ProgramResult {
        if dst.len() != PLAYER_PROGRESS_LEN {
            return Err(NicechunkChunkError::InvalidPlayerProgressData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&PLAYER_PROGRESS_MAGIC);
        dst[8..10].copy_from_slice(&PLAYER_PROGRESS_VERSION.to_le_bytes());
        dst[10] = args.bump;
        dst[11] = 1;
        dst[PLAYER_PROGRESS_OWNER_OFFSET..PLAYER_PROGRESS_OWNER_OFFSET + 32]
            .copy_from_slice(args.owner.as_ref());
        dst[PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET..PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(args.global_config.as_ref());
        dst[PLAYER_PROGRESS_PRECISION_XP_OFFSET..PLAYER_PROGRESS_PRECISION_XP_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        dst[PLAYER_PROGRESS_SMELTING_XP_OFFSET..PLAYER_PROGRESS_SMELTING_XP_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        dst[PLAYER_PROGRESS_EXPLORATION_XP_OFFSET..PLAYER_PROGRESS_EXPLORATION_XP_OFFSET + 8]
            .copy_from_slice(&0_u64.to_le_bytes());
        dst[PLAYER_PROGRESS_EXPLORED_CHUNK_COUNT_OFFSET
            ..PLAYER_PROGRESS_EXPLORED_CHUNK_COUNT_OFFSET + 4]
            .copy_from_slice(&0_u32.to_le_bytes());
        dst[PLAYER_PROGRESS_CREATED_SLOT_OFFSET..PLAYER_PROGRESS_CREATED_SLOT_OFFSET + 8]
            .copy_from_slice(&args.created_slot.to_le_bytes());
        dst[PLAYER_PROGRESS_UPDATED_SLOT_OFFSET..PLAYER_PROGRESS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&args.created_slot.to_le_bytes());
        dst[PLAYER_PROGRESS_CREATED_AT_OFFSET..PLAYER_PROGRESS_CREATED_AT_OFFSET + 8]
            .copy_from_slice(&args.created_at.to_le_bytes());
        Ok(())
    }

    pub fn validate(
        data: &[u8],
        owner: &Pubkey,
        global_config: &Pubkey,
    ) -> Result<Self, NicechunkChunkError> {
        if data.len() != PLAYER_PROGRESS_LEN
            || data[0..8] != PLAYER_PROGRESS_MAGIC
            || read_u16(data, 8) != PLAYER_PROGRESS_VERSION
            || data[11] != 1
        {
            return Err(NicechunkChunkError::InvalidPlayerProgressData);
        }
        if &data[PLAYER_PROGRESS_OWNER_OFFSET..PLAYER_PROGRESS_OWNER_OFFSET + 32] != owner.as_ref()
        {
            return Err(NicechunkChunkError::InvalidPlayerProgress);
        }
        if &data[PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET..PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkChunkError::InvalidPlayerProgress);
        }
        Ok(Self {
            precision_gathering_xp: read_u64(data, PLAYER_PROGRESS_PRECISION_XP_OFFSET),
            smelting_xp: read_u64(data, PLAYER_PROGRESS_SMELTING_XP_OFFSET),
            exploration_xp: read_u64(data, PLAYER_PROGRESS_EXPLORATION_XP_OFFSET),
            explored_chunk_count: read_u32(data, PLAYER_PROGRESS_EXPLORED_CHUNK_COUNT_OFFSET),
        })
    }

    pub fn precision_gathering_level_from_xp(xp: u64) -> u8 {
        let mut level = 0_u8;
        for (index, required_total) in PRECISION_GATHERING_TOTAL_XP_BY_LEVEL.iter().enumerate() {
            if xp >= *required_total {
                level = index as u8;
            }
        }
        level.min(10)
    }

    pub fn precision_gathering_volume_mm3_from_xp(xp: u64) -> u32 {
        let level = Self::precision_gathering_level_from_xp(xp) as u32;
        RESOURCE_BLOCK_VOLUME_MM3.saturating_mul(10 + level.saturating_mul(10)) / 100
    }

    pub fn exploration_level_from_xp(xp: u64) -> u8 {
        let mut level = 0_u8;
        for (index, required_total) in EXPLORATION_TOTAL_XP_BY_LEVEL.iter().enumerate() {
            if xp >= *required_total {
                level = index as u8;
            }
        }
        level.min(10)
    }

    pub fn exploration_chance_bps(chance_bps: u16, exploration_xp: u64) -> u16 {
        let level = Self::exploration_level_from_xp(exploration_xp) as u32;
        let weighted = (chance_bps as u32)
            .saturating_mul(100_u32.saturating_add(level.saturating_mul(10)))
            / 100;
        weighted.min(RESOURCE_DROP_CHANCE_DENOMINATOR) as u16
    }

    pub fn add_precision_gathering_xp(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        gained_xp: u64,
        updated_slot: u64,
    ) -> ProgramResult {
        let state = Self::validate(data, owner, global_config)?;
        let next_xp = state.precision_gathering_xp.saturating_add(gained_xp);
        data[PLAYER_PROGRESS_PRECISION_XP_OFFSET..PLAYER_PROGRESS_PRECISION_XP_OFFSET + 8]
            .copy_from_slice(&next_xp.to_le_bytes());
        data[PLAYER_PROGRESS_UPDATED_SLOT_OFFSET..PLAYER_PROGRESS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn add_exploration_xp(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        gained_xp: u64,
        updated_slot: u64,
    ) -> ProgramResult {
        let state = Self::validate(data, owner, global_config)?;
        let next_xp = state.exploration_xp.saturating_add(gained_xp);
        data[PLAYER_PROGRESS_EXPLORATION_XP_OFFSET..PLAYER_PROGRESS_EXPLORATION_XP_OFFSET + 8]
            .copy_from_slice(&next_xp.to_le_bytes());
        data[PLAYER_PROGRESS_UPDATED_SLOT_OFFSET..PLAYER_PROGRESS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn add_explored_chunk_count(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        gained_chunks: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        if gained_chunks == 0 {
            return Ok(());
        }
        let state = Self::validate(data, owner, global_config)?;
        let next_count = state.explored_chunk_count.saturating_add(gained_chunks);
        data[PLAYER_PROGRESS_EXPLORED_CHUNK_COUNT_OFFSET
            ..PLAYER_PROGRESS_EXPLORED_CHUNK_COUNT_OFFSET + 4]
            .copy_from_slice(&next_count.to_le_bytes());
        data[PLAYER_PROGRESS_UPDATED_SLOT_OFFSET..PLAYER_PROGRESS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
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
        Ok((
            self.world_x.div_euclid(chunk_size),
            self.world_z.div_euclid(chunk_size),
            self.world_x.rem_euclid(chunk_size) as u8,
            self.world_z.rem_euclid(chunk_size) as u8,
        ))
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
        })
    }
}

pub struct BatchMineArgs {
    pub mode: u8,
    pub blocks: Vec<MineBlockArgs>,
}

pub struct RangeMineArgs {
    pub mode: u8,
    pub min_x: i32,
    pub min_z: i32,
    pub size_x: u8,
    pub size_z: u8,
    pub blocks: Vec<MineBlockArgs>,
}

impl RangeMineArgs {
    const HEADER_LEN: usize = 15;
    const BLOCK_ID_BITS: usize = 6;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() < Self::HEADER_LEN {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }
        let mode = data[0];
        if mode != RANGE_MINE_MODE_DEBUG || cfg!(feature = "mainnet") {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }

        let min_x = read_i32(data, 1);
        let min_y = read_i16(data, 5);
        let min_z = read_i32(data, 7);
        let size_x = data[11] as usize;
        let size_y = read_u16(data, 12) as usize;
        let size_z = data[14] as usize;
        let volume = size_x
            .checked_mul(size_y)
            .and_then(|value| value.checked_mul(size_z))
            .ok_or(NicechunkChunkError::InvalidRangeMine)?;
        if size_x == 0
            || size_x > 16
            || size_y == 0
            || size_z == 0
            || size_z > 16
            || volume > RANGE_MINE_MAX_BLOCKS
        {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }

        let bitmap_len = volume.div_ceil(8);
        if data.len() < Self::HEADER_LEN + bitmap_len {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }
        let bitmap = &data[Self::HEADER_LEN..Self::HEADER_LEN + bitmap_len];
        if !unused_high_bits_are_zero(bitmap, volume) {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }
        let selected_count = bitmap
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum::<usize>();
        if selected_count == 0 || selected_count > RANGE_MINE_MAX_BLOCKS {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }

        let block_id_len = (selected_count * Self::BLOCK_ID_BITS).div_ceil(8);
        let expected_len = Self::HEADER_LEN + bitmap_len + block_id_len;
        if data.len() != expected_len {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }
        let block_ids = &data[Self::HEADER_LEN + bitmap_len..];
        if !unused_high_bits_are_zero(block_ids, selected_count * Self::BLOCK_ID_BITS) {
            return Err(NicechunkChunkError::InvalidRangeMine);
        }

        let layer_area = size_x * size_z;
        let mut blocks = Vec::with_capacity(selected_count);
        let mut selected_index = 0;
        for volume_index in 0..volume {
            if !packed_bit(bitmap, volume_index) {
                continue;
            }
            let block_id = packed_six_bit_value(block_ids, selected_index);
            if matches!(u16::from(block_id), BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK) {
                return Err(NicechunkChunkError::UnmineableBlock);
            }
            let y_offset = volume_index / layer_area;
            let within_layer = volume_index % layer_area;
            let z_offset = within_layer / size_x;
            let x_offset = within_layer % size_x;
            let world_x = min_x
                .checked_add(x_offset as i32)
                .ok_or(NicechunkChunkError::InvalidRangeMine)?;
            let world_y = i32::from(min_y)
                .checked_add(y_offset as i32)
                .and_then(|value| i16::try_from(value).ok())
                .ok_or(NicechunkChunkError::InvalidRangeMine)?;
            let world_z = min_z
                .checked_add(z_offset as i32)
                .ok_or(NicechunkChunkError::InvalidRangeMine)?;
            blocks.push(MineBlockArgs {
                world_x,
                world_y,
                world_z,
                expected_block_id: u16::from(block_id),
            });
            selected_index += 1;
        }
        Ok(Self {
            mode,
            min_x,
            min_z,
            size_x: size_x as u8,
            size_z: size_z as u8,
            blocks,
        })
    }
}

fn packed_bit(data: &[u8], bit_index: usize) -> bool {
    data.get(bit_index / 8)
        .map(|byte| byte & (1 << (bit_index % 8)) != 0)
        .unwrap_or(false)
}

fn packed_six_bit_value(data: &[u8], value_index: usize) -> u8 {
    let bit_index = value_index * RangeMineArgs::BLOCK_ID_BITS;
    let byte_index = bit_index / 8;
    let shift = bit_index % 8;
    let low = u16::from(data.get(byte_index).copied().unwrap_or_default());
    let high = u16::from(data.get(byte_index + 1).copied().unwrap_or_default());
    (((low | (high << 8)) >> shift) & 0x3f) as u8
}

fn unused_high_bits_are_zero(data: &[u8], used_bits: usize) -> bool {
    let remainder = used_bits % 8;
    remainder == 0
        || data
            .last()
            .map(|byte| byte & !((1_u8 << remainder) - 1) == 0)
            .unwrap_or(false)
}

impl BatchMineArgs {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() < 2 {
            return Err(NicechunkChunkError::InvalidBatchMine);
        }
        let mode = data[0];
        let count = data[1] as usize;
        if mode != BATCH_MINE_MODE_DEBUG
            || count == 0
            || count > BATCH_MINE_MAX_BLOCKS
            || data.len() != 2 + count * MineBlockArgs::LEN
        {
            return Err(NicechunkChunkError::InvalidBatchMine);
        }
        let mut blocks = Vec::with_capacity(count);
        for index in 0..count {
            let offset = 2 + index * MineBlockArgs::LEN;
            blocks.push(MineBlockArgs::unpack(
                &data[offset..offset + MineBlockArgs::LEN],
            )?);
        }
        Ok(Self { mode, blocks })
    }
}

pub fn batch_mine_reward_passes(
    global_config: &GlobalConfigView,
    block: &MineBlockArgs,
    salt: u32,
    chance_bps: u16,
) -> bool {
    if chance_bps == 0 {
        return false;
    }
    let chance = u32::from(chance_bps).min(RESOURCE_DROP_CHANCE_DENOMINATOR);
    hash_coord3(
        &global_config.world_seed,
        block.world_x,
        i32::from(block.world_y),
        block.world_z,
        1_600_u32.saturating_add(salt),
    ) % RESOURCE_DROP_CHANCE_DENOMINATOR
        < chance
}

pub fn range_mine_reward_passes(
    global_config: &GlobalConfigView,
    block: &MineBlockArgs,
    salt: u32,
    chance_bps: u16,
) -> bool {
    if chance_bps == 0 {
        return false;
    }
    let chance = u32::from(chance_bps).min(RESOURCE_DROP_CHANCE_DENOMINATOR);
    let mut hash = read_u32(&global_config.world_seed, 0)
        ^ read_u32(&global_config.world_seed, 12).rotate_left(11)
        ^ read_u32(&global_config.world_seed, 24).rotate_left(23)
        ^ salt.wrapping_mul(0x9e37_79b9)
        ^ (block.world_x as u32).wrapping_mul(0x85eb_ca6b)
        ^ (block.world_y as u32).wrapping_mul(0xc2b2_ae35)
        ^ (block.world_z as u32).wrapping_mul(0x27d4_eb2f);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    (hash ^ (hash >> 16)) % RESOURCE_DROP_CHANCE_DENOMINATOR < chance
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundationRecord {
    pub owner: Pubkey,
    pub foundation_id: u64,
    pub min_x: i32,
    pub min_z: i32,
    pub surface_y: i16,
    pub width: u8,
    pub depth: u8,
}

impl FoundationRecord {
    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != FOUNDATION_CHUNK_RECORD_LEN {
            return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
        }
        dst.fill(0);
        dst[0..32].copy_from_slice(self.owner.as_ref());
        dst[32..40].copy_from_slice(&self.foundation_id.to_le_bytes());
        dst[40..44].copy_from_slice(&self.min_x.to_le_bytes());
        dst[44..48].copy_from_slice(&self.min_z.to_le_bytes());
        dst[48..50].copy_from_slice(&self.surface_y.to_le_bytes());
        dst[50] = self.width;
        dst[51] = self.depth;
        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, NicechunkChunkError> {
        if src.len() != FOUNDATION_CHUNK_RECORD_LEN {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        let record = Self {
            owner: Pubkey::new_from_array(
                src[0..32]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationChunkData)?,
            ),
            foundation_id: read_u64(src, 32),
            min_x: read_i32(src, 40),
            min_z: read_i32(src, 44),
            surface_y: read_i16(src, 48),
            width: src[50],
            depth: src[51],
        };
        if !(FOUNDATION_MIN_SIZE..=FOUNDATION_MAX_SIZE).contains(&record.width)
            || !(FOUNDATION_MIN_SIZE..=FOUNDATION_MAX_SIZE).contains(&record.depth)
        {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        Ok(record)
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.min_x <= other.max_x()
            && self.max_x() >= other.min_x
            && self.min_z <= other.max_z()
            && self.max_z() >= other.min_z
    }

    pub fn protects(&self, world_x: i32, world_y: i16, world_z: i32) -> bool {
        world_y == self.surface_y.saturating_sub(1)
            && world_x >= self.min_x
            && world_x <= self.max_x()
            && world_z >= self.min_z
            && world_z <= self.max_z()
    }

    fn max_x(&self) -> i32 {
        self.min_x.saturating_add(self.width as i32 - 1)
    }

    fn max_z(&self) -> i32 {
        self.min_z.saturating_add(self.depth as i32 - 1)
    }
}

pub struct FoundationChunkState;

impl FoundationChunkState {
    pub fn pack_empty(
        dst: &mut [u8],
        bump: u8,
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
    ) -> ProgramResult {
        if dst.len() != FOUNDATION_CHUNK_LEN {
            return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&FOUNDATION_CHUNK_MAGIC);
        dst[8] = FOUNDATION_CHUNK_VERSION;
        dst[9] = bump;
        dst[10..12].copy_from_slice(&0_u16.to_le_bytes());
        dst[12..44].copy_from_slice(global_config.as_ref());
        dst[44..48].copy_from_slice(&chunk_x.to_le_bytes());
        dst[48..52].copy_from_slice(&chunk_z.to_le_bytes());
        Ok(())
    }

    pub fn validate(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<u16, NicechunkChunkError> {
        if data.len() != FOUNDATION_CHUNK_LEN
            || data[0..8] != FOUNDATION_CHUNK_MAGIC
            || data[8] != FOUNDATION_CHUNK_VERSION
            || &data[12..44] != global_config.as_ref()
            || read_i32(data, 44) != chunk_x
            || read_i32(data, 48) != chunk_z
        {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        let count = read_u16(data, 10);
        if count > FOUNDATION_CHUNK_CAPACITY {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        Ok(count)
    }

    pub fn append(
        data: &mut [u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        record: &FoundationRecord,
    ) -> ProgramResult {
        let count = Self::validate(data, global_config, chunk_x, chunk_z)?;
        if count >= FOUNDATION_CHUNK_CAPACITY {
            return Err(NicechunkChunkError::FoundationChunkCapacityExceeded.into());
        }
        let offset = FOUNDATION_CHUNK_HEADER_LEN + count as usize * FOUNDATION_CHUNK_RECORD_LEN;
        record.pack(&mut data[offset..offset + FOUNDATION_CHUNK_RECORD_LEN])?;
        data[10..12].copy_from_slice(&count.saturating_add(1).to_le_bytes());
        Ok(())
    }

    pub fn overlaps(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        candidate: &FoundationRecord,
    ) -> Result<bool, NicechunkChunkError> {
        let count = Self::validate(data, global_config, chunk_x, chunk_z)?;
        for index in 0..count as usize {
            if Self::record_at(data, index)?.overlaps(candidate) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn protects(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        world_x: i32,
        world_y: i16,
        world_z: i32,
    ) -> Result<bool, NicechunkChunkError> {
        if data.len() >= 8 && data[0..8] == FOUNDATION_CHUNK_V2_MAGIC {
            return FoundationChunkV2State::protects(
                data,
                global_config,
                chunk_x,
                chunk_z,
                world_x,
                world_y,
                world_z,
            );
        }
        let count = Self::validate(data, global_config, chunk_x, chunk_z)?;
        for index in 0..count as usize {
            if Self::record_at(data, index)?.protects(world_x, world_y, world_z) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn protects_any(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &[MineBlockArgs],
    ) -> Result<bool, NicechunkChunkError> {
        if data.len() >= 8 && data[0..8] == FOUNDATION_CHUNK_V2_MAGIC {
            return FoundationChunkV2State::protects_any(
                data,
                global_config,
                chunk_x,
                chunk_z,
                blocks,
            );
        }
        let count = Self::validate(data, global_config, chunk_x, chunk_z)?;
        for index in 0..count as usize {
            let record = Self::record_at(data, index)?;
            if blocks
                .iter()
                .any(|block| record.protects(block.world_x, block.world_y, block.world_z))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn record_at(data: &[u8], index: usize) -> Result<FoundationRecord, NicechunkChunkError> {
        let offset = FOUNDATION_CHUNK_HEADER_LEN + index * FOUNDATION_CHUNK_RECORD_LEN;
        FoundationRecord::unpack(&data[offset..offset + FOUNDATION_CHUNK_RECORD_LEN])
    }

    pub fn records(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Vec<FoundationRecord>, NicechunkChunkError> {
        let count = Self::validate(data, global_config, chunk_x, chunk_z)?;
        (0..count as usize)
            .map(|index| Self::record_at(data, index))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundationRecordV2 {
    pub owner: Pubkey,
    pub foundation_id: u64,
    pub min_x: i32,
    pub min_z: i32,
    pub surface_y: i16,
    pub width: u32,
    pub depth: u32,
}

impl FoundationRecordV2 {
    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != FOUNDATION_CHUNK_V2_RECORD_LEN
            || self.foundation_id == 0
            || self.width < 2
            || self.depth < 2
            || self.max_x().is_none()
            || self.max_z().is_none()
        {
            return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
        }
        dst.fill(0);
        dst[0..32].copy_from_slice(self.owner.as_ref());
        dst[32..40].copy_from_slice(&self.foundation_id.to_le_bytes());
        dst[40..44].copy_from_slice(&self.min_x.to_le_bytes());
        dst[44..48].copy_from_slice(&self.min_z.to_le_bytes());
        dst[48..50].copy_from_slice(&self.surface_y.to_le_bytes());
        dst[50..54].copy_from_slice(&self.width.to_le_bytes());
        dst[54..58].copy_from_slice(&self.depth.to_le_bytes());
        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, NicechunkChunkError> {
        if src.len() != FOUNDATION_CHUNK_V2_RECORD_LEN {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        let record = Self {
            owner: Pubkey::new_from_array(
                src[0..32]
                    .try_into()
                    .map_err(|_| NicechunkChunkError::InvalidFoundationChunkData)?,
            ),
            foundation_id: read_u64(src, 32),
            min_x: read_i32(src, 40),
            min_z: read_i32(src, 44),
            surface_y: read_i16(src, 48),
            width: read_u32(src, 50),
            depth: read_u32(src, 54),
        };
        if record.foundation_id == 0
            || record.width < 2
            || record.depth < 2
            || record.max_x().is_none()
            || record.max_z().is_none()
        {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        Ok(record)
    }

    pub fn from_legacy(record: &FoundationRecord) -> Self {
        Self {
            owner: record.owner,
            foundation_id: record.foundation_id,
            min_x: record.min_x,
            min_z: record.min_z,
            surface_y: record.surface_y,
            width: u32::from(record.width),
            depth: u32::from(record.depth),
        }
    }

    pub fn max_x(&self) -> Option<i32> {
        checked_u32_axis_end(self.min_x, self.width)
    }

    pub fn max_z(&self) -> Option<i32> {
        checked_u32_axis_end(self.min_z, self.depth)
    }

    pub fn protects(&self, world_x: i32, world_y: i16, world_z: i32) -> bool {
        world_y == self.surface_y.saturating_sub(1)
            && world_x >= self.min_x
            && self.max_x().map(|max| world_x <= max).unwrap_or(false)
            && world_z >= self.min_z
            && self.max_z().map(|max| world_z <= max).unwrap_or(false)
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.min_x <= other.max_x().unwrap_or(i32::MIN)
            && self.max_x().unwrap_or(i32::MAX) >= other.min_x
            && self.min_z <= other.max_z().unwrap_or(i32::MIN)
            && self.max_z().unwrap_or(i32::MAX) >= other.min_z
    }

    pub fn supersedes_legacy_index(&self, legacy: &Self, chunk_size: i32) -> bool {
        if chunk_size <= 0
            || self.owner != legacy.owner
            || self.foundation_id == legacy.foundation_id
            || self.surface_y != legacy.surface_y
            || !self.overlaps(legacy)
        {
            return false;
        }
        let (Some(self_max_x), Some(self_max_z), Some(legacy_max_x), Some(legacy_max_z)) =
            (self.max_x(), self.max_z(), legacy.max_x(), legacy.max_z())
        else {
            return false;
        };
        self.min_x.div_euclid(chunk_size) == legacy.min_x.div_euclid(chunk_size)
            && self_max_x.div_euclid(chunk_size) == legacy_max_x.div_euclid(chunk_size)
            && self.min_z.div_euclid(chunk_size) == legacy.min_z.div_euclid(chunk_size)
            && self_max_z.div_euclid(chunk_size) == legacy_max_z.div_euclid(chunk_size)
    }
}

pub struct FoundationChunkV2State;

impl FoundationChunkV2State {
    pub fn len(capacity: u16) -> Result<usize, NicechunkChunkError> {
        if capacity == 0 || capacity > FOUNDATION_CHUNK_V2_MAX_CAPACITY {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        Ok(FOUNDATION_CHUNK_V2_HEADER_LEN + capacity as usize * FOUNDATION_CHUNK_V2_RECORD_LEN)
    }

    pub fn pack_empty(
        dst: &mut [u8],
        bump: u8,
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        capacity: u16,
    ) -> ProgramResult {
        if dst.len() != Self::len(capacity)? {
            return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&FOUNDATION_CHUNK_V2_MAGIC);
        dst[8] = FOUNDATION_CHUNK_V2_VERSION;
        dst[9] = bump;
        dst[12..14].copy_from_slice(&capacity.to_le_bytes());
        dst[16..48].copy_from_slice(global_config.as_ref());
        dst[48..52].copy_from_slice(&chunk_x.to_le_bytes());
        dst[52..56].copy_from_slice(&chunk_z.to_le_bytes());
        Ok(())
    }

    pub fn validate(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<(u16, u16), NicechunkChunkError> {
        if data.len() < FOUNDATION_CHUNK_V2_HEADER_LEN
            || data[0..8] != FOUNDATION_CHUNK_V2_MAGIC
            || data[8] != FOUNDATION_CHUNK_V2_VERSION
            || &data[16..48] != global_config.as_ref()
            || read_i32(data, 48) != chunk_x
            || read_i32(data, 52) != chunk_z
        {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        let count = read_u16(data, 10);
        let capacity = read_u16(data, 12);
        if count > capacity || data.len() != Self::len(capacity)? {
            return Err(NicechunkChunkError::InvalidFoundationChunkData);
        }
        Ok((count, capacity))
    }

    pub fn append(
        data: &mut [u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        record: &FoundationRecordV2,
    ) -> ProgramResult {
        let (count, capacity) = Self::validate(data, global_config, chunk_x, chunk_z)?;
        let mut existing_index = None;
        for index in 0..count as usize {
            let current = Self::record_at(data, index)?;
            if current.foundation_id == record.foundation_id {
                if current.owner != record.owner {
                    return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
                }
                existing_index = Some(index);
                continue;
            }
            if current.overlaps(record) {
                return Err(NicechunkChunkError::FoundationOverlap.into());
            }
        }
        if let Some(index) = existing_index {
            let offset = FOUNDATION_CHUNK_V2_HEADER_LEN + index * FOUNDATION_CHUNK_V2_RECORD_LEN;
            return record.pack(&mut data[offset..offset + FOUNDATION_CHUNK_V2_RECORD_LEN]);
        }
        if count >= capacity {
            return Err(NicechunkChunkError::FoundationChunkCapacityExceeded.into());
        }
        let offset =
            FOUNDATION_CHUNK_V2_HEADER_LEN + count as usize * FOUNDATION_CHUNK_V2_RECORD_LEN;
        record.pack(&mut data[offset..offset + FOUNDATION_CHUNK_V2_RECORD_LEN])?;
        data[10..12].copy_from_slice(&count.saturating_add(1).to_le_bytes());
        Ok(())
    }

    pub fn contains_foundation(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        owner: &Pubkey,
        foundation_id: u64,
    ) -> Result<bool, NicechunkChunkError> {
        let (count, _) = Self::validate(data, global_config, chunk_x, chunk_z)?;
        for index in 0..count as usize {
            let current = Self::record_at(data, index)?;
            if current.foundation_id == foundation_id {
                return if current.owner == *owner {
                    Ok(true)
                } else {
                    Err(NicechunkChunkError::InvalidFoundationChunkData)
                };
            }
        }
        Ok(false)
    }

    pub fn remove(
        data: &mut [u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        owner: &Pubkey,
        foundation_id: u64,
    ) -> ProgramResult {
        let (count, _) = Self::validate(data, global_config, chunk_x, chunk_z)?;
        let mut found = None;
        for index in 0..count as usize {
            let current = Self::record_at(data, index)?;
            if current.foundation_id != foundation_id {
                continue;
            }
            if current.owner != *owner {
                return Err(NicechunkChunkError::InvalidFoundationChunkData.into());
            }
            found = Some(index);
            break;
        }
        let Some(index) = found else {
            return Ok(());
        };
        let offset = FOUNDATION_CHUNK_V2_HEADER_LEN + index * FOUNDATION_CHUNK_V2_RECORD_LEN;
        let used_end =
            FOUNDATION_CHUNK_V2_HEADER_LEN + count as usize * FOUNDATION_CHUNK_V2_RECORD_LEN;
        if index + 1 < count as usize {
            data.copy_within(offset + FOUNDATION_CHUNK_V2_RECORD_LEN..used_end, offset);
        }
        data[used_end - FOUNDATION_CHUNK_V2_RECORD_LEN..used_end].fill(0);
        data[10..12].copy_from_slice(&count.saturating_sub(1).to_le_bytes());
        Ok(())
    }

    pub fn protects(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        world_x: i32,
        world_y: i16,
        world_z: i32,
    ) -> Result<bool, NicechunkChunkError> {
        let (count, _) = Self::validate(data, global_config, chunk_x, chunk_z)?;
        for index in 0..count as usize {
            if Self::record_at(data, index)?.protects(world_x, world_y, world_z) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn protects_any(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &[MineBlockArgs],
    ) -> Result<bool, NicechunkChunkError> {
        let (count, _) = Self::validate(data, global_config, chunk_x, chunk_z)?;
        for index in 0..count as usize {
            let record = Self::record_at(data, index)?;
            if blocks
                .iter()
                .any(|block| record.protects(block.world_x, block.world_y, block.world_z))
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn records(
        data: &[u8],
        global_config: &Pubkey,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Vec<FoundationRecordV2>, NicechunkChunkError> {
        let (count, _) = Self::validate(data, global_config, chunk_x, chunk_z)?;
        (0..count as usize)
            .map(|index| Self::record_at(data, index))
            .collect()
    }

    fn record_at(data: &[u8], index: usize) -> Result<FoundationRecordV2, NicechunkChunkError> {
        let offset = FOUNDATION_CHUNK_V2_HEADER_LEN + index * FOUNDATION_CHUNK_V2_RECORD_LEN;
        FoundationRecordV2::unpack(&data[offset..offset + FOUNDATION_CHUNK_V2_RECORD_LEN])
    }
}

fn checked_u32_axis_end(start: i32, length: u32) -> Option<i32> {
    let delta = i64::from(length).checked_sub(1)?;
    let end = i64::from(start).checked_add(delta)?;
    i32::try_from(end).ok()
}

pub struct ChunkBrokenInitArgs {
    pub bump: u8,
    pub min_y: i16,
    pub capacity: u16,
}

pub struct ChunkBrokenState;

impl ChunkBrokenState {
    pub fn len_for_capacity(capacity: u16) -> usize {
        CHUNK_BROKEN_HEADER_LEN + capacity as usize * CHUNK_BROKEN_RECORD_LEN
    }

    pub fn pack_empty(dst: &mut [u8], args: &ChunkBrokenInitArgs) -> ProgramResult {
        if dst.len() != Self::len_for_capacity(args.capacity) {
            return Err(NicechunkChunkError::InvalidChunkBrokenData.into());
        }
        dst.fill(0);
        dst[0..4].copy_from_slice(&CHUNK_BROKEN_MAGIC);
        dst[4] = CHUNK_BROKEN_VERSION;
        dst[5] = args.bump;
        dst[6..8].copy_from_slice(&0_u16.to_le_bytes());
        dst[8..10].copy_from_slice(&args.capacity.to_le_bytes());
        dst[10..12].copy_from_slice(&args.min_y.to_le_bytes());
        Ok(())
    }

    pub fn validate_header(data: &[u8], min_y: i16) -> Result<(u16, u16), NicechunkChunkError> {
        if data.len() < CHUNK_BROKEN_HEADER_LEN
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
        let packed_value = u32::from_le_bytes([packed[0], packed[1], packed[2], 0]);
        for index in 0..count {
            let offset = CHUNK_BROKEN_HEADER_LEN + index * CHUNK_BROKEN_RECORD_LEN;
            let record_value =
                u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], 0]);
            if record_value == packed_value {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn packed_values(data: &[u8], min_y: i16) -> Result<Vec<u32>, NicechunkChunkError> {
        let (count, _) = Self::validate_header(data, min_y)?;
        let mut values = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let offset = CHUNK_BROKEN_HEADER_LEN + index * CHUNK_BROKEN_RECORD_LEN;
            values.push(u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                0,
            ]));
        }
        Ok(values)
    }

    pub fn append_packed(data: &mut [u8], min_y: i16, packed: [u8; 3]) -> ProgramResult {
        let (count, capacity) = Self::validate_header(data, min_y)?;
        if count >= capacity {
            return Err(NicechunkChunkError::ChunkBrokenCapacityExceeded.into());
        }
        let offset = CHUNK_BROKEN_HEADER_LEN + count as usize * CHUNK_BROKEN_RECORD_LEN;
        data[offset..offset + CHUNK_BROKEN_RECORD_LEN].copy_from_slice(&packed);
        data[6..8].copy_from_slice(&count.saturating_add(1).to_le_bytes());
        Ok(())
    }

    pub fn append_many_packed(data: &mut [u8], min_y: i16, records: &[[u8; 3]]) -> ProgramResult {
        let (count, capacity) = Self::validate_header(data, min_y)?;
        let next_count = usize::from(count)
            .checked_add(records.len())
            .ok_or(NicechunkChunkError::ChunkBrokenCapacityExceeded)?;
        if next_count > usize::from(capacity) {
            return Err(NicechunkChunkError::ChunkBrokenCapacityExceeded.into());
        }
        let mut offset = CHUNK_BROKEN_HEADER_LEN + usize::from(count) * CHUNK_BROKEN_RECORD_LEN;
        for packed in records {
            data[offset..offset + CHUNK_BROKEN_RECORD_LEN].copy_from_slice(packed);
            offset += CHUNK_BROKEN_RECORD_LEN;
        }
        data[6..8].copy_from_slice(&(next_count as u16).to_le_bytes());
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct ResourceDropRule {
    pub source_block_id: u16,
    pub drop_block_id: u16,
    pub chance_bps: u16,
    pub min_altitude: i16,
    pub max_altitude: i16,
    pub min_depth: i16,
    pub max_depth: i16,
    pub salt: u8,
    pub min_volume_mm3: u32,
    pub max_volume_mm3: u32,
}

#[derive(Clone, Copy)]
pub struct ResourceExtraDrop {
    pub block_id: u16,
    pub volume_mm3: u32,
}

impl ResourceDropRule {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() != RESOURCE_DROP_RULE_LEN {
            return Err(NicechunkChunkError::InvalidResourceDropTableData);
        }
        let rule = Self {
            source_block_id: read_u16(data, 0),
            drop_block_id: read_u16(data, 2),
            chance_bps: read_u16(data, 4),
            min_altitude: read_i16(data, 6),
            max_altitude: read_i16(data, 8),
            min_depth: read_i16(data, 10),
            max_depth: read_i16(data, 12),
            salt: data[14],
            min_volume_mm3: read_u32(data, 15),
            max_volume_mm3: read_u32(data, 19),
        };
        rule.validate()?;
        Ok(rule)
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != RESOURCE_DROP_RULE_LEN {
            return Err(NicechunkChunkError::InvalidResourceDropTableData.into());
        }
        self.validate()?;
        dst[0..2].copy_from_slice(&self.source_block_id.to_le_bytes());
        dst[2..4].copy_from_slice(&self.drop_block_id.to_le_bytes());
        dst[4..6].copy_from_slice(&self.chance_bps.to_le_bytes());
        dst[6..8].copy_from_slice(&self.min_altitude.to_le_bytes());
        dst[8..10].copy_from_slice(&self.max_altitude.to_le_bytes());
        dst[10..12].copy_from_slice(&self.min_depth.to_le_bytes());
        dst[12..14].copy_from_slice(&self.max_depth.to_le_bytes());
        dst[14] = self.salt;
        dst[15..19].copy_from_slice(&self.min_volume_mm3.to_le_bytes());
        dst[19..23].copy_from_slice(&self.max_volume_mm3.to_le_bytes());
        Ok(())
    }

    fn validate(&self) -> Result<(), NicechunkChunkError> {
        if self.source_block_id == BLOCK_AIR
            || self.source_block_id == BLOCK_WATER
            || self.source_block_id == BLOCK_BEDROCK
            || self.drop_block_id == BLOCK_AIR
            || self.drop_block_id == BLOCK_WATER
            || self.drop_block_id == BLOCK_BEDROCK
            || self.chance_bps > RESOURCE_DROP_CHANCE_DENOMINATOR as u16
            || self.min_altitude > self.max_altitude
            || self.min_depth > self.max_depth
            || self.min_volume_mm3 == 0
            || self.min_volume_mm3 > self.max_volume_mm3
        {
            return Err(NicechunkChunkError::InvalidResourceDropTableData);
        }
        Ok(())
    }
}

pub struct ResourceDropTableState;

impl ResourceDropTableState {
    pub fn len_for_rules(rule_count: usize) -> usize {
        RESOURCE_DROP_TABLE_HEADER_LEN + rule_count * RESOURCE_DROP_RULE_LEN
    }

    pub fn pack(dst: &mut [u8], bump: u8, rules: &[ResourceDropRule]) -> ProgramResult {
        if rules.is_empty()
            || rules.len() > RESOURCE_DROP_RULE_MAX_COUNT
            || dst.len() != Self::len_for_rules(rules.len())
        {
            return Err(NicechunkChunkError::InvalidResourceDropTableData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&RESOURCE_DROP_TABLE_MAGIC);
        dst[8] = RESOURCE_DROP_TABLE_VERSION;
        dst[9] = bump;
        dst[10] = rules.len() as u8;
        for (index, rule) in rules.iter().enumerate() {
            let offset = RESOURCE_DROP_TABLE_HEADER_LEN + index * RESOURCE_DROP_RULE_LEN;
            rule.pack(&mut dst[offset..offset + RESOURCE_DROP_RULE_LEN])?;
        }
        Ok(())
    }

    pub fn validate_payload(payload: &[u8]) -> Result<usize, NicechunkChunkError> {
        let rule_count = payload.first().copied().unwrap_or_default() as usize;
        if rule_count == 0
            || rule_count > RESOURCE_DROP_RULE_MAX_COUNT
            || payload.len() != 1 + rule_count * RESOURCE_DROP_RULE_LEN
        {
            return Err(NicechunkChunkError::InvalidResourceDropTableData);
        }
        for index in 0..rule_count {
            let offset = 1 + index * RESOURCE_DROP_RULE_LEN;
            ResourceDropRule::unpack(&payload[offset..offset + RESOURCE_DROP_RULE_LEN])?;
        }
        Ok(rule_count)
    }

    pub fn pack_payload(dst: &mut [u8], bump: u8, payload: &[u8]) -> ProgramResult {
        let rule_count = Self::validate_payload(payload)?;
        if dst.len() != Self::len_for_rules(rule_count) {
            return Err(NicechunkChunkError::InvalidResourceDropTableData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&RESOURCE_DROP_TABLE_MAGIC);
        dst[8] = RESOURCE_DROP_TABLE_VERSION;
        dst[9] = bump;
        dst[10] = rule_count as u8;
        dst[RESOURCE_DROP_TABLE_HEADER_LEN..].copy_from_slice(&payload[1..]);
        Ok(())
    }

    pub fn validate_header(data: &[u8]) -> Result<usize, NicechunkChunkError> {
        if data.len() < RESOURCE_DROP_TABLE_HEADER_LEN
            || data[0..8] != RESOURCE_DROP_TABLE_MAGIC
            || data[8] != RESOURCE_DROP_TABLE_VERSION
        {
            return Err(NicechunkChunkError::InvalidResourceDropTableData);
        }
        let rule_count = data[10] as usize;
        if rule_count == 0
            || rule_count > RESOURCE_DROP_RULE_MAX_COUNT
            || data.len() != Self::len_for_rules(rule_count)
        {
            return Err(NicechunkChunkError::InvalidResourceDropTableData);
        }
        Ok(rule_count)
    }
}

pub fn extra_drop_from_table(
    global_config: &GlobalConfigView,
    data: &[u8],
    world_x: i32,
    world_y: i16,
    world_z: i32,
    source_block_id: u16,
    exploration_xp: u64,
) -> Result<Option<ResourceExtraDrop>, NicechunkChunkError> {
    let rule_count = ResourceDropTableState::validate_header(data)?;
    let surface = generated_surface_height(global_config, world_x, world_z);
    let altitude = world_y.saturating_sub(global_config.sea_level);
    let depth = surface.saturating_sub(world_y);
    for index in 0..rule_count {
        let offset = RESOURCE_DROP_TABLE_HEADER_LEN + index * RESOURCE_DROP_RULE_LEN;
        let rule = ResourceDropRule::unpack(&data[offset..offset + RESOURCE_DROP_RULE_LEN])?;
        if rule.source_block_id != source_block_id
            || altitude < rule.min_altitude
            || altitude > rule.max_altitude
            || depth < rule.min_depth
            || depth > rule.max_depth
        {
            continue;
        }
        let roll = hash_coord3(
            &global_config.world_seed,
            world_x,
            world_y as i32,
            world_z,
            700_u32.saturating_add(rule.salt as u32),
        ) % RESOURCE_DROP_CHANCE_DENOMINATOR;
        let chance_bps =
            PlayerProgressState::exploration_chance_bps(rule.chance_bps, exploration_xp);
        if roll < chance_bps as u32 {
            let span = rule.max_volume_mm3.saturating_sub(rule.min_volume_mm3);
            let volume_mm3 = if span == 0 {
                rule.min_volume_mm3
            } else {
                let volume_roll = hash_coord3(
                    &global_config.world_seed,
                    world_x,
                    world_y as i32,
                    world_z,
                    900_u32.saturating_add(rule.salt as u32),
                ) % span.saturating_add(1);
                rule.min_volume_mm3.saturating_add(volume_roll)
            };
            return Ok(Some(ResourceExtraDrop {
                block_id: rule.drop_block_id,
                volume_mm3,
            }));
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceDecorationRule {
    pub rule_id: u16,
    pub decoration_id: u16,
    pub surface_block_id: u16,
    pub drop_block_id: u16,
    pub roll_start_bps: u16,
    pub roll_end_bps: u16,
    pub min_y: i16,
    pub max_y: i16,
    pub salt: u16,
    pub variant: u8,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceDecorationMatch {
    pub rule_id: u16,
    pub decoration_id: u16,
    pub surface_block_id: u16,
    pub drop_block_id: u16,
    pub variant: u8,
    pub flags: u8,
    pub surface_y: i16,
    pub roll: u16,
}

impl SurfaceDecorationRule {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkChunkError> {
        if data.len() != SURFACE_DECORATION_RULE_LEN {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
        }
        let rule = Self {
            rule_id: read_u16(data, 0),
            decoration_id: read_u16(data, 2),
            surface_block_id: read_u16(data, 4),
            drop_block_id: read_u16(data, 6),
            roll_start_bps: read_u16(data, 8),
            roll_end_bps: read_u16(data, 10),
            min_y: read_i16(data, 12),
            max_y: read_i16(data, 14),
            salt: read_u16(data, 16),
            variant: data[18],
            flags: data[19],
        };
        rule.validate()?;
        Ok(rule)
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != SURFACE_DECORATION_RULE_LEN {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData.into());
        }
        self.validate()?;
        dst[0..2].copy_from_slice(&self.rule_id.to_le_bytes());
        dst[2..4].copy_from_slice(&self.decoration_id.to_le_bytes());
        dst[4..6].copy_from_slice(&self.surface_block_id.to_le_bytes());
        dst[6..8].copy_from_slice(&self.drop_block_id.to_le_bytes());
        dst[8..10].copy_from_slice(&self.roll_start_bps.to_le_bytes());
        dst[10..12].copy_from_slice(&self.roll_end_bps.to_le_bytes());
        dst[12..14].copy_from_slice(&self.min_y.to_le_bytes());
        dst[14..16].copy_from_slice(&self.max_y.to_le_bytes());
        dst[16..18].copy_from_slice(&self.salt.to_le_bytes());
        dst[18] = self.variant;
        dst[19] = self.flags;
        Ok(())
    }

    fn validate(&self) -> Result<(), NicechunkChunkError> {
        if self.rule_id == 0
            || self.decoration_id == 0
            || matches!(
                self.surface_block_id,
                BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK
            )
            || matches!(self.drop_block_id, BLOCK_AIR | BLOCK_WATER | BLOCK_BEDROCK)
            || self.roll_start_bps >= self.roll_end_bps
            || self.roll_end_bps > SURFACE_DECORATION_ROLL_DENOMINATOR as u16
            || self.min_y > self.max_y
        {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
        }
        Ok(())
    }
}

pub struct SurfaceDecorationTableState;

impl SurfaceDecorationTableState {
    pub fn pack(
        dst: &mut [u8],
        bump: u8,
        revision: u32,
        rules: &[SurfaceDecorationRule],
    ) -> ProgramResult {
        if dst.len() != SURFACE_DECORATION_TABLE_LEN
            || rules.is_empty()
            || rules.len() > SURFACE_DECORATION_RULE_MAX_COUNT
        {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData.into());
        }
        validate_surface_decoration_rule_ids(rules)?;
        dst.fill(0);
        dst[0..8].copy_from_slice(&SURFACE_DECORATION_TABLE_MAGIC);
        dst[8] = SURFACE_DECORATION_TABLE_VERSION;
        dst[9] = bump;
        dst[10] = rules.len() as u8;
        dst[12..16].copy_from_slice(&revision.to_le_bytes());
        for (index, rule) in rules.iter().enumerate() {
            let offset = SURFACE_DECORATION_TABLE_HEADER_LEN + index * SURFACE_DECORATION_RULE_LEN;
            rule.pack(&mut dst[offset..offset + SURFACE_DECORATION_RULE_LEN])?;
        }
        Ok(())
    }

    pub fn validate_payload(payload: &[u8]) -> Result<usize, NicechunkChunkError> {
        let count = payload.first().copied().unwrap_or_default() as usize;
        if count == 0
            || count > SURFACE_DECORATION_RULE_MAX_COUNT
            || payload.len() != 1 + count * SURFACE_DECORATION_RULE_LEN
        {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
        }
        for index in 0..count {
            let offset = 1 + index * SURFACE_DECORATION_RULE_LEN;
            let rule = SurfaceDecorationRule::unpack(
                &payload[offset..offset + SURFACE_DECORATION_RULE_LEN],
            )?;
            for previous in 0..index {
                let previous_offset = 1 + previous * SURFACE_DECORATION_RULE_LEN;
                if read_u16(payload, previous_offset) == rule.rule_id {
                    return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
                }
            }
        }
        Ok(count)
    }

    pub fn pack_payload(dst: &mut [u8], bump: u8, revision: u32, payload: &[u8]) -> ProgramResult {
        let count = Self::validate_payload(payload)?;
        if dst.len() != SURFACE_DECORATION_TABLE_LEN {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&SURFACE_DECORATION_TABLE_MAGIC);
        dst[8] = SURFACE_DECORATION_TABLE_VERSION;
        dst[9] = bump;
        dst[10] = count as u8;
        dst[12..16].copy_from_slice(&revision.to_le_bytes());
        let rule_bytes = count * SURFACE_DECORATION_RULE_LEN;
        dst[SURFACE_DECORATION_TABLE_HEADER_LEN..SURFACE_DECORATION_TABLE_HEADER_LEN + rule_bytes]
            .copy_from_slice(&payload[1..]);
        Ok(())
    }

    pub fn validate_header(data: &[u8]) -> Result<(usize, u32), NicechunkChunkError> {
        if data.len() != SURFACE_DECORATION_TABLE_LEN
            || data[0..8] != SURFACE_DECORATION_TABLE_MAGIC
            || data[8] != SURFACE_DECORATION_TABLE_VERSION
        {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
        }
        let count = data[10] as usize;
        if count == 0 || count > SURFACE_DECORATION_RULE_MAX_COUNT {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
        }
        Ok((count, read_u32(data, 12)))
    }
}

pub fn unpack_surface_decoration_rules(
    data: &[u8],
) -> Result<(u32, Vec<SurfaceDecorationRule>), NicechunkChunkError> {
    let (count, revision) = SurfaceDecorationTableState::validate_header(data)?;
    let mut rules = Vec::with_capacity(count);
    for index in 0..count {
        let offset = SURFACE_DECORATION_TABLE_HEADER_LEN + index * SURFACE_DECORATION_RULE_LEN;
        rules.push(SurfaceDecorationRule::unpack(
            &data[offset..offset + SURFACE_DECORATION_RULE_LEN],
        )?);
    }
    validate_surface_decoration_rule_ids(&rules)?;
    Ok((revision, rules))
}

pub fn surface_decoration_at(
    global_config: &GlobalConfigView,
    rules: &[SurfaceDecorationRule],
    world_x: i32,
    world_z: i32,
) -> Option<SurfaceDecorationMatch> {
    let surface_y = generated_surface_height(global_config, world_x, world_z);
    if generated_water_level(global_config, world_x, world_z, surface_y)
        .map(|water_y| water_y > surface_y)
        .unwrap_or(false)
    {
        return None;
    }
    if generated_tree_block_id_at(global_config, world_x, surface_y.saturating_add(1), world_z)
        != BLOCK_AIR
    {
        return None;
    }
    let surface_block_id = generated_surface_block_id(global_config, world_x, world_z, surface_y);
    let mut active_salt = None;
    let mut active_roll = 0_u16;
    for rule in rules {
        if rule.surface_block_id != surface_block_id
            || surface_y < rule.min_y
            || surface_y > rule.max_y
        {
            continue;
        }
        if active_salt != Some(rule.salt) {
            active_salt = Some(rule.salt);
            active_roll = (hash_coord3(
                &global_config.world_seed,
                world_x,
                surface_y.saturating_add(1) as i32,
                world_z,
                1200_u32.saturating_add(rule.salt as u32),
            ) % SURFACE_DECORATION_ROLL_DENOMINATOR) as u16;
        }
        if active_roll < rule.roll_start_bps || active_roll >= rule.roll_end_bps {
            continue;
        }
        return Some(SurfaceDecorationMatch {
            rule_id: rule.rule_id,
            decoration_id: rule.decoration_id,
            surface_block_id,
            drop_block_id: rule.drop_block_id,
            variant: rule.variant,
            flags: rule.flags,
            surface_y,
            roll: active_roll,
        });
    }
    None
}

pub fn surface_decoration_from_table(
    global_config: &GlobalConfigView,
    data: &[u8],
    world_x: i32,
    world_z: i32,
) -> Result<Option<SurfaceDecorationMatch>, NicechunkChunkError> {
    let (count, _) = SurfaceDecorationTableState::validate_header(data)?;
    let surface_y = generated_surface_height(global_config, world_x, world_z);
    if generated_water_level(global_config, world_x, world_z, surface_y)
        .map(|water_y| water_y > surface_y)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    if generated_tree_block_id_at(global_config, world_x, surface_y.saturating_add(1), world_z)
        != BLOCK_AIR
    {
        return Ok(None);
    }
    let surface_block_id = generated_surface_block_id(global_config, world_x, world_z, surface_y);
    let mut active_salt = None;
    let mut active_roll = 0_u16;
    for index in 0..count {
        let offset = SURFACE_DECORATION_TABLE_HEADER_LEN + index * SURFACE_DECORATION_RULE_LEN;
        let rule =
            SurfaceDecorationRule::unpack(&data[offset..offset + SURFACE_DECORATION_RULE_LEN])?;
        if rule.surface_block_id != surface_block_id
            || surface_y < rule.min_y
            || surface_y > rule.max_y
        {
            continue;
        }
        if active_salt != Some(rule.salt) {
            active_salt = Some(rule.salt);
            active_roll = (hash_coord3(
                &global_config.world_seed,
                world_x,
                surface_y.saturating_add(1) as i32,
                world_z,
                1200_u32.saturating_add(rule.salt as u32),
            ) % SURFACE_DECORATION_ROLL_DENOMINATOR) as u16;
        }
        if active_roll < rule.roll_start_bps || active_roll >= rule.roll_end_bps {
            continue;
        }
        return Ok(Some(SurfaceDecorationMatch {
            rule_id: rule.rule_id,
            decoration_id: rule.decoration_id,
            surface_block_id,
            drop_block_id: rule.drop_block_id,
            variant: rule.variant,
            flags: rule.flags,
            surface_y,
            roll: active_roll,
        }));
    }
    Ok(None)
}

fn validate_surface_decoration_rule_ids(
    rules: &[SurfaceDecorationRule],
) -> Result<(), NicechunkChunkError> {
    for (index, rule) in rules.iter().enumerate() {
        rule.validate()?;
        if rules[..index]
            .iter()
            .any(|previous| previous.rule_id == rule.rule_id)
        {
            return Err(NicechunkChunkError::InvalidSurfaceDecorationTableData);
        }
    }
    Ok(())
}

pub fn pack_backpack_resource_y(world_y: i16, block_id: u16, min_y: i16) -> i16 {
    let y_offset = world_y as i32 - min_y as i32;
    if (0..=BACKPACK_PACKED_Y_MASK).contains(&y_offset)
        && block_id > 0
        && block_id < (1_u16 << (16 - BACKPACK_PACKED_Y_BITS))
    {
        ((block_id as i32) << BACKPACK_PACKED_Y_BITS | y_offset) as i16
    } else {
        world_y
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_global_config_view() -> GlobalConfigView {
        GlobalConfigView {
            development_wallet: Pubkey::new_unique(),
            world_seed: CANONICAL_WORLD_SEED,
            chunk_size: CANONICAL_CHUNK_SIZE,
            min_build_y: CANONICAL_MIN_BUILD_Y,
            max_build_y: CANONICAL_MAX_BUILD_Y,
            max_terrain_height: CANONICAL_MAX_TERRAIN_HEIGHT,
            sea_level: CANONICAL_SEA_LEVEL,
        }
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
    fn batch_mine_args_require_debug_mode_and_compact_same_shape_records() {
        let blocks = [
            MineBlockArgs {
                world_x: 7,
                world_y: 98,
                world_z: -4,
                expected_block_id: BLOCK_GRASS,
            },
            MineBlockArgs {
                world_x: 8,
                world_y: 97,
                world_z: -4,
                expected_block_id: BLOCK_DIRT,
            },
        ];
        let mut payload = vec![BATCH_MINE_MODE_DEBUG, blocks.len() as u8];
        for block in blocks {
            payload.extend_from_slice(&block.world_x.to_le_bytes());
            payload.extend_from_slice(&block.world_y.to_le_bytes());
            payload.extend_from_slice(&block.world_z.to_le_bytes());
            payload.extend_from_slice(&block.expected_block_id.to_le_bytes());
        }
        let decoded = BatchMineArgs::unpack(&payload).unwrap();
        assert_eq!(decoded.mode, BATCH_MINE_MODE_DEBUG);
        assert_eq!(decoded.blocks.len(), 2);
        assert_eq!(decoded.blocks[1].world_x, 8);
        assert_eq!(decoded.blocks[1].expected_block_id, BLOCK_DIRT);

        payload[0] = 0;
        assert!(matches!(
            BatchMineArgs::unpack(&payload),
            Err(NicechunkChunkError::InvalidBatchMine)
        ));
        assert!(matches!(
            BatchMineArgs::unpack(&[BATCH_MINE_MODE_DEBUG, 0]),
            Err(NicechunkChunkError::InvalidBatchMine)
        ));
    }

    #[test]
    fn range_mine_args_decode_a_canonical_640_block_bitmap() {
        let size_x = 16_u8;
        let size_y = 5_u16;
        let size_z = 8_u8;
        let count = usize::from(size_x) * usize::from(size_y) * usize::from(size_z);
        let mut payload = Vec::with_capacity(15 + count.div_ceil(8) + (count * 6).div_ceil(8));
        payload.push(RANGE_MINE_MODE_DEBUG);
        payload.extend_from_slice(&32_i32.to_le_bytes());
        payload.extend_from_slice(&100_i16.to_le_bytes());
        payload.extend_from_slice(&(-16_i32).to_le_bytes());
        payload.push(size_x);
        payload.extend_from_slice(&size_y.to_le_bytes());
        payload.push(size_z);
        payload.extend(std::iter::repeat(0xff).take(count.div_ceil(8)));
        let mut block_ids = vec![0_u8; (count * 6).div_ceil(8)];
        for index in 0..count {
            let bit_index = index * 6;
            let byte_index = bit_index / 8;
            let shift = bit_index % 8;
            let packed = u16::from(BLOCK_STONE) << shift;
            block_ids[byte_index] |= packed as u8;
            if byte_index + 1 < block_ids.len() {
                block_ids[byte_index + 1] |= (packed >> 8) as u8;
            }
        }
        payload.extend_from_slice(&block_ids);

        let decoded = RangeMineArgs::unpack(&payload).unwrap();
        assert_eq!(decoded.mode, RANGE_MINE_MODE_DEBUG);
        assert_eq!(decoded.blocks.len(), RANGE_MINE_MAX_BLOCKS);
        assert_eq!(decoded.blocks[0].world_x, 32);
        assert_eq!(decoded.blocks[0].world_y, 100);
        assert_eq!(decoded.blocks[0].world_z, -16);
        assert_eq!(decoded.blocks.last().unwrap().world_x, 47);
        assert_eq!(decoded.blocks.last().unwrap().world_y, 104);
        assert_eq!(decoded.blocks.last().unwrap().world_z, -9);
        assert!(decoded
            .blocks
            .iter()
            .all(|block| block.expected_block_id == BLOCK_STONE));
    }

    #[test]
    fn range_mine_args_reject_oversized_or_noncanonical_payloads() {
        let mut oversized = vec![0_u8; 15];
        oversized[0] = RANGE_MINE_MODE_DEBUG;
        oversized[11] = 16;
        oversized[12..14].copy_from_slice(&3_u16.to_le_bytes());
        oversized[14] = 16;
        assert!(matches!(
            RangeMineArgs::unpack(&oversized),
            Err(NicechunkChunkError::InvalidRangeMine)
        ));

        let mut empty = vec![0_u8; 16];
        empty[0] = RANGE_MINE_MODE_DEBUG;
        empty[11] = 1;
        empty[12..14].copy_from_slice(&1_u16.to_le_bytes());
        empty[14] = 1;
        assert!(matches!(
            RangeMineArgs::unpack(&empty),
            Err(NicechunkChunkError::InvalidRangeMine)
        ));
    }

    #[test]
    fn batch_mine_drop_gate_is_deterministic_and_respects_probability_bounds() {
        let config = test_global_config_view();
        let block = MineBlockArgs {
            world_x: 123,
            world_y: 87,
            world_z: -456,
            expected_block_id: BLOCK_STONE,
        };
        assert!(!batch_mine_reward_passes(&config, &block, 9, 0));
        assert!(batch_mine_reward_passes(
            &config,
            &block,
            9,
            RESOURCE_DROP_CHANCE_DENOMINATOR as u16,
        ));
        assert_eq!(
            batch_mine_reward_passes(&config, &block, 17, BATCH_MINE_BASE_DROP_CHANCE_BPS),
            batch_mine_reward_passes(&config, &block, 17, BATCH_MINE_BASE_DROP_CHANCE_BPS),
        );
    }

    #[test]
    fn range_mine_drop_gate_is_deterministic_and_respects_probability_bounds() {
        let config = test_global_config_view();
        let block = MineBlockArgs {
            world_x: 123,
            world_y: 87,
            world_z: -456,
            expected_block_id: BLOCK_STONE,
        };
        assert!(!range_mine_reward_passes(&config, &block, 9, 0));
        assert!(range_mine_reward_passes(
            &config,
            &block,
            9,
            RESOURCE_DROP_CHANCE_DENOMINATOR as u16,
        ));
        assert_eq!(
            range_mine_reward_passes(&config, &block, 17, RANGE_MINE_BASE_DROP_CHANCE_BPS),
            range_mine_reward_passes(&config, &block, 17, RANGE_MINE_BASE_DROP_CHANCE_BPS),
        );
    }

    #[test]
    fn legacy_core_terrain_bytes_cannot_override_chunk_js_world() {
        let mut legacy = vec![0xff_u8; GLOBAL_CONFIG_LEN];
        legacy[0..8].copy_from_slice(&GLOBAL_CONFIG_MAGIC);
        let config = GlobalConfigView::unpack(&legacy).unwrap();
        assert_eq!(config.development_wallet.as_ref(), &[0xff; 32]);
        assert_eq!(config.world_seed, CANONICAL_WORLD_SEED);
        assert_eq!(config.chunk_size, CANONICAL_CHUNK_SIZE);
        assert_eq!(config.min_build_y, CANONICAL_MIN_BUILD_Y);
        assert_eq!(config.max_build_y, CANONICAL_MAX_BUILD_Y);
        assert_eq!(config.max_terrain_height, CANONICAL_MAX_TERRAIN_HEIGHT);
        assert_eq!(config.sea_level, CANONICAL_SEA_LEVEL);
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
        };
        let below_surface = GeneratedBlockArgs {
            y: surface - 1,
            ..at_surface
        };
        let deep = GeneratedBlockArgs {
            y: config.min_build_y + 1,
            ..at_surface
        };
        let bedrock = GeneratedBlockArgs {
            y: config.min_build_y,
            ..at_surface
        };
        assert_eq!(
            generated_block_id_at(&config, &at_surface),
            generated_surface_block_id(&config, 1, 2, surface)
        );
        assert_eq!(
            generated_block_id_at(&config, &below_surface),
            generated_subsurface_block_id(&config, 1, 2, surface)
        );
        assert_eq!(generated_block_id_at(&config, &deep), BLOCK_DEEP_STONE);
        assert_eq!(generated_block_id_at(&config, &bedrock), BLOCK_BEDROCK);
    }

    #[test]
    fn non_tree_surface_objects_are_resolved_only_from_the_rule_pda() {
        let config = test_global_config_view();
        let former_cactus = GeneratedBlockArgs {
            chunk_x: 38,
            chunk_z: 62,
            local_x: 9,
            y: 115,
            local_z: 10,
        };
        assert_eq!(generated_block_id_at(&config, &former_cactus), BLOCK_AIR);

        let rules = test_surface_decoration_rules();
        let decoration = surface_decoration_at(&config, &rules, 826, -1997).unwrap();
        assert_eq!(decoration.rule_id, 20);
        assert_eq!(decoration.decoration_id, 8);
        assert_eq!(decoration.drop_block_id, 32);
        assert_eq!(decoration.roll, 2);
    }

    #[test]
    fn surface_decoration_table_is_fixed_capacity_and_round_trips() {
        let rules = test_surface_decoration_rules();
        let mut data = vec![0_u8; SURFACE_DECORATION_TABLE_LEN];
        SurfaceDecorationTableState::pack(&mut data, 247, 3, &rules).unwrap();
        assert_eq!(&data[0..8], &SURFACE_DECORATION_TABLE_MAGIC);
        assert_eq!(data[9], 247);
        assert_eq!(data[10] as usize, rules.len());
        let (revision, decoded) = unpack_surface_decoration_rules(&data).unwrap();
        assert_eq!(revision, 3);
        assert_eq!(decoded, rules);

        let payload = surface_decoration_payload(&rules);
        let mut payload_data = vec![0_u8; SURFACE_DECORATION_TABLE_LEN];
        SurfaceDecorationTableState::pack_payload(&mut payload_data, 247, 3, &payload).unwrap();
        assert_eq!(payload_data, data);

        let mut duplicate = rules.clone();
        duplicate[1].rule_id = duplicate[0].rule_id;
        assert!(SurfaceDecorationTableState::pack(&mut data, 247, 4, &duplicate).is_err());
        assert!(
            SurfaceDecorationTableState::validate_payload(&surface_decoration_payload(&duplicate))
                .is_err()
        );
    }

    #[test]
    fn surface_decoration_coordinates_match_chunk_js_rules() {
        let config = test_global_config_view();
        let rules = test_surface_decoration_rules();
        let payload = surface_decoration_payload(&rules);
        let mut table = vec![0_u8; SURFACE_DECORATION_TABLE_LEN];
        SurfaceDecorationTableState::pack_payload(&mut table, 247, 3, &payload).unwrap();
        let cases = [
            (826, -1997, 5, 20, 8, 2),
            (-131, -131, 5, 21, 103, 38),
            (-38, -185, 1, 5, 100, 386),
            (81, 172, 7, 32, 105, 139),
            (87, -165, 8, 36, 104, 403),
            (-349, -307, 3, 12, 101, 181),
            (-562, -261, 11, 51, 102, 275),
        ];
        for (x, z, surface_block_id, rule_id, decoration_id, roll) in cases {
            let found = surface_decoration_at(&config, &rules, x, z).unwrap();
            assert_eq!(
                surface_decoration_from_table(&config, &table, x, z).unwrap(),
                Some(found)
            );
            assert_eq!(
                found.surface_block_id, surface_block_id,
                "surface at {x},{z}"
            );
            assert_eq!(found.rule_id, rule_id, "rule at {x},{z}");
            assert_eq!(found.decoration_id, decoration_id, "decoration at {x},{z}");
            assert_eq!(found.roll, roll, "roll at {x},{z}");
        }
    }

    #[test]
    fn surface_decoration_is_suppressed_when_a_tree_occupies_the_bound_face() {
        let config = test_global_config_view();
        let x = 799;
        let z = -999;
        let surface = generated_surface_height(&config, x, z);
        assert_ne!(
            generated_tree_block_id_at(&config, x, surface.saturating_add(1), z),
            BLOCK_AIR
        );
        let rules = [SurfaceDecorationRule {
            rule_id: 4,
            decoration_id: 4,
            surface_block_id: BLOCK_GRASS,
            drop_block_id: 28,
            roll_start_bps: 210,
            roll_end_bps: 370,
            min_y: -32,
            max_y: 320,
            salt: 201,
            variant: 0,
            flags: 3,
        }];
        assert_eq!(surface_decoration_at(&config, &rules, x, z), None);
    }

    #[test]
    fn resource_drop_table_payload_is_zero_copy_compatible() {
        let config = test_global_config_view();
        let rules = [ResourceDropRule {
            source_block_id: BLOCK_GRASS,
            drop_block_id: BLOCK_STONE,
            chance_bps: RESOURCE_DROP_CHANCE_DENOMINATOR as u16,
            min_altitude: i16::MIN,
            max_altitude: i16::MAX,
            min_depth: i16::MIN,
            max_depth: i16::MAX,
            salt: 9,
            min_volume_mm3: 123,
            max_volume_mm3: 123,
        }];
        let mut payload = vec![rules.len() as u8];
        payload.resize(1 + rules.len() * RESOURCE_DROP_RULE_LEN, 0);
        rules[0].pack(&mut payload[1..]).unwrap();
        let mut table = vec![0_u8; ResourceDropTableState::len_for_rules(rules.len())];
        ResourceDropTableState::pack_payload(&mut table, 201, &payload).unwrap();
        assert_eq!(ResourceDropTableState::validate_header(&table).unwrap(), 1);
        let drop = extra_drop_from_table(&config, &table, 256, 114, 896, BLOCK_GRASS, 0)
            .unwrap()
            .unwrap();
        assert_eq!(drop.block_id, BLOCK_STONE);
        assert_eq!(drop.volume_mm3, 123);
    }

    #[test]
    fn canonical_resource_vectors_match_frontend() {
        let config = test_global_config_view();
        let cases = [
            (0, 85, 0, 85, BLOCK_SAND),
            (0, 96, 0, 85, BLOCK_WATER),
            (12, 27, -34, 83, BLOCK_COAL),
            (-246, 97, 164, 97, BLOCK_SAND),
            (287, 104, -294, 104, BLOCK_MUD),
            (256, 114, 896, 114, BLOCK_GRASS),
            (20, 45, 285, 101, BLOCK_COAL),
            (800, 132, 800, 132, BLOCK_STONE),
        ];
        for (world_x, y, world_z, surface, block_id) in cases {
            assert_eq!(generated_surface_height(&config, world_x, world_z), surface);
            let args = GeneratedBlockArgs {
                chunk_x: div_floor_i32(world_x, config.chunk_size as i32),
                chunk_z: div_floor_i32(world_z, config.chunk_size as i32),
                local_x: world_x.rem_euclid(config.chunk_size as i32) as u8,
                y,
                local_z: world_z.rem_euclid(config.chunk_size as i32) as u8,
            };
            assert_eq!(generated_block_id_at(&config, &args), block_id);
        }
    }

    #[test]
    fn tree_candidate_scan_matches_nearby_block_scan() {
        let config = test_global_config_view();
        let mut queries = Vec::new();
        for world_x in 248..264 {
            for world_z in 888..904 {
                let surface = generated_surface_height(&config, world_x, world_z);
                for y in surface.saturating_add(1)..=surface.saturating_add(9) {
                    queries.push((world_x, y, world_z));
                }
            }
        }
        let fast_started = std::time::Instant::now();
        let fast: Vec<_> = queries
            .iter()
            .map(|&(x, y, z)| generated_tree_block_id_at(&config, x, y, z))
            .collect();
        let fast_elapsed = fast_started.elapsed();
        let slow_started = std::time::Instant::now();
        let slow: Vec<_> = queries
            .iter()
            .map(|&(x, y, z)| slow_generated_tree_block_id_at(&config, x, y, z))
            .collect();
        let slow_elapsed = slow_started.elapsed();
        assert_eq!(fast, slow);
        let tree_blocks = fast.iter().filter(|&&block| block != BLOCK_AIR).count();
        assert!(tree_blocks > 0, "tree comparison region must contain trees");
        eprintln!(
            "tree validation benchmark: candidates={fast_elapsed:?}, brute_force={slow_elapsed:?}, queries={}",
            queries.len()
        );
    }

    #[test]
    fn generated_water_level_never_exceeds_static_upper_bound() {
        let config = test_global_config_view();
        for world_x in -96..96 {
            for world_z in -96..96 {
                let surface = generated_surface_height(&config, world_x, world_z);
                if let Some(level) = generated_water_level(&config, world_x, world_z, surface) {
                    assert!(
                        level <= config.sea_level + MAX_WATER_LEVEL_ABOVE_SEA,
                        "water level {level} exceeded static bound at {world_x},{world_z}"
                    );
                }
            }
        }
    }

    fn slow_generated_tree_block_id_at(
        global_config: &GlobalConfigView,
        world_x: i32,
        y: i16,
        world_z: i32,
    ) -> u16 {
        for tree_z in world_z - TREE_MAX_LEAF_RADIUS..=world_z + TREE_MAX_LEAF_RADIUS {
            for tree_x in world_x - TREE_MAX_LEAF_RADIUS..=world_x + TREE_MAX_LEAF_RADIUS {
                let surface = generated_surface_height(global_config, tree_x, tree_z);
                if !generated_can_grow_tree(global_config, tree_x, tree_z, surface) {
                    continue;
                }
                let tree = generated_tree_at(global_config, tree_x, tree_z, surface);
                if !tree.exists {
                    continue;
                }
                let block = generated_tree_volume_block(global_config, &tree, world_x, y, world_z);
                if block != BLOCK_AIR {
                    return block;
                }
            }
        }
        BLOCK_AIR
    }

    #[test]
    fn generated_block_id_can_return_coal_in_deep_seams() {
        let config = test_global_config_view();
        let mut found = false;
        'outer: for world_x in -128..128 {
            for world_z in -128..128 {
                let surface = generated_surface_height(&config, world_x, world_z);
                for y in config.min_build_y.saturating_add(5)..surface.saturating_sub(10) {
                    let args = GeneratedBlockArgs {
                        chunk_x: div_floor_i32(world_x, config.chunk_size as i32),
                        chunk_z: div_floor_i32(world_z, config.chunk_size as i32),
                        local_x: world_x.rem_euclid(config.chunk_size as i32) as u8,
                        y,
                        local_z: world_z.rem_euclid(config.chunk_size as i32) as u8,
                    };
                    if generated_block_id_at(&config, &args) == BLOCK_COAL {
                        found = true;
                        break 'outer;
                    }
                }
            }
        }
        assert!(found);
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
    fn foundation_chunk_index_handles_negative_boundaries_and_protection() {
        let global_config = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let record = FoundationRecord {
            owner,
            foundation_id: 42,
            min_x: -1,
            min_z: 15,
            surface_y: 101,
            width: 16,
            depth: 2,
        };
        let mut data = vec![0_u8; FOUNDATION_CHUNK_LEN];
        FoundationChunkState::pack_empty(&mut data, 199, &global_config, -1, 0).unwrap();
        FoundationChunkState::append(&mut data, &global_config, -1, 0, &record).unwrap();
        assert!(
            FoundationChunkState::protects(&data, &global_config, -1, 0, -1, 100, 15,).unwrap()
        );
        assert!(
            !FoundationChunkState::protects(&data, &global_config, -1, 0, -1, 99, 15,).unwrap()
        );
        assert!(
            !FoundationChunkState::protects(&data, &global_config, -1, 0, -2, 100, 15,).unwrap()
        );

        let overlap = FoundationRecord {
            owner,
            foundation_id: 43,
            min_x: 14,
            min_z: 16,
            surface_y: 200,
            width: 2,
            depth: 2,
        };
        assert!(FoundationChunkState::overlaps(&data, &global_config, -1, 0, &overlap,).unwrap());
    }

    #[test]
    fn foundation_chunk_v1_records_migrate_to_v2_without_losing_protection() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let legacy = FoundationRecord {
            owner,
            foundation_id: 77,
            min_x: 32,
            min_z: -17,
            surface_y: 109,
            width: 12,
            depth: 8,
        };
        let mut v1 = vec![0_u8; FOUNDATION_CHUNK_LEN];
        FoundationChunkState::pack_empty(&mut v1, 211, &global_config, 2, -2).unwrap();
        FoundationChunkState::append(&mut v1, &global_config, 2, -2, &legacy).unwrap();

        let records = FoundationChunkState::records(&v1, &global_config, 2, -2).unwrap();
        let mut v2 = vec![0_u8; FoundationChunkV2State::len(4).unwrap()];
        FoundationChunkV2State::pack_empty(&mut v2, 211, &global_config, 2, -2, 4).unwrap();
        for record in &records {
            FoundationChunkV2State::append(
                &mut v2,
                &global_config,
                2,
                -2,
                &FoundationRecordV2::from_legacy(record),
            )
            .unwrap();
        }

        assert_eq!(
            FoundationChunkV2State::records(&v2, &global_config, 2, -2).unwrap(),
            vec![FoundationRecordV2::from_legacy(&legacy)]
        );
        assert!(FoundationChunkState::protects(&v2, &global_config, 2, -2, 32, 108, -17).unwrap());
    }

    #[test]
    fn blueprint_id_can_supersede_its_same_owner_v1_index_during_upgrade() {
        let owner = Pubkey::new_unique();
        let legacy = FoundationRecordV2 {
            owner,
            foundation_id: 7_482_402_287_136_659_184,
            min_x: 747,
            min_z: 780,
            surface_y: 136,
            width: 16,
            depth: 16,
        };
        let blueprint = FoundationRecordV2 {
            owner,
            foundation_id: 12_065_219_072_965_175_186,
            min_x: 748,
            min_z: 781,
            surface_y: 136,
            width: 12,
            depth: 8,
        };

        assert!(blueprint.supersedes_legacy_index(&legacy, 16));
        assert!(!FoundationRecordV2 {
            owner: Pubkey::new_unique(),
            ..blueprint
        }
        .supersedes_legacy_index(&legacy, 16));
        assert!(!FoundationRecordV2 {
            foundation_id: legacy.foundation_id,
            ..blueprint
        }
        .supersedes_legacy_index(&legacy, 16));
        assert!(!FoundationRecordV2 {
            min_x: 752,
            width: 12,
            ..blueprint
        }
        .supersedes_legacy_index(&legacy, 16));
    }

    #[test]
    fn foundation_chunk_v2_supports_large_footprints_and_only_protects_the_base_layer() {
        let global_config = Pubkey::new_unique();
        let record = FoundationRecordV2 {
            owner: Pubkey::new_unique(),
            foundation_id: 901,
            min_x: -300,
            min_z: 700,
            surface_y: 140,
            width: 1_024,
            depth: 513,
        };
        let mut data = vec![0_u8; FoundationChunkV2State::len(4).unwrap()];
        FoundationChunkV2State::pack_empty(&mut data, 9, &global_config, -19, 43, 4).unwrap();
        FoundationChunkV2State::append(&mut data, &global_config, -19, 43, &record).unwrap();

        assert!(
            FoundationChunkV2State::protects(&data, &global_config, -19, 43, 723, 139, 1_212,)
                .unwrap()
        );
        assert!(
            !FoundationChunkV2State::protects(&data, &global_config, -19, 43, 723, 140, 1_212,)
                .unwrap()
        );
        assert!(
            !FoundationChunkV2State::protects(&data, &global_config, -19, 43, 724, 139, 1_212,)
                .unwrap()
        );
    }

    #[test]
    fn foundation_chunk_v2_registration_is_idempotent_and_rejects_overlap() {
        let global_config = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let original = FoundationRecordV2 {
            owner,
            foundation_id: 10,
            min_x: 0,
            min_z: 0,
            surface_y: 100,
            width: 32,
            depth: 32,
        };
        let mut data = vec![0_u8; FoundationChunkV2State::len(4).unwrap()];
        FoundationChunkV2State::pack_empty(&mut data, 17, &global_config, 0, 0, 4).unwrap();
        FoundationChunkV2State::append(&mut data, &global_config, 0, 0, &original).unwrap();
        FoundationChunkV2State::append(&mut data, &global_config, 0, 0, &original).unwrap();
        assert_eq!(
            FoundationChunkV2State::validate(&data, &global_config, 0, 0).unwrap(),
            (1, 4)
        );

        let resized = FoundationRecordV2 {
            width: 16,
            depth: 16,
            ..original
        };
        FoundationChunkV2State::append(&mut data, &global_config, 0, 0, &resized).unwrap();
        assert_eq!(
            FoundationChunkV2State::records(&data, &global_config, 0, 0).unwrap(),
            vec![resized]
        );

        let overlap = FoundationRecordV2 {
            foundation_id: 11,
            min_x: 15,
            min_z: 15,
            width: 2,
            depth: 2,
            ..original
        };
        assert_eq!(
            FoundationChunkV2State::append(&mut data, &global_config, 0, 0, &overlap).unwrap_err(),
            NicechunkChunkError::FoundationOverlap.into()
        );

        let adjacent = FoundationRecordV2 {
            foundation_id: 12,
            min_x: 16,
            min_z: 0,
            width: 2,
            depth: 2,
            ..original
        };
        FoundationChunkV2State::append(&mut data, &global_config, 0, 0, &adjacent).unwrap();
        assert_eq!(
            FoundationChunkV2State::validate(&data, &global_config, 0, 0).unwrap(),
            (2, 4)
        );
        FoundationChunkV2State::remove(
            &mut data,
            &global_config,
            0,
            0,
            &owner,
            original.foundation_id,
        )
        .unwrap();
        FoundationChunkV2State::remove(
            &mut data,
            &global_config,
            0,
            0,
            &owner,
            original.foundation_id,
        )
        .unwrap();
        assert_eq!(
            FoundationChunkV2State::records(&data, &global_config, 0, 0).unwrap(),
            vec![adjacent]
        );
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
    fn packed_broken_coords_append_in_one_header_update() {
        let mut data = vec![0_u8; ChunkBrokenState::len_for_capacity(4)];
        ChunkBrokenState::pack_empty(
            &mut data,
            &ChunkBrokenInitArgs {
                bump: 252,
                min_y: -32,
                capacity: 4,
            },
        )
        .unwrap();
        let records = [
            pack_broken_coord(1, 20, 2, -32).unwrap(),
            pack_broken_coord(3, 21, 4, -32).unwrap(),
            pack_broken_coord(5, 22, 6, -32).unwrap(),
        ];
        ChunkBrokenState::append_many_packed(&mut data, -32, &records).unwrap();
        assert_eq!(
            ChunkBrokenState::packed_values(&data, -32).unwrap().len(),
            3
        );
        assert_eq!(u16::from_le_bytes(data[6..8].try_into().unwrap()), 3);
        assert!(ChunkBrokenState::contains_packed(&data, records[2]).unwrap());
    }

    #[test]
    fn packed_broken_coord_rejects_out_of_range_y() {
        assert!(pack_broken_coord(0, -33, 0, -32).is_err());
        assert!(pack_broken_coord(0, 480, 0, -32).is_err());
    }

    fn surface_decoration_payload(rules: &[SurfaceDecorationRule]) -> Vec<u8> {
        let mut payload = vec![rules.len() as u8];
        payload.resize(1 + rules.len() * SURFACE_DECORATION_RULE_LEN, 0);
        for (index, rule) in rules.iter().enumerate() {
            let offset = 1 + index * SURFACE_DECORATION_RULE_LEN;
            rule.pack(&mut payload[offset..offset + SURFACE_DECORATION_RULE_LEN])
                .unwrap();
        }
        payload
    }

    fn test_surface_decoration_rules() -> Vec<SurfaceDecorationRule> {
        vec![
            SurfaceDecorationRule {
                rule_id: 20,
                decoration_id: 8,
                surface_block_id: BLOCK_SAND,
                drop_block_id: 32,
                roll_start_bps: 0,
                roll_end_bps: 24,
                min_y: -32,
                max_y: 320,
                salt: 205,
                variant: 0,
                flags: 3,
            },
            SurfaceDecorationRule {
                rule_id: 21,
                decoration_id: 103,
                surface_block_id: BLOCK_SAND,
                drop_block_id: BLOCK_STONE,
                roll_start_bps: 24,
                roll_end_bps: 46,
                min_y: -32,
                max_y: 320,
                salt: 205,
                variant: 0,
                flags: 3,
            },
            SurfaceDecorationRule {
                rule_id: 5,
                decoration_id: 100,
                surface_block_id: BLOCK_GRASS,
                drop_block_id: BLOCK_GRAVEL,
                roll_start_bps: 370,
                roll_end_bps: 425,
                min_y: -32,
                max_y: 320,
                salt: 201,
                variant: 0,
                flags: 3,
            },
            SurfaceDecorationRule {
                rule_id: 32,
                decoration_id: 105,
                surface_block_id: BLOCK_CLAY,
                drop_block_id: BLOCK_STONE,
                roll_start_bps: 130,
                roll_end_bps: 250,
                min_y: -32,
                max_y: 320,
                salt: 207,
                variant: 0,
                flags: 3,
            },
            SurfaceDecorationRule {
                rule_id: 36,
                decoration_id: 104,
                surface_block_id: BLOCK_MUD,
                drop_block_id: BLOCK_STONE,
                roll_start_bps: 400,
                roll_end_bps: 450,
                min_y: -32,
                max_y: 320,
                salt: 208,
                variant: 0,
                flags: 3,
            },
            SurfaceDecorationRule {
                rule_id: 12,
                decoration_id: 101,
                surface_block_id: BLOCK_STONE,
                drop_block_id: BLOCK_STONE,
                roll_start_bps: 165,
                roll_end_bps: 360,
                min_y: -32,
                max_y: 320,
                salt: 203,
                variant: 0,
                flags: 3,
            },
            SurfaceDecorationRule {
                rule_id: 51,
                decoration_id: 102,
                surface_block_id: BLOCK_SNOW,
                drop_block_id: BLOCK_STONE,
                roll_start_bps: 200,
                roll_end_bps: 360,
                min_y: -32,
                max_y: 320,
                salt: 211,
                variant: 0,
                flags: 7,
            },
        ]
    }
}
