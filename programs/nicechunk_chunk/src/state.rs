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
pub const RESOURCE_DROP_TABLE_MAGIC: [u8; 8] = *b"NCKDRP01";
pub const RESOURCE_DROP_TABLE_VERSION: u8 = 1;
pub const RESOURCE_DROP_TABLE_SEED: &[u8] = b"resource-drops";
pub const RESOURCE_DROP_TABLE_HEADER_LEN: usize = 16;
pub const RESOURCE_DROP_RULE_LEN: usize = 15;
pub const RESOURCE_DROP_RULE_MAX_COUNT: usize = 64;
pub const RESOURCE_DROP_CHANCE_DENOMINATOR: u32 = 10_000;
pub const BACKPACK_PACKED_Y_BITS: i32 = 9;
pub const BACKPACK_PACKED_Y_MASK: i32 = (1 << BACKPACK_PACKED_Y_BITS) - 1;

pub const GLOBAL_CONFIG_LEN: usize = 293;
pub const GLOBAL_CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
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
        return if tree_block != BLOCK_AIR {
            tree_block
        } else {
            BLOCK_AIR
        };
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

fn generated_coal_seam_at(
    global_config: &GlobalConfigView,
    world_x: i32,
    y: i16,
    world_z: i32,
    surface: i16,
) -> bool {
    if y <= global_config.min_build_y.saturating_add(3) || y >= surface.saturating_sub(7) {
        return false;
    }
    let seam_mass = hash_coord3(
        &global_config.world_seed,
        div_floor_i32(world_x, 8),
        div_floor_i32(y as i32, 4),
        div_floor_i32(world_z, 8),
        301,
    ) % 100;
    if seam_mass < 84 {
        return false;
    }
    hash_coord3(
        &global_config.world_seed,
        world_x.saturating_add((y as i32).saturating_mul(3)),
        y as i32,
        world_z.saturating_sub((y as i32).saturating_mul(5)),
        302,
    ) % 100
        >= 38
}

pub fn generated_surface_height(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
) -> i16 {
    let min_surface = global_config
        .min_build_y
        .saturating_add(8)
        .max(global_config.sea_level.saturating_sub(28));
    let max_surface = global_config
        .max_terrain_height
        .min(global_config.max_build_y.saturating_sub(1))
        .max(min_surface);

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

    let mountain_ridge =
        (value_noise2(&global_config.world_seed, wx, wz, 96, 29) as i32 - 128).abs();
    let ridge_lift = smooth_range_fixed(mountain_ridge, 70, 124);
    let mountain_mass = scale_by_fixed(
        smooth_range_fixed(
            value_noise2(&global_config.world_seed, wx, wz, 300, 30) as i32,
            194,
            244,
        ) as i32,
        inland,
    );
    let mountain = scale_by_fixed(6 + scale_by_fixed(20, ridge_lift), mountain_mass as u32);

    let mut land = global_config.sea_level as i32
        + 7
        + (inland as i32 * 8) / 1024
        + scale_by_fixed(plains + scale_by_fixed(hills + rolling, roughness), inland)
        + mountain;
    if terrain.water_mask > 0 {
        let water_level = generated_inland_water_level(global_config, wx, wz);
        let water_bed = water_level as i32 - 3 - (terrain.water_mask as i32 * 2) / 1024
            + (value_noise2(&global_config.world_seed, wx, wz, 32, 39) as i32 - 128) / 128;
        land = lerp_i32_fixed(land, water_bed, terrain.water_mask);
    }

    lerp_i32_fixed(ocean, coast.max(land), shelf).clamp(min_surface as i32, max_surface as i32)
        as i16
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
        if moisture > 224
            && value_noise2(&global_config.world_seed, world_x, world_z, 72, 109) > 168
        {
            return BLOCK_MOSS;
        }
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
    for cell_size in [7_i32, 9_i32] {
        let min_cell_x = tree_candidate_min_cell(world_x, TREE_MAX_LEAF_RADIUS, cell_size);
        let max_cell_x = tree_candidate_max_cell(world_x, TREE_MAX_LEAF_RADIUS, cell_size);
        let min_cell_z = tree_candidate_min_cell(world_z, TREE_MAX_LEAF_RADIUS, cell_size);
        let max_cell_z = tree_candidate_max_cell(world_z, TREE_MAX_LEAF_RADIUS, cell_size);
        let inner = (cell_size - 2).max(1) as u32;

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

                let wet = generated_wet_at(global_config, tree_x, tree_z);
                if (if wet { 7 } else { 9 }) != cell_size {
                    continue;
                }
                let density = if wet { 180 } else { 218 };
                if hash_coord3_from_base(tree_roll_hash_base, cell_x, 0, cell_z) & 255 <= density {
                    continue;
                }

                let surface = generated_surface_height(global_config, tree_x, tree_z);
                if !tree_vertical_bounds_can_contain(surface, y) {
                    continue;
                }
                if !generated_can_grow_tree(global_config, tree_x, tree_z, surface) {
                    continue;
                }
                let tree = generated_tree_from_candidate(global_config, tree_x, tree_z, surface);
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

#[allow(dead_code)]
fn generated_tree_at(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> GeneratedTree {
    let wet = generated_wet_at(global_config, world_x, world_z);
    let density = if wet { 180 } else { 218 };
    let cell_size = if wet { 7 } else { 9 };
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
    let tree = generated_tree_from_candidate(global_config, world_x, world_z, surface);
    GeneratedTree {
        exists: world_x == tree_x && world_z == tree_z && roll > density,
        ..tree
    }
}

fn generated_tree_from_candidate(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> GeneratedTree {
    let pine = generated_cold_at(global_config, world_x, world_z, surface)
        || surface >= global_config.sea_level.saturating_add(32)
        || (hash_coord3(
            &global_config.world_seed,
            world_x,
            surface as i32,
            world_z,
            404,
        ) & 255)
            > 206;
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
    surface >= global_config.sea_level.saturating_add(30)
        || (surface >= global_config.sea_level.saturating_add(18)
            && value_noise2(&global_config.world_seed, world_x, world_z, 160, 201) < 42)
}

fn generated_desert_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> bool {
    generated_desert_score_at(global_config, world_x, world_z) > 178
}

fn generated_wet_at(global_config: &GlobalConfigView, world_x: i32, world_z: i32) -> bool {
    generated_moisture_at(global_config, world_x, world_z) > 188
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
    let river = scale_by_fixed(smooth_range_fixed(river_line, 118, 128) as i32, inland);
    let lake = scale_by_fixed(
        smooth_range_fixed(
            value_noise2(&global_config.world_seed, wx, wz, 220, 37) as i32,
            210,
            242,
        ) as i32,
        inland,
    );
    GeneratedTerrainFactors {
        wx,
        wz,
        shelf,
        inland,
        water_mask: river.max(lake) as u32,
    }
}

fn generated_water_level(
    global_config: &GlobalConfigView,
    world_x: i32,
    world_z: i32,
    surface: i16,
) -> Option<i16> {
    if surface < global_config.sea_level {
        return Some(global_config.sea_level);
    }
    let terrain = generated_terrain_factors(global_config, world_x, world_z);
    if terrain.water_mask <= 96 {
        return None;
    }
    Some(generated_inland_water_level(
        global_config,
        terrain.wx,
        terrain.wz,
    ))
}

fn generated_inland_water_level(global_config: &GlobalConfigView, wx: i32, wz: i32) -> i16 {
    (global_config.sea_level as i32
        + 6
        + (value_noise2(&global_config.world_seed, wx, wz, 180, 41) as i32 - 128) / 128) as i16
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

pub fn unpack_resource_drop_rules(data: &[u8]) -> Result<Vec<ResourceDropRule>, NicechunkChunkError> {
    let rule_count = ResourceDropTableState::validate_header(data)?;
    let mut rules = Vec::with_capacity(rule_count);
    for index in 0..rule_count {
        let offset = RESOURCE_DROP_TABLE_HEADER_LEN + index * RESOURCE_DROP_RULE_LEN;
        rules.push(ResourceDropRule::unpack(
            &data[offset..offset + RESOURCE_DROP_RULE_LEN],
        )?);
    }
    Ok(rules)
}

pub fn extra_drop_block_id_at(
    global_config: &GlobalConfigView,
    rules: &[ResourceDropRule],
    world_x: i32,
    world_y: i16,
    world_z: i32,
    source_block_id: u16,
) -> Option<u16> {
    let surface = generated_surface_height(global_config, world_x, world_z);
    let altitude = world_y.saturating_sub(global_config.sea_level);
    let depth = surface.saturating_sub(world_y);
    for rule in rules {
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
        if roll < rule.chance_bps as u32 {
            return Some(rule.drop_block_id);
        }
    }
    None
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
            world_seed: [7_u8; 32],
            chunk_size: 16,
            min_build_y: -32,
            max_build_y: 256,
            max_terrain_height: 160,
            sea_level: 2,
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
    fn canonical_resource_vectors_match_frontend() {
        let config = test_global_config_view();
        let cases = [
            (1, 0, 2, -11, BLOCK_WATER),
            (1, -31, 2, -11, BLOCK_DEEP_STONE),
            (12, 0, -34, -13, BLOCK_WATER),
            (40, 20, 40, -13, BLOCK_AIR),
            (-7, 5, 18, -12, BLOCK_AIR),
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
        for world_x in -8..8 {
            for world_z in -8..8 {
                for y in -2..6 {
                    assert_eq!(
                        generated_tree_block_id_at(&config, world_x, y, world_z),
                        slow_generated_tree_block_id_at(&config, world_x, y, world_z),
                        "tree query mismatch at {world_x},{y},{world_z}"
                    );
                }
            }
        }
    }

    #[test]
    fn tree_candidate_cell_range_covers_nearby_scan_roots() {
        let config = test_global_config_view();
        let mut checked_candidates = 0;
        for world_coord in -128..128 {
            for cell_size in [7_i32, 9_i32] {
                let min_cell =
                    tree_candidate_min_cell(world_coord, TREE_MAX_LEAF_RADIUS, cell_size);
                let max_cell =
                    tree_candidate_max_cell(world_coord, TREE_MAX_LEAF_RADIUS, cell_size);
                let inner = (cell_size - 2).max(1) as u32;
                for candidate_coord in
                    world_coord - TREE_MAX_LEAF_RADIUS..=world_coord + TREE_MAX_LEAF_RADIUS
                {
                    let cell = div_floor_i32(candidate_coord, cell_size);
                    let tree_coord = cell
                        .saturating_mul(cell_size)
                        .saturating_add(1)
                        .saturating_add(
                            (hash_coord3(&config.world_seed, cell, 0, cell, 401) % inner) as i32,
                        );
                    if tree_coord == candidate_coord {
                        checked_candidates += 1;
                        assert!(
                            cell >= min_cell && cell <= max_cell,
                            "candidate cell {cell} outside optimized range {min_cell}..={max_cell}"
                        );
                    }
                }
            }
        }
        assert!(checked_candidates > 0);
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
        let y = config.min_build_y + 6;
        let mut found = false;
        'outer: for world_x in -128..128 {
            for world_z in -128..128 {
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
