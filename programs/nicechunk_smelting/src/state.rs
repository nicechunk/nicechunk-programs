use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkSmeltingError;

pub const RECIPE_TABLE_MAGIC: [u8; 8] = *b"NCKSMR01";
pub const RECIPE_TABLE_VERSION: u16 = 1;
pub const RECIPE_TABLE_SEED: &[u8] = b"smelting-recipes";
pub const SMELTING_AUTHORITY_SEED: &[u8] = b"smelting-authority";
pub const RECIPE_TABLE_HEADER_LEN: usize = 96;
pub const RECIPE_TABLE_MAX_RECIPES: usize = 12;
pub const RECIPE_MAX_INPUTS: usize = 8;
pub const RECIPE_MAX_OUTPUTS: usize = 4;
pub const BACKPACK_LEGACY_RECORD_LEN: usize = 10;
pub const BACKPACK_SLOT_RECORD_LEN: usize = 64;
pub const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
pub const BACKPACK_SLOT_KIND_ITEM: u8 = 2;
pub const BACKPACK_ITEM_CATEGORY_MATERIAL: u8 = 1;
pub const DEFAULT_RESOURCE_VOLUME_MM3: u32 = 1_000_000;
pub const DEFAULT_OUTPUT_VOLUME_DIVISOR: u32 = 60;
const BACKPACK_PACKED_Y_BITS: u16 = 9;
pub const RECIPE_RECORD_LEN: usize =
    8 + 1 + 1 + 1 + 1 + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN + 8;
pub const RECIPE_TABLE_LEN: usize =
    RECIPE_TABLE_HEADER_LEN + RECIPE_TABLE_MAX_RECIPES * RECIPE_RECORD_LEN;
pub const UPSERT_RECIPE_ARGS_LEN: usize =
    8 + 1 + 1 + 1 + 1 + RECIPE_MAX_INPUTS * BACKPACK_SLOT_RECORD_LEN + RECIPE_MAX_OUTPUTS * BACKPACK_SLOT_RECORD_LEN;

const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
const BACKPACK_LEGACY_VERSION: u16 = 1;
const BACKPACK_VERSION: u16 = 2;
const BACKPACK_HEADER_LEN: usize = 128;
const BACKPACK_MAX_CAPACITY: usize = 99;
const BACKPACK_LEGACY_LEN: usize = BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY * BACKPACK_LEGACY_RECORD_LEN;
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct BackpackResourceRecord {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
}

impl BackpackResourceRecord {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != BACKPACK_LEGACY_RECORD_LEN {
            return Err(NicechunkSmeltingError::InvalidInstruction);
        }
        Ok(Self {
            world_x: read_i32(data, 0),
            world_y: read_i16(data, 4),
            world_z: read_i32(data, 6),
        })
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != BACKPACK_LEGACY_RECORD_LEN {
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
        };
        if record.quantity == 0 {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        if record.kind == BACKPACK_SLOT_KIND_ITEM
            && (record.item_id == 0 || record.item_pda == Pubkey::default())
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
        if self.kind == BACKPACK_SLOT_KIND_ITEM
            && (self.item_id == 0 || self.item_pda == Pubkey::default())
        {
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
            inputs: [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS],
            outputs: [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS],
            updated_slot: 0,
        }
    }
}

impl RecipeRecord {
    pub fn unpack_args(data: &[u8], updated_slot: u64) -> Result<Self, NicechunkSmeltingError> {
        if data.len() != UPSERT_RECIPE_ARGS_LEN {
            return Err(NicechunkSmeltingError::InvalidInstruction);
        }
        let recipe_id = read_u64(data, 0);
        let enabled = data[8] == 1;
        let min_heat_tier = data[9];
        let input_count = data[10];
        let output_count = data[11];
        if recipe_id == 0
            || input_count == 0
            || input_count as usize > RECIPE_MAX_INPUTS
            || output_count == 0
            || output_count as usize > RECIPE_MAX_OUTPUTS
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let mut inputs = [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS];
        let mut outputs = [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS];
        let mut offset = 12;
        for (index, input) in inputs.iter_mut().enumerate() {
            if index < input_count as usize {
                *input = BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            }
            offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for (index, output) in outputs.iter_mut().enumerate() {
            if index < output_count as usize {
                *output = BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
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
        for input in self.inputs.iter() {
            input.pack(&mut writer.dst[writer.offset..writer.offset + BACKPACK_SLOT_RECORD_LEN])?;
            writer.offset += BACKPACK_SLOT_RECORD_LEN;
        }
        for output in self.outputs.iter() {
            output.pack(&mut writer.dst[writer.offset..writer.offset + BACKPACK_SLOT_RECORD_LEN])?;
            writer.offset += BACKPACK_SLOT_RECORD_LEN;
        }
        writer.u64(self.updated_slot)?;
        if writer.offset != RECIPE_RECORD_LEN {
            return Err(NicechunkSmeltingError::PackSizeMismatch.into());
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
        if recipe_id == 0 {
            return Ok(Self::default());
        }
        if input_count == 0
            || input_count as usize > RECIPE_MAX_INPUTS
            || output_count == 0
            || output_count as usize > RECIPE_MAX_OUTPUTS
        {
            return Err(NicechunkSmeltingError::InvalidRecipe);
        }
        let mut inputs = [BackpackSlotRecord::default(); RECIPE_MAX_INPUTS];
        let mut outputs = [BackpackSlotRecord::default(); RECIPE_MAX_OUTPUTS];
        let mut offset = 12;
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
            inputs,
            outputs,
            updated_slot: read_u64(data, offset),
        })
    }
}

pub struct RecipeTable;

impl RecipeTable {
    pub const LEN: usize = RECIPE_TABLE_LEN;
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

    pub fn set_authority(data: &mut [u8], authority: &Pubkey, updated_slot: u64) -> ProgramResult {
        Self::validate(data)?;
        data[Self::AUTHORITY_OFFSET..Self::AUTHORITY_OFFSET + 32].copy_from_slice(authority.as_ref());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn validate_authority(data: &[u8], authority: &Pubkey) -> ProgramResult {
        Self::validate(data)?;
        if Self::authority(data)? != *authority {
            return Err(NicechunkSmeltingError::UnauthorizedAuthority.into());
        }
        Ok(())
    }

    pub fn upsert_recipe(data: &mut [u8], recipe: &RecipeRecord, updated_slot: u64) -> ProgramResult {
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

    pub fn find_recipe(data: &[u8], recipe_id: u64) -> Result<RecipeRecord, NicechunkSmeltingError> {
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

impl BackpackAccountView {
    pub fn validate(data: &[u8]) -> ProgramResult {
        if !is_supported_backpack_len(data.len()) || data[0..8] != BACKPACK_MAGIC {
            return Err(NicechunkSmeltingError::InvalidBackpackData.into());
        }
        let version = read_u16(data, 8);
        if !is_supported_backpack_version(version) || data[11] != 1 {
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
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if multiplier == 0 || indexes.len() != recipe.input_count as usize * multiplier as usize {
            return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
        }
        let capacity = data[BACKPACK_CAPACITY_OFFSET] as usize;
        let item_count = data[BACKPACK_ITEM_COUNT_OFFSET] as usize;
        let remove_count = indexes.len().saturating_add(fuel_indexes.len());
        if item_count.saturating_sub(remove_count).saturating_add(recipe.output_count as usize) > capacity {
            return Err(NicechunkSmeltingError::BackpackCapacityExceeded.into());
        }

        let mut seen_indexes = [false; BACKPACK_MAX_CAPACITY];
        let mut matched_inputs = [0_u16; RECIPE_MAX_INPUTS];
        for index in indexes {
            let selected = *index as usize;
            if selected >= item_count || seen_indexes[selected] {
                return Err(NicechunkSmeltingError::InvalidInputIndex.into());
            }
            seen_indexes[selected] = true;
            let record = Self::slot_at(data, *index)?;
            let mut matched = false;
            for recipe_index in 0..recipe.input_count as usize {
                if matched_inputs[recipe_index] < multiplier && recipe_input_matches(&recipe.inputs[recipe_index], &record) {
                    matched_inputs[recipe_index] = matched_inputs[recipe_index].saturating_add(1);
                    matched = true;
                    break;
                }
            }
            if !matched {
                return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
            }
        }
        for matched in matched_inputs.iter().take(recipe.input_count as usize) {
            if *matched != multiplier {
                return Err(NicechunkSmeltingError::InputRecipeMismatch.into());
            }
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
        Ok(())
    }

    fn slot_at(data: &[u8], index: u8) -> Result<BackpackSlotRecord, NicechunkSmeltingError> {
        let record_len = backpack_record_len(data)?;
        let offset = BACKPACK_RECORDS_OFFSET + index as usize * record_len;
        if record_len == BACKPACK_LEGACY_RECORD_LEN {
            let resource = BackpackResourceRecord::unpack(&data[offset..offset + record_len])?;
            return Ok(BackpackSlotRecord::from_block_resource(resource));
        }
        BackpackSlotRecord::unpack(&data[offset..offset + record_len])
    }
}

fn recipe_input_matches(expected: &BackpackSlotRecord, actual: &BackpackSlotRecord) -> bool {
    if expected.kind != actual.kind {
        return false;
    }
    match expected.kind {
        BACKPACK_SLOT_KIND_BLOCK => packed_block_id(expected.resource.world_y) == packed_block_id(actual.resource.world_y),
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
        return 2;
    }
    if slot.kind != BACKPACK_SLOT_KIND_BLOCK {
        return 0;
    }
    match packed_block_id(slot.resource.world_y) {
        14 | 20 => 4,             // basalt / lava heat
        47 => 3,                  // coal
        22 | 24 | 26 | 27 => 2,   // wood-like fuels
        29 | 31 | 36 => 1,        // dry grass / dead bush / thorn
        _ => 0,
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

fn backpack_record_len(data: &[u8]) -> Result<usize, NicechunkSmeltingError> {
    if data.len() == BACKPACK_LEGACY_LEN && read_u16(data, 8) == BACKPACK_LEGACY_VERSION {
        return Ok(BACKPACK_LEGACY_RECORD_LEN);
    }
    if data.len() == BACKPACK_LEN && read_u16(data, 8) == BACKPACK_VERSION {
        return Ok(BACKPACK_SLOT_RECORD_LEN);
    }
    Err(NicechunkSmeltingError::InvalidBackpackData)
}

fn is_supported_backpack_len(len: usize) -> bool {
    len == BACKPACK_LEN || len == BACKPACK_LEGACY_LEN
}

fn is_supported_backpack_version(version: u16) -> bool {
    version == BACKPACK_VERSION || version == BACKPACK_LEGACY_VERSION
}
