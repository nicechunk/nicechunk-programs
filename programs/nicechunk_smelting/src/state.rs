use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkSmeltingError;

pub const RECIPE_TABLE_MAGIC: [u8; 8] = *b"NCKSMR01";
pub const RECIPE_TABLE_VERSION: u16 = 2;
pub const RECIPE_TABLE_SEED: &[u8] = b"smelting-recipes";
pub const SMELTING_AUTHORITY_SEED: &[u8] = b"smelting-authority";
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
pub const RECIPE_TABLE_HEADER_LEN: usize = 96;
pub const RECIPE_TABLE_MAX_RECIPES: usize = 10;
pub const RECIPE_MAX_INPUTS: usize = 8;
pub const RECIPE_MAX_OUTPUTS: usize = 4;
pub const BACKPACK_RESOURCE_RECORD_LEN: usize = 10;
pub const BACKPACK_SLOT_RECORD_LEN: usize = 80;
pub const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
pub const BACKPACK_SLOT_KIND_ITEM: u8 = 2;
pub const BACKPACK_ITEM_CATEGORY_MATERIAL: u8 = 1;
pub const DEFAULT_RESOURCE_VOLUME_MM3: u32 = 1_000_000;
pub const RECIPE_YIELD_BPS_DENOMINATOR: u16 = 10_000;
pub const SMELTING_SKILL_BASE_OUTPUT_BPS: u16 = 10_000;
pub const SMELTING_SKILL_OUTPUT_BPS_PER_LEVEL: u16 = 500;
pub const SMELTING_SKILL_MAX_OUTPUT_BPS: u16 = 15_000;
pub const SMELTING_XP_PER_ACTION: u64 = 1;
pub const DURABILITY_BPS_DENOMINATOR: u64 = 10_000;
const MATERIAL_MERGE_RECIPE_ID_OFFSET: u64 = 1_000;
const BACKPACK_PACKED_Y_BITS: u16 = 9;
pub const RECIPE_RECORD_LEN: usize = 8
    + 1
    + 1
    + 1
    + 1
    + 2
    + 2
    + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN
    + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN
    + 8;
pub const RECIPE_TABLE_LEN: usize =
    RECIPE_TABLE_HEADER_LEN + RECIPE_TABLE_MAX_RECIPES * RECIPE_RECORD_LEN;
pub const UPSERT_RECIPE_ARGS_LEN: usize = 8
    + 1
    + 1
    + 1
    + 1
    + 2
    + 2
    + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN
    + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN;

const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
const BACKPACK_VERSION: u16 = 4;
const BACKPACK_HEADER_LEN: usize = 128;
pub const BACKPACK_MAX_CAPACITY: usize = 99;
const BACKPACK_LEN: usize = BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY * BACKPACK_SLOT_RECORD_LEN;
const BACKPACK_OWNER_OFFSET: usize = 20;
const BACKPACK_CAPACITY_OFFSET: usize = 52;
const BACKPACK_ITEM_COUNT_OFFSET: usize = 53;
const BACKPACK_RECORDS_OFFSET: usize = BACKPACK_HEADER_LEN;

pub struct RecipeTableInitArgs<'a> {
    pub bump: u8,
    pub table_id: u64,
    pub authority: &'a Pubkey,
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct PlayerProgressInitArgs<'a> {
    pub bump: u8,
    pub owner: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub created_slot: u64,
    pub created_at: i64,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct BackpackResourceRecord {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
}

impl BackpackResourceRecord {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != BACKPACK_RESOURCE_RECORD_LEN {
            return Err(NicechunkSmeltingError::InvalidInstruction);
        }
        Ok(Self {
            world_x: read_i32(data, 0),
            world_y: read_i16(data, 4),
            world_z: read_i32(data, 6),
        })
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != BACKPACK_RESOURCE_RECORD_LEN {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
        }
        dst[0..4].copy_from_slice(&self.world_x.to_le_bytes());
        dst[4..6].copy_from_slice(&self.world_y.to_le_bytes());
        dst[6..10].copy_from_slice(&self.world_z.to_le_bytes());
        Ok(())
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct BackpackSlotRecord {
    pub kind: u8,
    pub category: u8,
    pub flags: u16,
    pub quantity: u32,
    pub resource: BackpackResourceRecord,
    pub item_code: u16,
    pub item_id: u64,
    pub item_pda: Pubkey,
    pub volume_mm3: u32,
    pub durability_current: u32,
    pub durability_max: u32,
    pub grade: u8,
    pub item_level: u8,
    pub quality_bps: u16,
    pub metadata: u32,
}

impl BackpackSlotRecord {
    pub fn from_block_resource(resource: BackpackResourceRecord) -> Self {
        Self {
            kind: BACKPACK_SLOT_KIND_BLOCK,
            category: 0,
            flags: 0,
            quantity: 1,
            resource,
            item_code: 0,
            item_id: 0,
            item_pda: Pubkey::default(),
            volume_mm3: 0,
            durability_current: 0,
            durability_max: 0,
            grade: 0,
            item_level: 0,
            quality_bps: 0,
            metadata: 0,
        }
    }

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != BACKPACK_SLOT_RECORD_LEN {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let kind = data[0];
        if kind != BACKPACK_SLOT_KIND_BLOCK && kind != BACKPACK_SLOT_KIND_ITEM {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let record = Self {
            kind,
            category: data[1],
            flags: read_u16(data, 2),
            quantity: read_u32(data, 4),
            resource: BackpackResourceRecord::unpack(&data[8..18])?,
            item_code: read_u16(data, 18),
            item_id: read_u64(data, 20),
            item_pda: Pubkey::new_from_array(
                data[28..60]
                    .try_into()
                    .map_err(|_| NicechunkSmeltingError::InvalidRecipe)?,
            ),
            volume_mm3: read_u32(data, 60),
            durability_current: read_u32(data, 64),
            durability_max: read_u32(data, 68),
            grade: data[72],
            item_level: data[73],
            quality_bps: read_u16(data, 74),
            metadata: read_u32(data, 76),
        };
        if record.quantity == 0 {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        if record.kind == BACKPACK_SLOT_KIND_ITEM && (record.category == 0 || record.item_code == 0)
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        Ok(record)
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != BACKPACK_SLOT_RECORD_LEN {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
        }
        if self.kind != BACKPACK_SLOT_KIND_BLOCK && self.kind != BACKPACK_SLOT_KIND_ITEM {
            return Err(NicechunkSmeltingError::InvalidRecipe.into());
        }
        if self.quantity == 0 {
            return Err(NicechunkSmeltingError::InvalidRecipe.into());
        }
        if self.kind == BACKPACK_SLOT_KIND_ITEM && (self.category == 0 || self.item_code == 0) {
            return Err(NicechunkSmeltingError::InvalidRecipe.into());
        }
        dst.fill(0);
        dst[0] = self.kind;
        dst[1] = self.category;
        dst[2..4].copy_from_slice(&self.flags.to_le_bytes());
        dst[4..8].copy_from_slice(&self.quantity.to_le_bytes());
        self.resource.pack(&mut dst[8..18])?;
        dst[18..20].copy_from_slice(&self.item_code.to_le_bytes());
        dst[20..28].copy_from_slice(&self.item_id.to_le_bytes());
        dst[28..60].copy_from_slice(self.item_pda.as_ref());
        dst[60..64].copy_from_slice(&self.volume_mm3.to_le_bytes());
        dst[64..68].copy_from_slice(&self.durability_current.to_le_bytes());
        dst[68..72].copy_from_slice(&self.durability_max.to_le_bytes());
        dst[72] = self.grade;
        dst[73] = self.item_level;
        dst[74..76].copy_from_slice(&self.quality_bps.to_le_bytes());
        dst[76..80].copy_from_slice(&self.metadata.to_le_bytes());
        Ok(())
    }

    pub fn validate_output_item(&self) -> Result<(), NicechunkSmeltingError> {
        if self.kind == BACKPACK_SLOT_KIND_ITEM
            && (self.item_id == 0
                || self.item_pda == Pubkey::default()
                || self.volume_mm3 == 0
                || self.durability_current == 0
                || self.durability_max == 0
                || self.durability_current > self.durability_max
                || self.grade == 0
                || self.grade > 10
                || self.item_level == 0
                || self.item_level > 100
                || self.quality_bps == 0
                || self.quality_bps as u64 > DURABILITY_BPS_DENOMINATOR)
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        Ok(())
    }

    pub fn validate_recipe_output(
        &self,
        recipe_table: &Pubkey,
    ) -> Result<(), NicechunkSmeltingError> {
        self.validate_output_item()?;
        if self.kind != BACKPACK_SLOT_KIND_ITEM
            || self.category != BACKPACK_ITEM_CATEGORY_MATERIAL
            || self.item_pda != *recipe_table
        {
            return Err(NicechunkSmeltingError::InvalidRecipeOutput);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct RecipeRecord {
    pub recipe_id: u64,
    pub enabled: bool,
    pub min_heat_tier: u8,
    pub input_count: u8,
    pub output_count: u8,
    pub yield_bps: u16,
    pub inputs: [BackpackSlotRecord; RECIPE_MAX_INPUTS],
    pub outputs: [BackpackSlotRecord; RECIPE_MAX_OUTPUTS],
    pub updated_slot: u64,
}

impl Default for RecipeRecord {
    fn default() -> Self {
        Self {
            recipe_id: 0,
            enabled: false,
            min_heat_tier: 0,
            input_count: 0,
            output_count: 0,
            yield_bps: RECIPE_YIELD_BPS_DENOMINATOR,
            inputs: [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS],
            outputs: [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS],
            updated_slot: 0,
        }
    }
}

impl RecipeRecord {
    pub fn unpack_civilization_patch_args(
        data: &[u8],
        updated_slot: u64,
    ) -> Result<Self, NicechunkSmeltingError> {
        if data.len() < 16 {
            return Err(NicechunkSmeltingError::InvalidInstruction);
        }
        let recipe_id = read_u64(data, 0);
        let enabled = data[8] == 1;
        let min_heat_tier = data[9];
        let input_count = data[10];
        let output_count = data[11];
        let yield_bps = read_u16(data, 12);
        let expected_len = 16_usize
            .saturating_add(input_count as usize * BACKPACK_SLOT_RECORD_LEN)
            .saturating_add(output_count as usize * BACKPACK_SLOT_RECORD_LEN);
        if data.len() != expected_len
            || recipe_id == 0
            || input_count == 0
            || input_count as usize > RECIPE_MAX_INPUTS
            || output_count == 0
            || output_count as usize > RECIPE_MAX_OUTPUTS
            || yield_bps == 0
            || yield_bps > RECIPE_YIELD_BPS_DENOMINATOR
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let mut inputs = [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS];
        let mut outputs = [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS];
        let mut offset = 16;
        for input in inputs.iter_mut().take(input_count as usize) {
            *input = BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for output in outputs.iter_mut().take(output_count as usize) {
            *output = BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            output.validate_output_item()?;
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for index in input_count as usize..RECIPE_MAX_INPUTS {
            inputs[index] = inputs[0];
        }
        for index in output_count as usize..RECIPE_MAX_OUTPUTS {
            outputs[index] = outputs[0];
        }
        Ok(Self {
            recipe_id,
            enabled,
            min_heat_tier,
            input_count,
            output_count,
            yield_bps,
            inputs,
            outputs,
            updated_slot,
        })
    }

    pub fn unpack_args(data: &[u8], updated_slot: u64) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != UPSERT_RECIPE_ARGS_LEN {
            return Err(NicechunkSmeltingError::InvalidInstruction);
        }
        let recipe_id = read_u64(data, 0);
        let enabled = data[8] == 1;
        let min_heat_tier = data[9];
        let input_count = data[10];
        let output_count = data[11];
        let yield_bps = read_u16(data, 12);
        if recipe_id == 0
            || input_count == 0
            || input_count as usize > RECIPE_MAX_INPUTS
            || output_count == 0
            || output_count as usize > RECIPE_MAX_OUTPUTS
            || yield_bps == 0
            || yield_bps > RECIPE_YIELD_BPS_DENOMINATOR
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let mut inputs = [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS];
        let mut outputs = [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS];
        let mut offset = 16;
        for (index, input) in inputs.iter_mut().enumerate() {
            if index < input_count as usize {
                *input =
                    BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            }
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for (index, output) in outputs.iter_mut().enumerate() {
            if index < output_count as usize {
                *output =
                    BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
                output.validate_output_item()?;
            }
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for index in input_count as usize..RECIPE_MAX_INPUTS {
            inputs[index] = inputs[0];
        }
        for index in output_count as usize..RECIPE_MAX_OUTPUTS {
            outputs[index] = outputs[0];
        }
        Ok(Self {
            recipe_id,
            enabled,
            min_heat_tier,
            input_count,
            output_count,
            yield_bps,
            inputs,
            outputs,
            updated_slot,
        })
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != RECIPE_RECORD_LEN {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
        }
        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.u64(self.recipe_id)?;
        writer.u8(if self.enabled { 1 } else { 0 })?;
        writer.u8(self.min_heat_tier)?;
        writer.u8(self.input_count)?;
        writer.u8(self.output_count)?;
        writer.u16(self.yield_bps)?;
        writer.u16(0)?;
        for input in self.inputs.iter() {
            input.pack(&mut writer.dst[writer.offset..writer.offset + BACKPACK_SLOT_RECORD_LEN])?;
            writer.offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for output in self.outputs.iter() {
            output
                .pack(&mut writer.dst[writer.offset..writer.offset + BACKPACK_SLOT_RECORD_LEN])?;
            writer.offset += BACKPACK_SLOT_RECORD_LEN;
        }
        writer.u64(self.updated_slot)?;
        if writer.offset != RECIPE_RECORD_LEN {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_outputs_for_table(&self, recipe_table: &Pubkey) -> ProgramResult {
        if self.output_count == 0 || self.output_count as usize > RECIPE_MAX_OUTPUTS {
            return Err(NicechunkSmeltingError::InvalidRecipe.into());
        }
        for output in self.outputs.iter().take(self.output_count as usize) {
            output.validate_recipe_output(recipe_table)?;
        }
        Ok(())
    }

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != RECIPE_RECORD_LEN {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let recipe_id = read_u64(data, 0);
        let enabled = data[8] == 1;
        let min_heat_tier = data[9];
        let input_count = data[10];
        let output_count = data[11];
        let yield_bps = read_u16(data, 12);
        if recipe_id == 0 {
            return Ok(Self::default());
        }
        if input_count == 0
            || input_count as usize > RECIPE_MAX_INPUTS
            || output_count == 0
            || output_count as usize > RECIPE_MAX_OUTPUTS
            || yield_bps == 0
            || yield_bps > RECIPE_YIELD_BPS_DENOMINATOR
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let mut inputs = [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS];
        let mut outputs = [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS];
        let mut offset = 16;
        for input in inputs.iter_mut() {
            *input = BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for output in outputs.iter_mut() {
            *output = BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        Ok(Self {
            recipe_id,
            enabled,
            min_heat_tier,
            input_count,
            output_count,
            yield_bps,
            inputs,
            outputs,
            updated_slot: read_u64(data, offset),
        })
    }
}

pub struct PlayerProgressState {
    pub smelting_xp: u64,
}

impl PlayerProgressState {
    pub fn pack_empty(dst: &mut [u8], args: &PlayerProgressInitArgs) -> ProgramResult {
        if dst.len() != PLAYER_PROGRESS_LEN {
            return Err(NicechunkSmeltingError::InvalidPlayerProgressData.into());
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
    ) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != PLAYER_PROGRESS_LEN
            || data[0..8] != PLAYER_PROGRESS_MAGIC
            || read_u16(data, 8) != PLAYER_PROGRESS_VERSION
            || data[11] != 1
        {
            return Err(NicechunkSmeltingError::InvalidPlayerProgressData);
        }
        if &data[PLAYER_PROGRESS_OWNER_OFFSET..PLAYER_PROGRESS_OWNER_OFFSET + 32] != owner.as_ref()
        {
            return Err(NicechunkSmeltingError::InvalidPlayerProgress);
        }
        if &data[PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET..PLAYER_PROGRESS_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkSmeltingError::InvalidPlayerProgress);
        }
        Ok(Self {
            smelting_xp: read_u64(data, PLAYER_PROGRESS_SMELTING_XP_OFFSET),
        })
    }

    pub fn smelting_output_bps_from_level(level: u8) -> u16 {
        let level = u16::from(level.min(10));
        SMELTING_SKILL_BASE_OUTPUT_BPS
            .saturating_add(level.saturating_mul(SMELTING_SKILL_OUTPUT_BPS_PER_LEVEL))
            .min(SMELTING_SKILL_MAX_OUTPUT_BPS)
    }

    pub fn add_smelting_xp(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        gained_xp: u64,
        updated_slot: u64,
    ) -> ProgramResult {
        let state = Self::validate(data, owner, global_config)?;
        let next_xp = state.smelting_xp.saturating_add(gained_xp);
        data[PLAYER_PROGRESS_SMELTING_XP_OFFSET..PLAYER_PROGRESS_SMELTING_XP_OFFSET + 8]
            .copy_from_slice(&next_xp.to_le_bytes());
        data[PLAYER_PROGRESS_UPDATED_SLOT_OFFSET..PLAYER_PROGRESS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

pub struct RecipeTable;

impl RecipeTable {
    pub const LEN: usize = RECIPE_TABLE_LEN;
    pub const BUMP_OFFSET: usize = 10;
    pub const TABLE_ID_OFFSET: usize = 12;
    pub const AUTHORITY_OFFSET: usize = 20;
    pub const RECIPE_COUNT_OFFSET: usize = 52;
    pub const UPDATED_SLOT_OFFSET: usize = 62;
    pub const RECORDS_OFFSET: usize = RECIPE_TABLE_HEADER_LEN;

    pub fn pack_empty(dst: &mut [u8], args: &RecipeTableInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkSmeltingError::InvalidRecipeTableData.into());
        }
        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&RECIPE_TABLE_MAGIC)?;
        writer.u16(RECIPE_TABLE_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.u64(args.table_id)?;
        writer.pubkey(args.authority)?;
        writer.u16(0)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.bytes(&[0_u8; 18])?;
        if writer.offset != RECIPE_TABLE_HEADER_LEN {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate(data: &[u8]) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != RECIPE_TABLE_MAGIC {
            return Err(NicechunkSmeltingError::InvalidRecipeTableData.into());
        }
        if read_u16(data, 8) != RECIPE_TABLE_VERSION || data[11] != 1 {
            return Err(NicechunkSmeltingError::InvalidRecipeTableData.into());
        }
        if read_u16(data, Self::RECIPE_COUNT_OFFSET) as usize > RECIPE_TABLE_MAX_RECIPES {
            return Err(NicechunkSmeltingError::InvalidRecipeTableData.into());
        }
        Ok(())
    }

    pub fn authority(data: &[u8]) -> Result<Pubkey, NicechunkSmeltingError> {
        if data.len() != Self::LEN {
            return Err(NicechunkSmeltingError::InvalidRecipeTableData);
        }
        Ok(Pubkey::new_from_array(
            data[Self::AUTHORITY_OFFSET..Self::AUTHORITY_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkSmeltingError::InvalidRecipeTableData)?,
        ))
    }

    pub fn table_id(data: &[u8]) -> Result<u64, NicechunkSmeltingError> {
        Self::validate(data).map_err(|_| NicechunkSmeltingError::InvalidRecipeTableData)?;
        Ok(read_u64(data, Self::TABLE_ID_OFFSET))
    }

    pub fn validate_identity(
        data: &[u8],
        program_id: &Pubkey,
        recipe_table: &Pubkey,
        authority: &Pubkey,
    ) -> Result<u64, NicechunkSmeltingError> {
        Self::validate(data).map_err(|_| NicechunkSmeltingError::InvalidRecipeTableData)?;
        if Self::authority(data)? != *authority {
            return Err(NicechunkSmeltingError::UnauthorizedAuthority);
        }
        let table_id = Self::table_id(data)?;
        let table_id_bytes = table_id.to_le_bytes();
        let (expected_table, expected_bump) =
            Pubkey::find_program_address(&[RECIPE_TABLE_SEED, &table_id_bytes], program_id);
        if *recipe_table != expected_table || data[Self::BUMP_OFFSET] != expected_bump {
            return Err(NicechunkSmeltingError::InvalidRecipeTablePda);
        }
        Ok(table_id)
    }

    pub fn upsert_recipe(
        data: &mut [u8],
        recipe: &RecipeRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate(data)?;
        let mut empty_slot: Option<usize> = None;
        for index in 0..RECIPE_TABLE_MAX_RECIPES {
            let offset = Self::RECORDS_OFFSET + index * RECIPE_RECORD_LEN;
            let existing = RecipeRecord::unpack(&data[offset..offset + RECIPE_RECORD_LEN])?;
            if existing.recipe_id == recipe.recipe_id {
                recipe.pack(&mut data[offset..offset + RECIPE_RECORD_LEN])?;
                data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
                    .copy_from_slice(&updated_slot.to_le_bytes());
                return Ok(());
            }
            if existing.recipe_id == 0 && empty_slot.is_none() {
                empty_slot = Some(index);
            }
        }
        let index = empty_slot.ok_or(NicechunkSmeltingError::RecipeTableFull)?;
        let offset = Self::RECORDS_OFFSET + index * RECIPE_RECORD_LEN;
        recipe.pack(&mut data[offset..offset + RECIPE_RECORD_LEN])?;
        let count = read_u16(data, Self::RECIPE_COUNT_OFFSET).saturating_add(1);
        data[Self::RECIPE_COUNT_OFFSET..Self::RECIPE_COUNT_OFFSET + 2]
            .copy_from_slice(&count.to_le_bytes());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn find_recipe(
        data: &[u8],
        recipe_id: u64,
    ) -> Result<RecipeRecord, NicechunkSmeltingError> {
        Self::validate(data).map_err(|_| NicechunkSmeltingError::InvalidRecipeTableData)?;
        for index in 0..RECIPE_TABLE_MAX_RECIPES {
            let offset = Self::RECORDS_OFFSET + index * RECIPE_RECORD_LEN;
            let recipe = RecipeRecord::unpack(&data[offset..offset + RECIPE_RECORD_LEN])?;
            if recipe.recipe_id == recipe_id && recipe.enabled {
                return Ok(recipe);
            }
        }
        Err(NicechunkSmeltingError::RecipeNotFound)
    }
}

pub struct BackpackAccountView;

#[derive(Debug)]
pub struct ValidatedRecipeInputs {
    pub input_volume_mm3: u64,
    pub consumed_input_units: u64,
    pub consumption_quantities: [u32; BACKPACK_MAX_CAPACITY],
}

impl BackpackAccountView {
    pub fn validate(data: &[u8]) -> ProgramResult {
        if data.len() != BACKPACK_LEN || data[0..8] != BACKPACK_MAGIC {
            return Err(NicechunkSmeltingError::InvalidBackpackData.into());
        }
        let version = read_u16(data, 8);
        if version != BACKPACK_VERSION || data[11] != 1 {
            return Err(NicechunkSmeltingError::InvalidBackpackData.into());
        }
        let capacity = data[BACKPACK_CAPACITY_OFFSET] as usize;
        let item_count = data[BACKPACK_ITEM_COUNT_OFFSET] as usize;
        if capacity == 0 || capacity > BACKPACK_MAX_CAPACITY || item_count > capacity {
            return Err(NicechunkSmeltingError::InvalidBackpackData.into());
        }
        Ok(())
    }

    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        Self::validate(data)?;
        if &data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkSmeltingError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn validate_recipe_inputs(
        data: &[u8],
        owner: &Pubkey,
        indexes: &[u8],
        fuel_indexes: &[u8],
        recipe: &RecipeRecord,
        multiplier: u16,
    ) -> Result<ValidatedRecipeInputs, solana_program::program_error::ProgramError> {
        Self::validate_owner(data, owner)?;
        if multiplier == 0 || indexes.is_empty() || indexes.len() > BACKPACK_MAX_CAPACITY {
            return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
        }
        let capacity = data[BACKPACK_CAPACITY_OFFSET] as usize;
        let item_count = data[BACKPACK_ITEM_COUNT_OFFSET] as usize;

        let mut seen_indexes = [false; BACKPACK_MAX_CAPACITY];
        let mut matched_inputs = [0_u64; RECIPE_MAX_INPUTS];
        let mut consumption_quantities = [0_u32; BACKPACK_MAX_CAPACITY];
        let mut input_volume_mm3 = 0_u64;
        let mut consumed_input_units = 0_u64;
        let merge_recipe = recipe_is_material_merge(recipe);
        if merge_recipe
            && (!fuel_indexes.is_empty()
                || indexes.len() < 2
                || multiplier as usize != indexes.len())
        {
            return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
        }
        for index in indexes {
            let selected = *index as usize;
            if selected >= item_count || seen_indexes[selected] {
                return Err(NicechunkSmeltingError::InvalidInputIndex.into());
            }
            seen_indexes[selected] = true;
            let record = Self::slot_at(data, *index)?;
            if merge_recipe {
                if !recipe_input_matches(&recipe.inputs[0], &record) {
                    return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
                }
                consumption_quantities[selected] = record.quantity;
                consumed_input_units = consumed_input_units.saturating_add(record.quantity as u64);
                input_volume_mm3 = input_volume_mm3.saturating_add(slot_volume_mm3(&record) as u64);
                continue;
            }

            let mut remaining = record.quantity as u64;
            for recipe_index in 0..recipe.input_count as usize {
                let required =
                    (recipe.inputs[recipe_index].quantity as u64).saturating_mul(multiplier as u64);
                if remaining == 0
                    || matched_inputs[recipe_index] >= required
                    || !recipe_input_matches(&recipe.inputs[recipe_index], &record)
                {
                    continue;
                }
                let consumed = remaining.min(required.saturating_sub(matched_inputs[recipe_index]));
                matched_inputs[recipe_index] =
                    matched_inputs[recipe_index].saturating_add(consumed);
                consumption_quantities[selected] = consumption_quantities[selected]
                    .saturating_add(consumed.min(u32::MAX as u64) as u32);
                remaining = remaining.saturating_sub(consumed);
            }
            let consumed = consumption_quantities[selected];
            if consumed == 0 {
                return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
            }
            consumed_input_units = consumed_input_units.saturating_add(consumed as u64);
            input_volume_mm3 = input_volume_mm3
                .saturating_add(proportional_consumed_volume_mm3(&record, consumed)? as u64);
        }
        if !merge_recipe {
            for (recipe_index, matched) in matched_inputs
                .iter()
                .enumerate()
                .take(recipe.input_count as usize)
            {
                let required =
                    (recipe.inputs[recipe_index].quantity as u64).saturating_mul(multiplier as u64);
                if *matched != required {
                    return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
                }
            }
        } else if consumed_input_units > u32::MAX as u64 || input_volume_mm3 > u32::MAX as u64 {
            return Err(NicechunkSmeltingError::OutputOverflow.into());
        }
        let mut max_fuel_tier = 0_u8;
        for index in fuel_indexes {
            let selected = *index as usize;
            if selected >= item_count || seen_indexes[selected] {
                return Err(NicechunkSmeltingError::InvalidInputIndex.into());
            }
            seen_indexes[selected] = true;
            max_fuel_tier = max_fuel_tier.max(fuel_heat_tier(&Self::slot_at(data, *index)?));
        }
        if max_fuel_tier < recipe.min_heat_tier {
            return Err(NicechunkSmeltingError::FuelHeatTooLow.into());
        }
        let fully_removed_inputs = indexes
            .iter()
            .filter(|index| {
                Self::slot_at(data, **index)
                    .map(|record| consumption_quantities[**index as usize] == record.quantity)
                    .unwrap_or(false)
            })
            .count();
        let fully_removed_fuels = fuel_indexes
            .iter()
            .filter(|index| {
                Self::slot_at(data, **index)
                    .map(|record| record.quantity == 1)
                    .unwrap_or(false)
            })
            .count();
        let remove_count = fully_removed_inputs.saturating_add(fully_removed_fuels);
        if item_count
            .saturating_sub(remove_count)
            .saturating_add(recipe.output_count as usize)
            > capacity
        {
            return Err(NicechunkSmeltingError::BackpackCapacityExceeded.into());
        }
        Ok(ValidatedRecipeInputs {
            input_volume_mm3: input_volume_mm3.max(1),
            consumed_input_units,
            consumption_quantities,
        })
    }

    fn slot_at(data: &[u8], index: u8) -> Result<BackpackSlotRecord, NicechunkSmeltingError> {
        let offset = BACKPACK_RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])
    }
}

pub(crate) fn recipe_is_material_merge(recipe: &RecipeRecord) -> bool {
    if recipe.input_count != 1
        || recipe.output_count != 1
        || recipe.min_heat_tier != 0
        || recipe.yield_bps != RECIPE_YIELD_BPS_DENOMINATOR
    {
        return false;
    }
    let input = &recipe.inputs[0];
    let output = &recipe.outputs[0];
    input.item_code > 0
        && recipe.recipe_id
            == u64::from(input.item_code).saturating_add(MATERIAL_MERGE_RECIPE_ID_OFFSET)
        && input.kind == BACKPACK_SLOT_KIND_ITEM
        && output.kind == BACKPACK_SLOT_KIND_ITEM
        && input.category == BACKPACK_ITEM_CATEGORY_MATERIAL
        && output.category == BACKPACK_ITEM_CATEGORY_MATERIAL
        && input.item_code == output.item_code
        && output.item_id == u64::from(output.item_code)
        && input.quantity == 1
        && output.quantity == 1
        && input.volume_mm3 > 0
        && input.volume_mm3 == output.volume_mm3
}

fn proportional_consumed_volume_mm3(
    record: &BackpackSlotRecord,
    consumed_quantity: u32,
) -> Result<u32, NicechunkSmeltingError> {
    if consumed_quantity == 0 || consumed_quantity > record.quantity {
        return Err(NicechunkSmeltingError::InputRecipeMismatch);
    }
    let total_volume = slot_volume_mm3(record);
    if consumed_quantity == record.quantity {
        return Ok(total_volume);
    }
    if total_volume <= 1 {
        return Err(NicechunkSmeltingError::InputRecipeMismatch);
    }
    let proportional = (total_volume as u64)
        .saturating_mul(consumed_quantity as u64)
        .saturating_div(record.quantity as u64);
    Ok(proportional
        .max(1)
        .min(total_volume.saturating_sub(1) as u64) as u32)
}

fn recipe_input_matches(expected: &BackpackSlotRecord, actual: &BackpackSlotRecord) -> bool {
    if expected.kind != actual.kind {
        return false;
    }
    match expected.kind {
        BACKPACK_SLOT_KIND_BLOCK => {
            packed_block_id(expected.resource.world_y) == packed_block_id(actual.resource.world_y)
        }
        BACKPACK_SLOT_KIND_ITEM => {
            expected.category == actual.category
                && expected.item_code == actual.item_code
                && (expected.item_id == 0 || expected.item_id == actual.item_id)
        }
        _ => false,
    }
}

fn fuel_heat_tier(slot: &BackpackSlotRecord) -> u8 {
    if slot.kind == BACKPACK_SLOT_KIND_ITEM
        && slot.category == BACKPACK_ITEM_CATEGORY_MATERIAL
        && slot.item_code == 1001
    {
        return 3;
    }
    if slot.kind != BACKPACK_SLOT_KIND_BLOCK {
        return 0;
    }
    match packed_block_id(slot.resource.world_y) {
        47 => 4,                // coal
        22 | 24 | 26 | 27 => 2, // wood-like fuels
        29 | 31 | 36 => 1,      // dry grass / dead bush / thorn
        _ => 0,
    }
}

fn slot_volume_mm3(slot: &BackpackSlotRecord) -> u32 {
    if slot.volume_mm3 > 0 {
        slot.volume_mm3
    } else {
        DEFAULT_RESOURCE_VOLUME_MM3
    }
}

fn packed_block_id(packed_y: i16) -> u16 {
    if packed_y < 0 {
        return 0;
    }
    (packed_y as u16) >> BACKPACK_PACKED_Y_BITS
}

struct ByteWriter<'a> {
    dst: &'a mut [u8],
    offset: usize,
}

impl ByteWriter<'_> {
    fn bytes(&mut self, bytes: &[u8]) -> ProgramResult {
        let end = self.offset + bytes.len();
        if end > self.dst.len() {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe_table_fixture(
        program_id: &Pubkey,
        table_id: u64,
        authority: &Pubkey,
    ) -> (Pubkey, Vec<u8>) {
        let table_id_bytes = table_id.to_le_bytes();
        let (recipe_table, bump) =
            Pubkey::find_program_address(&[RECIPE_TABLE_SEED, &table_id_bytes], program_id);
        let mut data = vec![0_u8; RecipeTable::LEN];
        RecipeTable::pack_empty(
            &mut data,
            &RecipeTableInitArgs {
                bump,
                table_id,
                authority,
                created_slot: 1,
                created_at: 1,
            },
        )
        .unwrap();
        (recipe_table, data)
    }

    fn material_output(recipe_table: &Pubkey) -> BackpackSlotRecord {
        BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_ITEM,
            category: BACKPACK_ITEM_CATEGORY_MATERIAL,
            quantity: 1,
            item_code: 1001,
            item_id: 1001,
            item_pda: *recipe_table,
            volume_mm3: 1_000_000,
            durability_current: 100,
            durability_max: 100,
            grade: 1,
            item_level: 1,
            quality_bps: 10_000,
            ..BackpackSlotRecord::default()
        }
    }

    fn block_slot(block_id: u16) -> BackpackSlotRecord {
        BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_BLOCK,
            quantity: 1,
            resource: BackpackResourceRecord {
                world_y: (block_id << BACKPACK_PACKED_Y_BITS) as i16,
                ..BackpackResourceRecord::default()
            },
            ..BackpackSlotRecord::default()
        }
    }

    fn material_slot(item_code: u16) -> BackpackSlotRecord {
        BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_ITEM,
            category: BACKPACK_ITEM_CATEGORY_MATERIAL,
            quantity: 1,
            item_code,
            ..BackpackSlotRecord::default()
        }
    }

    fn backpack_fixture(owner: &Pubkey, capacity: u8, slots: &[BackpackSlotRecord]) -> Vec<u8> {
        let mut data = vec![0_u8; BACKPACK_LEN];
        data[0..8].copy_from_slice(&BACKPACK_MAGIC);
        data[8..10].copy_from_slice(&BACKPACK_VERSION.to_le_bytes());
        data[11] = 1;
        data[BACKPACK_OWNER_OFFSET..BACKPACK_OWNER_OFFSET + 32].copy_from_slice(owner.as_ref());
        data[BACKPACK_CAPACITY_OFFSET] = capacity;
        data[BACKPACK_ITEM_COUNT_OFFSET] = slots.len() as u8;
        for (index, slot) in slots.iter().enumerate() {
            let offset = BACKPACK_RECORDS_OFFSET + index * BACKPACK_SLOT_RECORD_LEN;
            slot.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])
                .unwrap();
        }
        data
    }

    fn material_recipe(
        input_code: u16,
        input_quantity: u32,
        output_code: u16,
        min_heat_tier: u8,
    ) -> RecipeRecord {
        let mut input = material_slot(input_code);
        input.quantity = input_quantity;
        input.volume_mm3 = 250_000;
        let output_pda = Pubkey::new_unique();
        let mut output = material_output(&output_pda);
        output.item_code = output_code;
        output.item_id = output_code as u64;
        output.volume_mm3 = input.volume_mm3;
        RecipeRecord {
            recipe_id: output_code as u64,
            enabled: true,
            min_heat_tier,
            input_count: 1,
            output_count: 1,
            yield_bps: RECIPE_YIELD_BPS_DENOMINATOR,
            inputs: [input; RECIPE_MAX_INPUTS],
            outputs: [output; RECIPE_MAX_OUTPUTS],
            updated_slot: 1,
        }
    }

    fn material_merge_recipe(item_code: u16) -> RecipeRecord {
        let mut recipe = material_recipe(item_code, 1, item_code, 0);
        recipe.recipe_id = u64::from(item_code) + MATERIAL_MERGE_RECIPE_ID_OFFSET;
        recipe
    }

    fn block_recipe(block_id: u16, input_quantity: u32, output_code: u16) -> RecipeRecord {
        let mut input = block_slot(block_id);
        input.quantity = input_quantity;
        input.volume_mm3 = 1_000_000;
        let output_pda = Pubkey::new_unique();
        let mut output = material_output(&output_pda);
        output.item_code = output_code;
        output.item_id = output_code as u64;
        RecipeRecord {
            recipe_id: output_code as u64,
            enabled: true,
            min_heat_tier: 0,
            input_count: 1,
            output_count: 1,
            yield_bps: RECIPE_YIELD_BPS_DENOMINATOR,
            inputs: [input; RECIPE_MAX_INPUTS],
            outputs: [output; RECIPE_MAX_OUTPUTS],
            updated_slot: 1,
        }
    }

    #[test]
    fn fuel_tiers_match_the_consumable_browser_rules() {
        assert_eq!(fuel_heat_tier(&block_slot(29)), 1);
        assert_eq!(fuel_heat_tier(&block_slot(22)), 2);
        assert_eq!(fuel_heat_tier(&material_slot(1001)), 3);
        assert_eq!(fuel_heat_tier(&block_slot(47)), 4);
    }

    #[test]
    fn volcanic_recipe_inputs_are_not_backpack_fuels() {
        assert_eq!(fuel_heat_tier(&block_slot(14)), 0);
        assert_eq!(fuel_heat_tier(&block_slot(20)), 0);
    }

    #[test]
    fn one_material_stack_satisfies_a_multi_unit_recipe() {
        let owner = Pubkey::new_unique();
        let recipe = material_recipe(1031, 2, 1032, 0);
        let mut planks = material_slot(1031);
        planks.quantity = 4;
        planks.volume_mm3 = 950_000;
        let backpack = backpack_fixture(&owner, 2, &[planks]);

        let validated =
            BackpackAccountView::validate_recipe_inputs(&backpack, &owner, &[0], &[], &recipe, 1)
                .unwrap();

        assert_eq!(validated.consumption_quantities[0], 2);
        assert_eq!(validated.consumed_input_units, 2);
        assert_eq!(validated.input_volume_mm3, 475_000);
    }

    #[test]
    fn stacked_basalt_inputs_supply_three_recipe_batches() {
        let owner = Pubkey::new_unique();
        let recipe = block_recipe(14, 4, 1042);
        let mut large_stack = block_slot(14);
        large_stack.quantity = 30;
        large_stack.volume_mm3 = 30_000_000;
        let mut small_stack = block_slot(14);
        small_stack.quantity = 1;
        small_stack.volume_mm3 = 1_000_000;
        let backpack = backpack_fixture(&owner, 2, &[large_stack, small_stack]);

        let validated = BackpackAccountView::validate_recipe_inputs(
            &backpack,
            &owner,
            &[1, 0],
            &[],
            &recipe,
            3,
        )
        .unwrap();

        assert_eq!(validated.consumption_quantities[0], 11);
        assert_eq!(validated.consumption_quantities[1], 1);
        assert_eq!(validated.consumed_input_units, 12);
        assert_eq!(validated.input_volume_mm3, 12_000_000);
    }

    #[test]
    fn merge_recipe_consumes_complete_selected_stacks() {
        let owner = Pubkey::new_unique();
        let recipe = material_merge_recipe(1031);
        let mut first = material_slot(1031);
        first.quantity = 3;
        first.volume_mm3 = 300_000;
        let mut second = material_slot(1031);
        second.quantity = 5;
        second.volume_mm3 = 500_000;
        let backpack = backpack_fixture(&owner, 2, &[first, second]);

        let validated = BackpackAccountView::validate_recipe_inputs(
            &backpack,
            &owner,
            &[0, 1],
            &[],
            &recipe,
            2,
        )
        .unwrap();

        assert_eq!(validated.consumption_quantities[0], 3);
        assert_eq!(validated.consumption_quantities[1], 5);
        assert_eq!(validated.consumed_input_units, 8);
        assert_eq!(validated.input_volume_mm3, 800_000);
    }

    #[test]
    fn merge_recipe_rejects_unrepresentable_output_volume() {
        let owner = Pubkey::new_unique();
        let recipe = material_merge_recipe(1031);
        let mut first = material_slot(1031);
        first.volume_mm3 = u32::MAX;
        let mut second = material_slot(1031);
        second.volume_mm3 = 1;
        let backpack = backpack_fixture(&owner, 2, &[first, second]);

        let error = BackpackAccountView::validate_recipe_inputs(
            &backpack,
            &owner,
            &[0, 1],
            &[],
            &recipe,
            2,
        )
        .unwrap_err();

        assert_eq!(
            error,
            solana_program::program_error::ProgramError::Custom(
                NicechunkSmeltingError::OutputOverflow as u32,
            ),
        );
    }

    #[test]
    fn merge_recipe_rejects_unrepresentable_output_quantity() {
        let owner = Pubkey::new_unique();
        let recipe = material_merge_recipe(1031);
        let mut first = material_slot(1031);
        first.quantity = u32::MAX;
        first.volume_mm3 = 250_000;
        let mut second = material_slot(1031);
        second.volume_mm3 = 250_000;
        let backpack = backpack_fixture(&owner, 2, &[first, second]);

        let error = BackpackAccountView::validate_recipe_inputs(
            &backpack,
            &owner,
            &[0, 1],
            &[],
            &recipe,
            2,
        )
        .unwrap_err();

        assert_eq!(
            error,
            solana_program::program_error::ProgramError::Custom(
                NicechunkSmeltingError::OutputOverflow as u32,
            ),
        );
    }

    #[test]
    fn merge_recipe_rejects_fuel_indexes() {
        let owner = Pubkey::new_unique();
        let recipe = material_merge_recipe(1031);
        let first = material_slot(1031);
        let second = material_slot(1031);
        let fuel = block_slot(47);
        let backpack = backpack_fixture(&owner, 3, &[first, second, fuel]);

        let error = BackpackAccountView::validate_recipe_inputs(
            &backpack,
            &owner,
            &[0, 1],
            &[2],
            &recipe,
            2,
        )
        .unwrap_err();

        assert_eq!(
            error,
            solana_program::program_error::ProgramError::Custom(
                NicechunkSmeltingError::InputRecipeMismatch as u32,
            ),
        );
    }

    #[test]
    fn material_merge_identity_requires_every_canonical_field() {
        let canonical = material_merge_recipe(1031);
        assert!(recipe_is_material_merge(&canonical));

        let mut cases = Vec::new();
        let mut recipe = canonical;
        recipe.recipe_id -= 1;
        cases.push(("recipe id", recipe));
        let mut recipe = canonical;
        recipe.input_count = 2;
        cases.push(("input count", recipe));
        let mut recipe = canonical;
        recipe.output_count = 2;
        cases.push(("output count", recipe));
        let mut recipe = canonical;
        recipe.min_heat_tier = 1;
        cases.push(("heat tier", recipe));
        let mut recipe = canonical;
        recipe.yield_bps -= 1;
        cases.push(("yield", recipe));
        let mut recipe = canonical;
        recipe.inputs[0].kind = BACKPACK_SLOT_KIND_BLOCK;
        cases.push(("input kind", recipe));
        let mut recipe = canonical;
        recipe.outputs[0].kind = BACKPACK_SLOT_KIND_BLOCK;
        cases.push(("output kind", recipe));
        let mut recipe = canonical;
        recipe.inputs[0].category = 2;
        cases.push(("input category", recipe));
        let mut recipe = canonical;
        recipe.outputs[0].category = 2;
        cases.push(("output category", recipe));
        let mut recipe = canonical;
        recipe.outputs[0].item_code += 1;
        cases.push(("item code", recipe));
        let mut recipe = canonical;
        recipe.outputs[0].item_id += 1;
        cases.push(("item id", recipe));
        let mut recipe = canonical;
        recipe.inputs[0].quantity = 2;
        cases.push(("input quantity", recipe));
        let mut recipe = canonical;
        recipe.outputs[0].quantity = 2;
        cases.push(("output quantity", recipe));
        let mut recipe = canonical;
        recipe.inputs[0].volume_mm3 = 0;
        cases.push(("input volume", recipe));
        let mut recipe = canonical;
        recipe.outputs[0].volume_mm3 += 1;
        cases.push(("output volume", recipe));

        for (field, recipe) in cases {
            assert!(
                !recipe_is_material_merge(&recipe),
                "changed {field} must not classify as a material merge",
            );
        }
    }

    #[test]
    fn stacked_input_and_fuel_do_not_fake_a_free_backpack_slot() {
        let owner = Pubkey::new_unique();
        let recipe = material_recipe(1031, 1, 1032, 3);
        let mut planks = material_slot(1031);
        planks.quantity = 4;
        planks.volume_mm3 = 950_000;
        let mut charcoal = material_slot(1001);
        charcoal.quantity = 4;
        charcoal.volume_mm3 = 750_000;
        let backpack = backpack_fixture(&owner, 2, &[planks, charcoal]);

        let error =
            BackpackAccountView::validate_recipe_inputs(&backpack, &owner, &[0], &[1], &recipe, 1)
                .unwrap_err();

        assert!(matches!(
            error,
            solana_program::program_error::ProgramError::Custom(code)
                if code == NicechunkSmeltingError::BackpackCapacityExceeded as u32
        ));
    }

    #[test]
    fn recipe_table_identity_binds_stored_id_bump_pda_and_authority() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let (recipe_table, mut data) = recipe_table_fixture(&program_id, 225, &authority);

        assert_eq!(
            RecipeTable::validate_identity(&data, &program_id, &recipe_table, &authority).unwrap(),
            225,
        );
        assert!(matches!(
            RecipeTable::validate_identity(
                &data,
                &program_id,
                &recipe_table,
                &Pubkey::new_unique(),
            ),
            Err(NicechunkSmeltingError::UnauthorizedAuthority),
        ));
        data[RecipeTable::BUMP_OFFSET] ^= 1;
        assert!(matches!(
            RecipeTable::validate_identity(&data, &program_id, &recipe_table, &authority),
            Err(NicechunkSmeltingError::InvalidRecipeTablePda),
        ));
    }

    #[test]
    fn smelting_outputs_must_be_materials_backed_by_the_recipe_table() {
        let recipe_table = Pubkey::new_unique();
        assert!(material_output(&recipe_table)
            .validate_recipe_output(&recipe_table)
            .is_ok());

        let mut wrong_pda = material_output(&recipe_table);
        wrong_pda.item_pda = Pubkey::new_unique();
        assert!(matches!(
            wrong_pda.validate_recipe_output(&recipe_table),
            Err(NicechunkSmeltingError::InvalidRecipeOutput),
        ));

        let mut wrong_category = material_output(&recipe_table);
        wrong_category.category = 2;
        assert!(matches!(
            wrong_category.validate_recipe_output(&recipe_table),
            Err(NicechunkSmeltingError::InvalidRecipeOutput),
        ));
    }
}
