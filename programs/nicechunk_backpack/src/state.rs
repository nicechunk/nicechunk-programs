use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkBackpackError;

pub const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
pub const BACKPACK_VERSION: u16 = 3;
pub const BACKPACK_SEED: &[u8] = b"backpack";
pub const BACKPACK_DEFAULT_CAPACITY: u8 = 50;
pub const BACKPACK_MAX_CAPACITY: u8 = 99;
pub const BACKPACK_HEADER_LEN: usize = 128;
pub const BACKPACK_RESOURCE_RECORD_LEN: usize = 10;
pub const BACKPACK_SLOT_RECORD_LEN: usize = 80;
pub const BACKPACK_RECORD_LEN: usize = BACKPACK_SLOT_RECORD_LEN;
pub const BACKPACK_LEN: usize =
    BACKPACK_HEADER_LEN + BACKPACK_MAX_CAPACITY as usize * BACKPACK_RECORD_LEN;
pub const BACKPACK_STATE_CARRIED: u8 = 1;
pub const BACKPACK_SLOT_KIND_BLOCK: u8 = 1;
pub const BACKPACK_SLOT_KIND_ITEM: u8 = 2;
pub const BACKPACK_ITEM_CATEGORY_MATERIAL: u8 = 1;
pub const BACKPACK_ITEM_CATEGORY_FORGED: u8 = 2;
pub const BACKPACK_FORGED_ITEM_CODE: u16 = 8;
pub const SESSION_ACTION_BREAK_BLOCK: u8 = 1;
pub const DEFAULT_MATERIAL_VOLUME_MM3: u32 = 1_000_000;
pub const DURABILITY_BPS_DENOMINATOR: u64 = 10_000;
pub const MAX_FORGING_INPUTS: usize = 24;

pub const PLAYER_PROFILE_LEN: usize = 473;
pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
pub const PLAYER_PROFILE_FORGING_XP_OFFSET: usize = 449;

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
        if data.len() != BACKPACK_LEN || data[0..8] != BACKPACK_MAGIC {
            return Err(NicechunkBackpackError::InvalidBackpackData);
        }
        let version = read_u16(data, 8);
        if version != BACKPACK_VERSION || data[11] != 1 {
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
        Self::append_resource_with_volume(data, owner, record, 0, updated_slot)
    }

    pub fn append_resource_with_volume(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackResourceRecord,
        volume_mm3: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let capacity = data[Self::CAPACITY_OFFSET];
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count >= capacity {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        BackpackSlotRecord::from_block_resource_with_volume(*record, volume_mm3)
            .pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
        data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_add(1);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn append_resources_lossy(
        data: &mut [u8],
        owner: &Pubkey,
        records: &[BackpackResourceRecord],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::append_resources_lossy_with_volumes(data, owner, records, &[], updated_slot)
    }

    pub fn append_resources_lossy_with_volumes(
        data: &mut [u8],
        owner: &Pubkey,
        records: &[BackpackResourceRecord],
        volumes_mm3: &[u32],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let capacity = data[Self::CAPACITY_OFFSET];
        let mut item_count = data[Self::ITEM_COUNT_OFFSET];
        if records.is_empty() || item_count >= capacity {
            return Ok(());
        }

        for (index, record) in records.iter().enumerate() {
            if item_count >= capacity {
                break;
            }
            let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
            BackpackSlotRecord::from_block_resource_with_volume(
                *record,
                volumes_mm3.get(index).copied().unwrap_or(0),
            )
            .pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            item_count = item_count.saturating_add(1);
        }
        data[Self::ITEM_COUNT_OFFSET] = item_count;
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn append_item(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackSlotRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if record.kind == BACKPACK_SLOT_KIND_BLOCK && record.quantity == 0 {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }
        if record.kind == BACKPACK_SLOT_KIND_ITEM
            && (record.quantity == 0 || record.item_id == 0 || record.item_pda == Pubkey::default())
        {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }
        let capacity = data[Self::CAPACITY_OFFSET];
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count >= capacity {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        let mut normalized = *record;
        normalized.fill_default_metadata();
        normalized.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
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

        let start = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        let end = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        if start + BACKPACK_SLOT_RECORD_LEN < end {
            data.copy_within(start + BACKPACK_SLOT_RECORD_LEN..end, start);
        }
        let last_start = Self::RECORDS_OFFSET + (item_count - 1) as usize * BACKPACK_SLOT_RECORD_LEN;
        data[last_start..last_start + BACKPACK_SLOT_RECORD_LEN].fill(0);
        data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_sub(1);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn remove_resources_at(
        data: &mut [u8],
        owner: &Pubkey,
        indexes: &[u8],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if indexes.is_empty() || indexes.len() > BACKPACK_MAX_CAPACITY as usize {
            return Err(NicechunkBackpackError::InvalidInstruction.into());
        }

        let mut selected = [false; BACKPACK_MAX_CAPACITY as usize];
        for index in indexes {
            if *index >= item_count {
                return Err(NicechunkBackpackError::InvalidResourceIndex.into());
            }
            let selected_index = *index as usize;
            if selected[selected_index] {
                return Err(NicechunkBackpackError::InvalidInstruction.into());
            }
            selected[selected_index] = true;
        }

        // Remove from high to low so earlier selected indexes keep their original meaning.
        let mut cursor = BACKPACK_MAX_CAPACITY as usize;
        while cursor > 0 {
            cursor -= 1;
            if selected[cursor] {
                Self::remove_resource_at(data, owner, cursor as u8, updated_slot)?;
            }
        }
        Ok(())
    }

    pub fn slot_at(data: &[u8], index: u8) -> Result<BackpackSlotRecord, NicechunkBackpackError> {
        Self::validate(data)?;
        if index >= data[Self::ITEM_COUNT_OFFSET] {
            return Err(NicechunkBackpackError::InvalidResourceIndex);
        }
        let offset = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])
    }

    pub fn forge_equipment_from_materials(
        data: &mut [u8],
        owner: &Pubkey,
        indexes: &[u8],
        item_id: u64,
        requested_item_level: u8,
        item_pda: &Pubkey,
        forging_level: u8,
        updated_slot: u64,
    ) -> Result<ForgeOutcome, solana_program::program_error::ProgramError> {
        Self::validate_owner(data, owner)?;
        if indexes.is_empty() || indexes.len() > MAX_FORGING_INPUTS {
            return Err(NicechunkBackpackError::InvalidInstruction.into());
        }
        if item_id == 0 || *item_pda == Pubkey::default() {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }

        let item_count = data[Self::ITEM_COUNT_OFFSET];
        let mut selected = [false; BACKPACK_MAX_CAPACITY as usize];
        let mut materials = Vec::with_capacity(indexes.len());
        for index in indexes {
            if *index >= item_count {
                return Err(NicechunkBackpackError::InvalidResourceIndex.into());
            }
            let selected_index = *index as usize;
            if selected[selected_index] {
                return Err(NicechunkBackpackError::InvalidInstruction.into());
            }
            selected[selected_index] = true;
            let mut slot = Self::slot_at(data, *index)?;
            slot.fill_default_metadata();
            if slot.kind != BACKPACK_SLOT_KIND_ITEM
                || slot.category != BACKPACK_ITEM_CATEGORY_MATERIAL
                || slot.item_code == 0
            {
                return Err(NicechunkBackpackError::InvalidForgingMaterial.into());
            }
            materials.push(slot);
        }

        let outcome = calculate_forge_outcome(&materials, requested_item_level, forging_level);
        Self::remove_resources_at(data, owner, indexes, updated_slot)?;
        let output = BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_ITEM,
            category: BACKPACK_ITEM_CATEGORY_FORGED,
            flags: 0,
            quantity: 1,
            resource: BackpackResourceRecord::default(),
            item_code: BACKPACK_FORGED_ITEM_CODE,
            item_id,
            item_pda: *item_pda,
            volume_mm3: outcome.volume_mm3,
            durability_current: outcome.durability_max,
            durability_max: outcome.durability_max,
            grade: outcome.grade,
            item_level: outcome.item_level,
            quality_bps: outcome.quality_bps,
            metadata: 0,
        };
        Self::append_item(data, owner, &output, updated_slot)?;
        Ok(outcome)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct BackpackResourceRecord {
    pub world_x: i32,
    pub world_y: i16,
    pub world_z: i32,
}

impl BackpackResourceRecord {
    pub const LEN: usize = BACKPACK_RESOURCE_RECORD_LEN;

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

#[derive(Clone, Copy, Default)]
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
    pub const LEN: usize = BACKPACK_SLOT_RECORD_LEN;

    pub fn from_block_resource(resource: BackpackResourceRecord) -> Self {
        Self::from_block_resource_with_volume(resource, 0)
    }

    pub fn from_block_resource_with_volume(
        resource: BackpackResourceRecord,
        volume_mm3: u32,
    ) -> Self {
        Self {
            kind: BACKPACK_SLOT_KIND_BLOCK,
            category: 0,
            flags: 0,
            quantity: 1,
            resource,
            item_code: 0,
            item_id: 0,
            item_pda: Pubkey::default(),
            volume_mm3,
            durability_current: 0,
            durability_max: 0,
            grade: 0,
            item_level: 0,
            quality_bps: 0,
            metadata: 0,
        }
    }

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkBackpackError> {
        if data.len() != BACKPACK_SLOT_RECORD_LEN {
            return Err(NicechunkBackpackError::InvalidInventoryItem);
        }
        let kind = data[0];
        if kind != BACKPACK_SLOT_KIND_BLOCK && kind != BACKPACK_SLOT_KIND_ITEM {
            return Err(NicechunkBackpackError::InvalidInventoryItem);
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
                    .map_err(|_| NicechunkBackpackError::InvalidInventoryItem)?,
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
            return Err(NicechunkBackpackError::InvalidInventoryItem);
        }
        if record.kind == BACKPACK_SLOT_KIND_ITEM
            && (record.item_id == 0 || record.item_pda == Pubkey::default())
        {
            return Err(NicechunkBackpackError::InvalidInventoryItem);
        }
        Ok(record)
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != BACKPACK_SLOT_RECORD_LEN {
            return Err(NicechunkBackpackError::PackSizeMismatch.into());
        }
        if self.kind != BACKPACK_SLOT_KIND_BLOCK && self.kind != BACKPACK_SLOT_KIND_ITEM {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }
        if self.quantity == 0 {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }
        if self.kind == BACKPACK_SLOT_KIND_ITEM
            && (self.item_id == 0 || self.item_pda == Pubkey::default())
        {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
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

    pub fn fill_default_metadata(&mut self) {
        if self.kind != BACKPACK_SLOT_KIND_ITEM {
            return;
        }
        if self.grade == 0 {
            self.grade = material_grade_for_item_code(self.item_code);
        }
        if self.item_level == 0 {
            self.item_level = material_item_level_for_grade(self.grade);
        }
        if self.quality_bps == 0 {
            self.quality_bps = material_quality_bps_for_grade(self.grade);
        }
        if self.durability_max == 0 {
            self.durability_max = material_durability_max(self);
        }
        if self.durability_current == 0 {
            self.durability_current = self.durability_max;
        }
        if self.durability_current > self.durability_max {
            self.durability_current = self.durability_max;
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct ForgeOutcome {
    pub grade: u8,
    pub item_level: u8,
    pub durability_max: u32,
    pub quality_bps: u16,
    pub volume_mm3: u32,
    pub gained_xp: u64,
}

fn calculate_forge_outcome(
    materials: &[BackpackSlotRecord],
    requested_item_level: u8,
    forging_level: u8,
) -> ForgeOutcome {
    let mut total_volume = 0_u64;
    let mut total_raw_durability = 0_u64;
    let mut total_effective_durability = 0_u64;
    let mut weighted_grade = 0_u64;
    let mut weighted_quality = 0_u64;
    let mut weak_grade_cap = 10_u8;

    for material in materials {
        let volume = slot_volume_mm3(material) as u64;
        let grade = material.grade.max(1).min(10);
        let quality = material.quality_bps.max(1).min(DURABILITY_BPS_DENOMINATOR as u16) as u64;
        let max_durability = material.durability_max.max(1) as u64;
        let current_durability = material.durability_current.max(1).min(material.durability_max.max(1)) as u64;
        total_volume = total_volume.saturating_add(volume);
        total_raw_durability = total_raw_durability.saturating_add(max_durability);
        total_effective_durability = total_effective_durability
            .saturating_add(current_durability.saturating_mul(quality) / DURABILITY_BPS_DENOMINATOR);
        weighted_grade = weighted_grade.saturating_add(grade as u64 * volume);
        weighted_quality = weighted_quality.saturating_add(quality * volume);
    }

    for material in materials {
        let volume = slot_volume_mm3(material) as u64;
        if total_volume > 0 && volume.saturating_mul(5) >= total_volume {
            weak_grade_cap = weak_grade_cap.min(material.grade.max(1).min(10).saturating_add(2).min(10));
        }
    }

    let material_grade = if total_volume > 0 {
        ((weighted_grade + total_volume / 2) / total_volume) as u8
    } else {
        1
    }
    .max(1)
    .min(10);
    let quality_bps = if total_volume > 0 {
        ((weighted_quality + total_volume / 2) / total_volume) as u16
    } else {
        material_quality_bps_for_grade(material_grade)
    }
    .max(1)
    .min(DURABILITY_BPS_DENOMINATOR as u16);
    let material_level = material_item_level_from_durability(total_effective_durability, total_volume);
    let item_level = requested_item_level.max(1).min(100).min(material_level.max(1));
    let item_level_grade = 1_u8.saturating_add((item_level.saturating_sub(1) / 10).min(9));
    let skill_grade = 1_u8.saturating_add(forging_level.min(9));
    let skill_cap = 3_u8.saturating_add(forging_level.min(7)).min(10);
    let blended_grade =
        ((material_grade as u16 * 2 + item_level_grade as u16 + skill_grade as u16 + 2) / 4)
            as u8;
    let grade = blended_grade
        .max(1)
        .min(10)
        .min(skill_cap)
        .min(material_grade.saturating_add(1).min(10))
        .min(weak_grade_cap);

    let skill_factor = 90_u64
        .saturating_add(grade as u64 * 5)
        .saturating_add(forging_level.min(10) as u64 * 3);
    let level_factor = 100_u64.saturating_add(item_level as u64 / 2);
    let candidate = total_effective_durability
        .saturating_mul(skill_factor)
        .saturating_mul(level_factor)
        / 10_000;
    let material_cap = total_raw_durability
        .saturating_mul(105_u64.saturating_add(forging_level.min(10) as u64))
        / 100;
    let durability_max = candidate
        .max(1)
        .min(material_cap.max(1))
        .min(u32::MAX as u64) as u32;
    let gained_xp = (grade as u64)
        .saturating_mul(grade as u64)
        .saturating_mul(25)
        .saturating_add(item_level as u64 * grade as u64)
        .saturating_add(materials.len() as u64);

    ForgeOutcome {
        grade,
        item_level,
        durability_max,
        quality_bps,
        volume_mm3: total_volume.max(1).min(u32::MAX as u64) as u32,
        gained_xp,
    }
}

fn material_durability_max(slot: &BackpackSlotRecord) -> u32 {
    let volume_units = (slot_volume_mm3(slot) as u64).saturating_add(999) / 1_000;
    let grade = slot.grade.max(1).min(10) as u64;
    let quality = slot
        .quality_bps
        .max(1)
        .min(DURABILITY_BPS_DENOMINATOR as u16) as u64;
    let base_per_unit = 14_u64.saturating_add(grade.saturating_mul(6));
    volume_units
        .saturating_mul(base_per_unit)
        .saturating_mul(quality)
        .saturating_div(DURABILITY_BPS_DENOMINATOR)
        .max(1)
        .min(u32::MAX as u64) as u32
}

fn material_grade_for_item_code(item_code: u16) -> u8 {
    if item_code == 0 {
        return 1;
    }
    let material_index = item_code.saturating_sub(1001);
    match material_index {
        0..=5 => 2,
        6..=13 => 3,
        14..=23 => 4,
        24..=35 => 5,
        36..=47 => 6,
        48..=63 => 7,
        64..=80 => 8,
        _ => 4,
    }
}

fn material_item_level_for_grade(grade: u8) -> u8 {
    grade.max(1).min(10).saturating_mul(10).saturating_sub(4)
}

fn material_quality_bps_for_grade(grade: u8) -> u16 {
    (5_200_u16)
        .saturating_add(grade.max(1).min(10) as u16 * 420)
        .min(DURABILITY_BPS_DENOMINATOR as u16)
}

fn material_item_level_from_durability(effective_durability: u64, total_volume_mm3: u64) -> u8 {
    let durability_level = integer_sqrt(effective_durability / 25).min(80);
    let volume_level = (total_volume_mm3 / 500_000).min(20);
    (1_u64.saturating_add(durability_level).saturating_add(volume_level))
        .min(100) as u8
}

fn slot_volume_mm3(slot: &BackpackSlotRecord) -> u32 {
    if slot.volume_mm3 > 0 {
        slot.volume_mm3
    } else {
        DEFAULT_MATERIAL_VOLUME_MM3
    }
}

fn integer_sqrt(value: u64) -> u64 {
    if value <= 1 {
        return value;
    }
    let mut estimate = value;
    let mut next = (estimate + value / estimate) / 2;
    while next < estimate {
        estimate = next;
        next = (estimate + value / estimate) / 2;
    }
    estimate
}

pub struct PlayerProfileView;

impl PlayerProfileView {
    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        if data.len() != PLAYER_PROFILE_LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkBackpackError::InvalidPlayerProfile.into());
        }
        if &data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkBackpackError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn forging_level(data: &[u8]) -> Result<u8, NicechunkBackpackError> {
        if data.len() != PLAYER_PROFILE_LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkBackpackError::InvalidPlayerProfile);
        }
        Ok(forging_level_from_xp(read_u64(data, PLAYER_PROFILE_FORGING_XP_OFFSET)))
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

pub fn forging_level_from_xp(xp: u64) -> u8 {
    let mut level = 0_u8;
    for (index, required_total) in FORGING_TOTAL_XP_BY_LEVEL.iter().enumerate() {
        if xp >= *required_total {
            level = index as u8;
        }
    }
    level.min(10)
}

const FORGING_TOTAL_XP_BY_LEVEL: [u64; 11] = [
    0, 250, 900, 2_100, 4_000, 6_800, 10_700, 16_000, 23_000, 32_000, 45_000,
];

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
