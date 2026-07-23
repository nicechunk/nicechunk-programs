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
pub const BACKPACK_ITEM_CATEGORY_BLUEPRINT: u8 = 3;
pub const BACKPACK_FORGED_ITEM_CODE: u16 = 8;
pub const BACKPACK_BLUEPRINT_ITEM_CODE: u16 = 9;
pub const BACKPACK_ITEM_FLAG_UNIQUE: u16 = 1;
pub const BACKPACK_ITEM_FLAG_MASS_VALID: u16 = 1 << 15;
pub const BACKPACK_FLAG_TOTAL_MASS_INITIALIZED: u8 = 1;
pub const BACKPACK_DEFAULT_RESOURCE_VOLUME_MM3: u32 = 1_000_000;
pub const BACKPACK_PACKED_Y_BITS: u16 = 9;
pub const LEGACY_FORGED_MATERIAL_ID: u16 = u16::MAX;
pub const MATERIAL_PHYSICS_MAGIC: [u8; 8] = *b"NCKPHY01";
pub const MATERIAL_PHYSICS_VERSION: u16 = 1;
pub const MATERIAL_PHYSICS_SEED: &[u8] = b"material-physics-v1";
pub const MATERIAL_PHYSICS_HEADER_LEN: usize = 128;
pub const MATERIAL_PHYSICS_RECORD_LEN: usize = 4;
pub const MATERIAL_PHYSICS_MAX_RECORDS: usize = 240;
pub const MATERIAL_PHYSICS_LEN: usize =
    MATERIAL_PHYSICS_HEADER_LEN + MATERIAL_PHYSICS_MAX_RECORDS * MATERIAL_PHYSICS_RECORD_LEN;
pub const BLUEPRINT_ITEM_MAGIC: [u8; 8] = *b"NCKBPT01";
pub const BLUEPRINT_ITEM_VERSION: u16 = 1;
pub const BLUEPRINT_ITEM_SEED: &[u8] = b"blueprint-item";
pub const SESSION_ACTION_BREAK_BLOCK: u8 = 1;
pub const DURABILITY_BPS_DENOMINATOR: u64 = 10_000;
pub const MAX_FORGING_INPUTS: usize = 24;
pub const MAX_VERIFIED_FORGE_CODE_BYTES: usize = 640;
const NCF1_VERSION: u32 = 14;
const NCF1_ATTRIBUTE_COUNT: usize = 12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForgeMaterialRequirements {
    pub required_volume_mm3: u64,
    pub required_effective_durability: u64,
    pub output_mass_grams: u32,
}

impl ForgeMaterialRequirements {
    pub fn validate(&self) -> Result<(), NicechunkBackpackError> {
        if self.required_volume_mm3 == 0
            || self.required_effective_durability == 0
            || self.output_mass_grams == 0
        {
            return Err(NicechunkBackpackError::InvalidForgeMaterialRequirements);
        }
        Ok(())
    }
}

pub fn verified_forge_design(
    code: &[u8],
) -> Result<(u32, ForgeMaterialRequirements), NicechunkBackpackError> {
    if code.len() < 14 || code.len() > MAX_VERIFIED_FORGE_CODE_BYTES {
        return Err(NicechunkBackpackError::InvalidForgeMaterialRequirements);
    }
    let mut bit_offset = 0_usize;
    if read_bits(code, &mut bit_offset, 4)? != NCF1_VERSION {
        return Err(NicechunkBackpackError::InvalidForgeMaterialRequirements);
    }
    let mass_grams = (read_bits(code, &mut bit_offset, 16)? as u64).saturating_mul(5);
    let volume_cm3 = read_bits(code, &mut bit_offset, 16)? as u64;
    let mut attributes = [0_u64; NCF1_ATTRIBUTE_COUNT];
    for attribute in attributes.iter_mut() {
        let compact = read_bits(code, &mut bit_offset, 6)? as u64;
        *attribute = compact.saturating_mul(100).saturating_add(31) / 63;
    }
    if mass_grams == 0 || volume_cm3 == 0 {
        return Err(NicechunkBackpackError::InvalidForgeMaterialRequirements);
    }

    let brittleness_penalty = attributes[4].saturating_sub(55).saturating_mul(18);
    let weighted_material_score = attributes[1]
        .saturating_mul(30)
        .saturating_add(attributes[2].saturating_mul(25))
        .saturating_add(attributes[0].saturating_mul(20))
        .saturating_add(attributes[11].saturating_mul(15))
        .saturating_add(attributes[3].saturating_mul(10))
        .saturating_sub(brittleness_penalty);
    let material_score = weighted_material_score.saturating_add(50) / 100;
    let mass_requirement = mass_grams.saturating_mul(3).saturating_add(19) / 20;
    let volume_requirement = integer_sqrt(volume_cm3).saturating_mul(18);
    let attribute_requirement = material_score.saturating_mul(126).saturating_add(24) / 25;
    let requirements = ForgeMaterialRequirements {
        required_volume_mm3: volume_cm3.saturating_mul(1_000),
        required_effective_durability: mass_requirement
            .saturating_add(volume_requirement)
            .saturating_add(attribute_requirement)
            .max(1),
        output_mass_grams: mass_grams.min(u32::MAX as u64) as u32,
    };
    requirements.validate()?;
    Ok((fnv1a32(code), requirements))
}

fn read_bits(
    bytes: &[u8],
    bit_offset: &mut usize,
    bit_count: usize,
) -> Result<u32, NicechunkBackpackError> {
    if bit_count > 32 || bit_offset.saturating_add(bit_count) > bytes.len().saturating_mul(8) {
        return Err(NicechunkBackpackError::InvalidForgeMaterialRequirements);
    }
    let mut value = 0_u32;
    for _ in 0..bit_count {
        let byte = bytes[*bit_offset / 8];
        let bit = (byte >> (7 - (*bit_offset % 8))) & 1;
        value = (value << 1) | bit as u32;
        *bit_offset += 1;
    }
    Ok(value)
}

fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for byte in bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForgeMaterialCapacity {
    pub total_volume_mm3: u64,
    pub total_effective_durability: u64,
}

impl ForgeMaterialCapacity {
    pub fn satisfies(&self, requirements: &ForgeMaterialRequirements) -> bool {
        self.total_volume_mm3 >= requirements.required_volume_mm3
            && self.total_effective_durability >= requirements.required_effective_durability
    }
}

pub const PLAYER_PROFILE_LEN: usize = 773;
pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
pub const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;
pub const PLAYER_PROFILE_FORGING_XP_OFFSET: usize = 449;

pub const PLAYER_EQUIPMENT_MAGIC: [u8; 8] = *b"NCKEQP01";
pub const PLAYER_EQUIPMENT_VERSION: u16 = 1;
pub const PLAYER_EQUIPMENT_SEED: &[u8] = b"player-equipment-v1";
pub const PLAYER_EQUIPMENT_LEN: usize = 7_040;
pub const PLAYER_EQUIPMENT_OWNER_OFFSET: usize = 12;
pub const PLAYER_EQUIPMENT_SLOTS_OFFSET: usize = 128;
pub const PLAYER_EQUIPMENT_SLOT_LEN: usize = 768;
pub const PLAYER_EQUIPMENT_SLOT_COUNT: usize = 9;
pub const PLAYER_EQUIPMENT_RECORD_STATE_OFFSET: usize = 0;
pub const PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET: usize = 3;
pub const PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET: usize = 40;
pub const PLAYER_EQUIPMENT_FLAG_CUSTODY: u8 = 1 << 1;
pub const EQUIPMENT_TRANSFER_AUTHORITY_SEED: &[u8] = b"equipment-transfer-v1";

pub const PLAYER_SESSION_LEN: usize = 184;
pub const PLAYER_SESSION_MAGIC: [u8; 8] = *b"NCKSES01";
pub const PLAYER_SESSION_OWNER_OFFSET: usize = 12;
pub const PLAYER_SESSION_AUTHORITY_OFFSET: usize = 44;
pub const PLAYER_SESSION_PROFILE_OFFSET: usize = 76;
pub const PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET: usize = 142;
pub const PLAYER_SESSION_EXPIRES_AT_OFFSET: usize = 144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialPhysicsRecord {
    pub material_id: u16,
    pub density_kg_m3: u16,
}

pub struct MaterialPhysicsState {
    pub authority: Pubkey,
    pub global_config: Pubkey,
    pub revision: u32,
    pub record_count: u8,
}

impl MaterialPhysicsState {
    pub const LEN: usize = MATERIAL_PHYSICS_LEN;
    pub const AUTHORITY_OFFSET: usize = 12;
    pub const GLOBAL_CONFIG_OFFSET: usize = 44;
    pub const REVISION_OFFSET: usize = 76;
    pub const RECORD_COUNT_OFFSET: usize = 80;
    pub const CREATED_SLOT_OFFSET: usize = 84;
    pub const UPDATED_SLOT_OFFSET: usize = 92;
    pub const CREATED_AT_OFFSET: usize = 100;
    pub const RECORDS_OFFSET: usize = MATERIAL_PHYSICS_HEADER_LEN;

    pub fn pack_empty(
        dst: &mut [u8],
        bump: u8,
        authority: &Pubkey,
        global_config: &Pubkey,
        created_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN || *authority == Pubkey::default() {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&MATERIAL_PHYSICS_MAGIC);
        dst[8..10].copy_from_slice(&MATERIAL_PHYSICS_VERSION.to_le_bytes());
        dst[10] = bump;
        dst[11] = 1;
        dst[Self::AUTHORITY_OFFSET..Self::AUTHORITY_OFFSET + 32]
            .copy_from_slice(authority.as_ref());
        dst[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        dst[Self::CREATED_SLOT_OFFSET..Self::CREATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        dst[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        dst[Self::CREATED_AT_OFFSET..Self::CREATED_AT_OFFSET + 8]
            .copy_from_slice(&created_at.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8], global_config: &Pubkey) -> Result<Self, NicechunkBackpackError> {
        if data.len() != Self::LEN
            || data[0..8] != MATERIAL_PHYSICS_MAGIC
            || read_u16(data, 8) != MATERIAL_PHYSICS_VERSION
            || data[11] != 1
            || data[Self::RECORD_COUNT_OFFSET] as usize > MATERIAL_PHYSICS_MAX_RECORDS
            || &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
                != global_config.as_ref()
        {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
        }
        let state = Self {
            authority: read_pubkey(data, Self::AUTHORITY_OFFSET)?,
            global_config: read_pubkey(data, Self::GLOBAL_CONFIG_OFFSET)?,
            revision: read_u32(data, Self::REVISION_OFFSET),
            record_count: data[Self::RECORD_COUNT_OFFSET],
        };
        let mut previous = 0_u16;
        for index in 0..state.record_count as usize {
            let record = Self::record(data, index)?;
            if record.material_id == 0
                || record.density_kg_m3 == 0
                || (index > 0 && record.material_id <= previous)
            {
                return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
            }
            previous = record.material_id;
        }
        Ok(state)
    }

    pub fn replace_records(
        data: &mut [u8],
        global_config: &Pubkey,
        authority: &Pubkey,
        records: &[MaterialPhysicsRecord],
        updated_slot: u64,
    ) -> ProgramResult {
        let state = Self::validate(data, global_config)?;
        if records.is_empty() || records.len() > MATERIAL_PHYSICS_MAX_RECORDS {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRecord.into());
        }
        let mut previous = 0_u16;
        for (index, record) in records.iter().enumerate() {
            if record.material_id == 0
                || record.density_kg_m3 == 0
                || (index > 0 && record.material_id <= previous)
            {
                return Err(NicechunkBackpackError::InvalidMaterialPhysicsRecord.into());
            }
            previous = record.material_id;
        }
        let revision = state
            .revision
            .checked_add(1)
            .ok_or(NicechunkBackpackError::ArithmeticOverflow)?;

        data[Self::RECORDS_OFFSET..].fill(0);
        for (index, record) in records.iter().enumerate() {
            let offset = Self::RECORDS_OFFSET + index * MATERIAL_PHYSICS_RECORD_LEN;
            data[offset..offset + 2].copy_from_slice(&record.material_id.to_le_bytes());
            data[offset + 2..offset + 4].copy_from_slice(&record.density_kg_m3.to_le_bytes());
        }
        data[Self::RECORD_COUNT_OFFSET] = records.len() as u8;
        if &state.authority != authority {
            data[Self::AUTHORITY_OFFSET..Self::AUTHORITY_OFFSET + 32]
                .copy_from_slice(authority.as_ref());
        }
        data[Self::REVISION_OFFSET..Self::REVISION_OFFSET + 4]
            .copy_from_slice(&revision.to_le_bytes());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn density_kg_m3(
        data: &[u8],
        global_config: &Pubkey,
        material_id: u16,
    ) -> Result<u16, NicechunkBackpackError> {
        let state = Self::validate(data, global_config)?;
        let mut low = 0_usize;
        let mut high = state.record_count as usize;
        while low < high {
            let middle = low + (high - low) / 2;
            let record = Self::record(data, middle)?;
            match record.material_id.cmp(&material_id) {
                core::cmp::Ordering::Less => low = middle + 1,
                core::cmp::Ordering::Greater => high = middle,
                core::cmp::Ordering::Equal => return Ok(record.density_kg_m3),
            }
        }
        Err(NicechunkBackpackError::MissingMaterialPhysicsRecord)
    }

    pub fn mass_grams(
        data: &[u8],
        global_config: &Pubkey,
        material_id: u16,
        volume_mm3: u32,
    ) -> Result<u32, NicechunkBackpackError> {
        let density = Self::density_kg_m3(data, global_config, material_id)? as u64;
        let volume = volume_mm3.max(1) as u64;
        let mass = volume
            .checked_mul(density)
            .ok_or(NicechunkBackpackError::ArithmeticOverflow)?
            .checked_add(999_999)
            .ok_or(NicechunkBackpackError::ArithmeticOverflow)?
            / 1_000_000;
        u32::try_from(mass.max(1)).map_err(|_| NicechunkBackpackError::ArithmeticOverflow)
    }

    fn record(data: &[u8], index: usize) -> Result<MaterialPhysicsRecord, NicechunkBackpackError> {
        if index >= MATERIAL_PHYSICS_MAX_RECORDS {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRecord);
        }
        let offset = Self::RECORDS_OFFSET + index * MATERIAL_PHYSICS_RECORD_LEN;
        Ok(MaterialPhysicsRecord {
            material_id: read_u16(data, offset),
            density_kg_m3: read_u16(data, offset + 2),
        })
    }
}

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
    pub const TOTAL_MASS_GRAMS_OFFSET: usize = 90;
    pub const LAST_MINE_PRE_MASS_GRAMS_OFFSET: usize = 98;
    pub const LAST_MINE_ACTION_ID_OFFSET: usize = 106;
    pub const MINE_SEQUENCE_OFFSET: usize = 114;
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
        writer.u8(BACKPACK_FLAG_TOTAL_MASS_INITIALIZED)?;
        writer.i32(0)?;
        writer.i16(0)?;
        writer.i32(0)?;
        writer.u64(args.created_slot)?;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        writer.u64(0)?;
        writer.u64(0)?;
        writer.u64(0)?;
        writer.u64(0)?;
        writer.bytes(&[0_u8; 6])?;
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

    pub fn total_mass_grams(data: &[u8]) -> Result<u64, NicechunkBackpackError> {
        Self::validate(data)?;
        if data[Self::FLAGS_OFFSET] & BACKPACK_FLAG_TOTAL_MASS_INITIALIZED == 0 {
            return Err(NicechunkBackpackError::BackpackMassMigrationRequired);
        }
        Ok(read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET))
    }

    pub fn last_mine_pre_mass_grams(data: &[u8]) -> Result<u64, NicechunkBackpackError> {
        Self::total_mass_grams(data)?;
        Ok(read_u64(data, Self::LAST_MINE_PRE_MASS_GRAMS_OFFSET))
    }

    pub fn mine_sequence(data: &[u8]) -> Result<u64, NicechunkBackpackError> {
        Self::total_mass_grams(data)?;
        Ok(read_u64(data, Self::MINE_SEQUENCE_OFFSET))
    }

    pub fn migrate_mass(
        data: &mut [u8],
        owner: &Pubkey,
        physics_data: &[u8],
        global_config: &Pubkey,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        MaterialPhysicsState::validate(physics_data, global_config)?;
        if data[Self::FLAGS_OFFSET] & BACKPACK_FLAG_TOTAL_MASS_INITIALIZED != 0 {
            return Ok(());
        }
        let item_count = data[Self::ITEM_COUNT_OFFSET] as usize;
        let mut migrated_records = Vec::with_capacity(item_count);
        let mut total_mass = 0_u64;
        for index in 0..item_count {
            let offset = Self::RECORDS_OFFSET + index * BACKPACK_SLOT_RECORD_LEN;
            let mut record =
                BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
            if !record.has_valid_mass() {
                let mass = record.derived_mass_grams(physics_data, global_config)?;
                record.set_mass_grams(mass);
            }
            total_mass = total_mass
                .checked_add(record.mass_grams().unwrap_or(0) as u64)
                .ok_or(NicechunkBackpackError::ArithmeticOverflow)?;
            let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
            record.pack(&mut packed)?;
            migrated_records.push(packed);
        }
        for (index, packed) in migrated_records.iter().enumerate() {
            let offset = Self::RECORDS_OFFSET + index * BACKPACK_SLOT_RECORD_LEN;
            data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(packed);
        }
        data[Self::TOTAL_MASS_GRAMS_OFFSET..Self::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&total_mass.to_le_bytes());
        data[Self::FLAGS_OFFSET] |= BACKPACK_FLAG_TOTAL_MASS_INITIALIZED;
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn record_mining_action(
        data: &mut [u8],
        owner: &Pubkey,
        action_id: u64,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if action_id == 0 {
            return Err(NicechunkBackpackError::InvalidMiningAction.into());
        }
        let total_mass = Self::total_mass_grams(data)?;
        if read_u64(data, Self::LAST_MINE_ACTION_ID_OFFSET) != action_id {
            let next_sequence = read_u64(data, Self::MINE_SEQUENCE_OFFSET)
                .checked_add(1)
                .ok_or(NicechunkBackpackError::ArithmeticOverflow)?;
            data[Self::LAST_MINE_PRE_MASS_GRAMS_OFFSET..Self::LAST_MINE_PRE_MASS_GRAMS_OFFSET + 8]
                .copy_from_slice(&total_mass.to_le_bytes());
            data[Self::LAST_MINE_ACTION_ID_OFFSET..Self::LAST_MINE_ACTION_ID_OFFSET + 8]
                .copy_from_slice(&action_id.to_le_bytes());
            data[Self::MINE_SEQUENCE_OFFSET..Self::MINE_SEQUENCE_OFFSET + 8]
                .copy_from_slice(&next_sequence.to_le_bytes());
        }
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    fn mass_after_add(data: &[u8], mass_grams: u64) -> Result<u64, NicechunkBackpackError> {
        let current = Self::total_mass_grams(data)?;
        current
            .checked_add(mass_grams)
            .ok_or(NicechunkBackpackError::ArithmeticOverflow)
    }

    fn mass_after_subtract(data: &[u8], mass_grams: u64) -> Result<u64, NicechunkBackpackError> {
        let current = Self::total_mass_grams(data)?;
        current
            .checked_sub(mass_grams)
            .ok_or(NicechunkBackpackError::BackpackMassInvariantViolation)
    }

    fn write_total_mass(data: &mut [u8], total_mass_grams: u64) {
        data[Self::TOTAL_MASS_GRAMS_OFFSET..Self::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&total_mass_grams.to_le_bytes());
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
        Self::append_resource_with_volume_and_metadata(
            data,
            owner,
            record,
            volume_mm3,
            0,
            updated_slot,
        )
    }

    pub fn append_resource_with_volume_and_metadata(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackResourceRecord,
        volume_mm3: u32,
        metadata: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::append_resource_with_volume_metadata_and_mass(
            data,
            owner,
            record,
            volume_mm3,
            metadata,
            0,
            updated_slot,
        )
    }

    pub fn append_resource_with_volume_metadata_and_mass(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackResourceRecord,
        volume_mm3: u32,
        metadata: u32,
        mass_grams: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let capacity = data[Self::CAPACITY_OFFSET];
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count >= capacity {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        let slot = BackpackSlotRecord::from_block_resource_with_volume_metadata_and_mass(
            *record, volume_mm3, metadata, mass_grams,
        );
        let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        slot.pack(&mut packed)?;
        let next_mass = Self::mass_after_add(data, mass_grams as u64)?;

        data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(&packed);
        Self::write_total_mass(data, next_mass);
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
        Self::append_resources_lossy_with_volumes_and_metadata(
            data,
            owner,
            records,
            volumes_mm3,
            &[],
            updated_slot,
        )
    }

    pub fn append_resources_lossy_with_volumes_and_metadata(
        data: &mut [u8],
        owner: &Pubkey,
        records: &[BackpackResourceRecord],
        volumes_mm3: &[u32],
        metadata: &[u32],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::append_resources_lossy_with_physics(
            data,
            owner,
            records,
            volumes_mm3,
            metadata,
            &[],
            updated_slot,
        )
    }

    pub fn append_resources_lossy_with_physics(
        data: &mut [u8],
        owner: &Pubkey,
        records: &[BackpackResourceRecord],
        volumes_mm3: &[u32],
        metadata: &[u32],
        masses_grams: &[u32],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let capacity = data[Self::CAPACITY_OFFSET];
        let mut item_count = data[Self::ITEM_COUNT_OFFSET];
        if records.is_empty() || item_count >= capacity {
            return Ok(());
        }

        let accepted_count = records
            .len()
            .min(capacity.saturating_sub(item_count) as usize);
        let mut packed_records = Vec::with_capacity(accepted_count);
        let mut added_mass = 0_u64;
        for (index, record) in records.iter().take(accepted_count).enumerate() {
            let mass_grams = masses_grams.get(index).copied().unwrap_or(0);
            let slot = BackpackSlotRecord::from_block_resource_with_volume_metadata_and_mass(
                *record,
                volumes_mm3.get(index).copied().unwrap_or(0),
                metadata.get(index).copied().unwrap_or(0),
                mass_grams,
            );
            let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
            slot.pack(&mut packed)?;
            packed_records.push(packed);
            added_mass = added_mass
                .checked_add(mass_grams as u64)
                .ok_or(NicechunkBackpackError::ArithmeticOverflow)?;
        }
        let next_mass = Self::mass_after_add(data, added_mass)?;

        for packed in packed_records {
            let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
            data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(&packed);
            item_count = item_count.saturating_add(1);
        }
        Self::write_total_mass(data, next_mass);
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
        if !record.has_valid_mass() {
            return Err(NicechunkBackpackError::BackpackMassInvariantViolation.into());
        }
        let capacity = data[Self::CAPACITY_OFFSET];
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count >= capacity {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record.pack(&mut packed)?;
        let next_mass = Self::mass_after_add(data, record.mass_grams().unwrap_or(0) as u64)?;
        let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(&packed);
        Self::write_total_mass(data, next_mass);
        data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_add(1);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn append_issued_item(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackSlotRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if !record.has_valid_mass() {
            return Err(NicechunkBackpackError::BackpackMassInvariantViolation.into());
        }
        let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record.pack(&mut packed)?;

        let mut capacity = data[Self::CAPACITY_OFFSET];
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count >= capacity {
            if capacity >= BACKPACK_MAX_CAPACITY {
                return Err(NicechunkBackpackError::BackpackFull.into());
            }
            capacity = capacity.saturating_add(1);
        }
        let next_mass = Self::mass_after_add(data, record.mass_grams().unwrap_or(0) as u64)?;

        let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(&packed);
        data[Self::CAPACITY_OFFSET] = capacity;
        Self::write_total_mass(data, next_mass);
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

        let removed = Self::slot_at(data, index)?;
        let next_mass = Self::mass_after_subtract(data, removed.mass_grams().unwrap_or(0) as u64)?;

        let start = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        let end = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        if start + BACKPACK_SLOT_RECORD_LEN < end {
            data.copy_within(start + BACKPACK_SLOT_RECORD_LEN..end, start);
        }
        let last_start =
            Self::RECORDS_OFFSET + (item_count - 1) as usize * BACKPACK_SLOT_RECORD_LEN;
        data[last_start..last_start + BACKPACK_SLOT_RECORD_LEN].fill(0);
        Self::write_total_mass(data, next_mass);
        data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_sub(1);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn replace_slot_at(
        data: &mut [u8],
        owner: &Pubkey,
        index: u8,
        record: &BackpackSlotRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if index >= data[Self::ITEM_COUNT_OFFSET] {
            return Err(NicechunkBackpackError::InvalidResourceIndex.into());
        }
        if !record.has_valid_mass() {
            return Err(NicechunkBackpackError::BackpackMassInvariantViolation.into());
        }
        let previous = Self::slot_at(data, index)?;
        let current_mass = Self::total_mass_grams(data)?;
        let next_mass = current_mass
            .checked_sub(previous.mass_grams().unwrap_or(0) as u64)
            .and_then(|mass| mass.checked_add(record.mass_grams().unwrap_or(0) as u64))
            .ok_or(NicechunkBackpackError::BackpackMassInvariantViolation)?;
        let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record.pack(&mut packed)?;
        let offset = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(&packed);
        Self::write_total_mass(data, next_mass);
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

        let mut removed_mass = 0_u64;
        for (index, is_selected) in selected.iter().enumerate().take(item_count as usize) {
            if *is_selected {
                removed_mass = removed_mass
                    .checked_add(Self::slot_at(data, index as u8)?.mass_grams().unwrap_or(0) as u64)
                    .ok_or(NicechunkBackpackError::ArithmeticOverflow)?;
            }
        }
        let next_mass = Self::mass_after_subtract(data, removed_mass)?;

        let mut write_index = 0_usize;
        for read_index in 0..item_count as usize {
            if selected[read_index] {
                continue;
            }
            if write_index != read_index {
                let source = Self::RECORDS_OFFSET + read_index * BACKPACK_SLOT_RECORD_LEN;
                let target = Self::RECORDS_OFFSET + write_index * BACKPACK_SLOT_RECORD_LEN;
                data.copy_within(source..source + BACKPACK_SLOT_RECORD_LEN, target);
            }
            write_index += 1;
        }
        let clear_start = Self::RECORDS_OFFSET + write_index * BACKPACK_SLOT_RECORD_LEN;
        let clear_end = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        data[clear_start..clear_end].fill(0);
        data[Self::ITEM_COUNT_OFFSET] = write_index as u8;
        Self::write_total_mass(data, next_mass);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
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
        design_hash: u32,
        item_pda: &Pubkey,
        forging_level: u8,
        updated_slot: u64,
    ) -> Result<ForgeOutcome, solana_program::program_error::ProgramError> {
        Self::forge_equipment_from_materials_internal(
            data,
            owner,
            indexes,
            item_id,
            design_hash,
            item_pda,
            forging_level,
            updated_slot,
            None,
        )
    }

    pub fn forge_equipment_from_verified_materials(
        data: &mut [u8],
        owner: &Pubkey,
        indexes: &[u8],
        item_id: u64,
        design_hash: u32,
        item_pda: &Pubkey,
        forging_level: u8,
        updated_slot: u64,
        requirements: ForgeMaterialRequirements,
    ) -> Result<ForgeOutcome, solana_program::program_error::ProgramError> {
        requirements.validate()?;
        Self::forge_equipment_from_materials_internal(
            data,
            owner,
            indexes,
            item_id,
            design_hash,
            item_pda,
            forging_level,
            updated_slot,
            Some(requirements),
        )
    }

    fn forge_equipment_from_materials_internal(
        data: &mut [u8],
        owner: &Pubkey,
        indexes: &[u8],
        item_id: u64,
        design_hash: u32,
        item_pda: &Pubkey,
        forging_level: u8,
        updated_slot: u64,
        requirements: Option<ForgeMaterialRequirements>,
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
            let slot = Self::slot_at(data, *index)?;
            if slot.kind != BACKPACK_SLOT_KIND_ITEM
                || slot.category != BACKPACK_ITEM_CATEGORY_MATERIAL
                || slot.item_code == 0
            {
                return Err(NicechunkBackpackError::InvalidForgingMaterial.into());
            }
            if slot.durability_max == 0 || slot.durability_current == 0 {
                return Err(NicechunkBackpackError::InvalidForgingMaterial.into());
            }
            materials.push(slot);
        }

        if let Some(required) = requirements {
            let capacity = forge_material_capacity(&materials);
            if !capacity.satisfies(&required) {
                return Err(NicechunkBackpackError::InsufficientForgeMaterialParameters.into());
            }
        }

        let mut outcome = calculate_forge_outcome(&materials, forging_level);
        outcome.mass_grams = requirements
            .map(|required| required.output_mass_grams)
            .unwrap_or_else(|| {
                materials
                    .iter()
                    .fold(0_u64, |total, material| {
                        total.saturating_add(material.mass_grams().unwrap_or(0) as u64)
                    })
                    .min(u32::MAX as u64) as u32
            });
        Self::remove_resources_at(data, owner, indexes, updated_slot)?;
        let mut output = BackpackSlotRecord {
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
            metadata: design_hash,
        };
        output.set_mass_grams(outcome.mass_grams);
        Self::append_item(data, owner, &output, updated_slot)?;
        Ok(outcome)
    }
}

pub struct BlueprintItemAccount;

impl BlueprintItemAccount {
    pub const LEN: usize = 96;
    pub const ITEM_ID_OFFSET: usize = 12;
    pub const OWNER_OFFSET: usize = 20;
    pub const ISSUER_OFFSET: usize = 52;
    pub const CREATED_SLOT_OFFSET: usize = 84;

    pub fn pack(
        dst: &mut [u8],
        bump: u8,
        item_id: u64,
        owner: &Pubkey,
        issuer: &Pubkey,
        created_slot: u64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN || item_id == 0 {
            return Err(NicechunkBackpackError::InvalidBlueprintItem.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&BLUEPRINT_ITEM_MAGIC);
        dst[8..10].copy_from_slice(&BLUEPRINT_ITEM_VERSION.to_le_bytes());
        dst[10] = bump;
        dst[11] = 1;
        dst[Self::ITEM_ID_OFFSET..Self::ITEM_ID_OFFSET + 8].copy_from_slice(&item_id.to_le_bytes());
        dst[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32].copy_from_slice(owner.as_ref());
        dst[Self::ISSUER_OFFSET..Self::ISSUER_OFFSET + 32].copy_from_slice(issuer.as_ref());
        dst[Self::CREATED_SLOT_OFFSET..Self::CREATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8]) -> Result<(), NicechunkBackpackError> {
        if data.len() != Self::LEN
            || data[0..8] != BLUEPRINT_ITEM_MAGIC
            || read_u16(data, 8) != BLUEPRINT_ITEM_VERSION
            || data[11] != 1
            || read_u64(data, Self::ITEM_ID_OFFSET) == 0
        {
            return Err(NicechunkBackpackError::InvalidBlueprintItem);
        }
        Ok(())
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
        Self::from_block_resource_with_volume_and_metadata(resource, volume_mm3, 0)
    }

    pub fn from_block_resource_with_volume_and_metadata(
        resource: BackpackResourceRecord,
        volume_mm3: u32,
        metadata: u32,
    ) -> Self {
        Self::from_block_resource_with_volume_metadata_and_mass(resource, volume_mm3, metadata, 0)
    }

    pub fn from_block_resource_with_volume_metadata_and_mass(
        resource: BackpackResourceRecord,
        volume_mm3: u32,
        metadata: u32,
        mass_grams: u32,
    ) -> Self {
        let mut record = Self {
            kind: BACKPACK_SLOT_KIND_BLOCK,
            category: 0,
            flags: BACKPACK_ITEM_FLAG_MASS_VALID,
            quantity: 1,
            resource,
            item_code: 0,
            item_id: 0,
            item_pda: Pubkey::default(),
            volume_mm3,
            durability_current: mass_grams,
            durability_max: 0,
            grade: 0,
            item_level: 0,
            quality_bps: 0,
            metadata,
        };
        record.set_mass_grams(mass_grams);
        record
    }

    pub fn has_valid_mass(&self) -> bool {
        self.flags & BACKPACK_ITEM_FLAG_MASS_VALID != 0
    }

    pub fn mass_grams(&self) -> Option<u32> {
        if !self.has_valid_mass() {
            return None;
        }
        Some(if self.kind == BACKPACK_SLOT_KIND_BLOCK {
            self.durability_current
        } else {
            self.resource.world_x as u32
        })
    }

    pub fn set_mass_grams(&mut self, mass_grams: u32) {
        self.flags |= BACKPACK_ITEM_FLAG_MASS_VALID;
        if self.kind == BACKPACK_SLOT_KIND_BLOCK {
            self.durability_current = mass_grams;
        } else {
            self.resource.world_x = mass_grams as i32;
        }
    }

    pub fn derived_mass_grams(
        &self,
        physics_data: &[u8],
        global_config: &Pubkey,
    ) -> Result<u32, NicechunkBackpackError> {
        if self.kind == BACKPACK_SLOT_KIND_BLOCK {
            let packed_y = self.resource.world_y;
            if packed_y < 0 {
                return Err(NicechunkBackpackError::MissingMaterialPhysicsRecord);
            }
            let material_id = (packed_y as u16) >> BACKPACK_PACKED_Y_BITS;
            let volume = if self.volume_mm3 == 0 {
                BACKPACK_DEFAULT_RESOURCE_VOLUME_MM3
            } else {
                self.volume_mm3
            };
            return MaterialPhysicsState::mass_grams(
                physics_data,
                global_config,
                material_id,
                volume,
            );
        }
        match self.category {
            BACKPACK_ITEM_CATEGORY_MATERIAL => MaterialPhysicsState::mass_grams(
                physics_data,
                global_config,
                self.item_code,
                self.volume_mm3,
            ),
            BACKPACK_ITEM_CATEGORY_FORGED => MaterialPhysicsState::mass_grams(
                physics_data,
                global_config,
                LEGACY_FORGED_MATERIAL_ID,
                self.volume_mm3,
            ),
            BACKPACK_ITEM_CATEGORY_BLUEPRINT => Ok(0),
            _ => Ok(0),
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
        if record.kind == BACKPACK_SLOT_KIND_ITEM {
            record.validate_item_metadata()?;
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
        if self.kind == BACKPACK_SLOT_KIND_ITEM {
            self.validate_item_metadata()?;
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

    fn validate_item_metadata(&self) -> Result<(), NicechunkBackpackError> {
        if self.volume_mm3 == 0
            || self.durability_current == 0
            || self.durability_max == 0
            || self.durability_current > self.durability_max
            || self.grade == 0
            || self.grade > 10
            || self.item_level == 0
            || self.item_level > 100
            || self.quality_bps == 0
            || self.quality_bps > DURABILITY_BPS_DENOMINATOR as u16
        {
            return Err(NicechunkBackpackError::InvalidInventoryItem);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForgeOutcome {
    pub grade: u8,
    pub item_level: u8,
    pub durability_max: u32,
    pub quality_bps: u16,
    pub volume_mm3: u32,
    pub mass_grams: u32,
    pub gained_xp: u64,
}

pub fn forge_material_capacity(materials: &[BackpackSlotRecord]) -> ForgeMaterialCapacity {
    let mut capacity = ForgeMaterialCapacity::default();
    for material in materials {
        let quality_bps = material
            .quality_bps
            .max(1)
            .min(DURABILITY_BPS_DENOMINATOR as u16) as u64;
        let durability_current = material
            .durability_current
            .min(material.durability_max.max(1)) as u64;
        capacity.total_volume_mm3 = capacity
            .total_volume_mm3
            .saturating_add(material.volume_mm3 as u64);
        capacity.total_effective_durability = capacity.total_effective_durability.saturating_add(
            durability_current.saturating_mul(quality_bps) / DURABILITY_BPS_DENOMINATOR,
        );
    }
    capacity
}

fn calculate_forge_outcome(materials: &[BackpackSlotRecord], forging_level: u8) -> ForgeOutcome {
    let mut total_volume = 0_u64;
    let mut total_raw_durability = 0_u64;
    let mut total_effective_durability = 0_u64;
    let mut weighted_grade = 0_u64;
    let mut weighted_quality = 0_u64;
    let mut weak_grade_cap = 10_u8;

    for material in materials {
        let volume = material.volume_mm3 as u64;
        let grade = material.grade.max(1).min(10);
        let quality = material
            .quality_bps
            .max(1)
            .min(DURABILITY_BPS_DENOMINATOR as u16) as u64;
        let max_durability = material.durability_max.max(1) as u64;
        let current_durability = material
            .durability_current
            .min(material.durability_max.max(1)) as u64;
        total_volume = total_volume.saturating_add(volume);
        total_raw_durability = total_raw_durability.saturating_add(max_durability);
        total_effective_durability = total_effective_durability.saturating_add(
            current_durability.saturating_mul(quality) / DURABILITY_BPS_DENOMINATOR,
        );
        weighted_grade = weighted_grade.saturating_add(grade as u64 * volume);
        weighted_quality = weighted_quality.saturating_add(quality * volume);
    }

    for material in materials {
        let volume = material.volume_mm3 as u64;
        if total_volume > 0 && volume.saturating_mul(5) >= total_volume {
            weak_grade_cap =
                weak_grade_cap.min(material.grade.max(1).min(10).saturating_add(2).min(10));
        }
    }

    let material_grade = if total_volume > 0 {
        ((weighted_grade + total_volume / 2) / total_volume) as u8
    } else {
        1
    }
    .max(1)
    .min(10);
    let quality_bps = (((weighted_quality + total_volume / 2) / total_volume) as u16)
        .max(1)
        .min(DURABILITY_BPS_DENOMINATOR as u16);
    let material_level =
        material_item_level_from_durability(total_effective_durability, total_volume);
    let item_level = material_level.max(1);
    let item_level_grade = 1_u8.saturating_add((item_level.saturating_sub(1) / 10).min(9));
    let skill_grade = 1_u8.saturating_add(forging_level.min(9));
    let skill_cap = 3_u8.saturating_add(forging_level.min(7)).min(10);
    let blended_grade =
        ((material_grade as u16 * 2 + item_level_grade as u16 + skill_grade as u16 + 2) / 4) as u8;
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
        .saturating_mul(25);

    ForgeOutcome {
        grade,
        item_level,
        durability_max,
        quality_bps,
        volume_mm3: total_volume.max(1).min(u32::MAX as u64) as u32,
        mass_grams: 0,
        gained_xp,
    }
}

fn material_item_level_from_durability(effective_durability: u64, total_volume_mm3: u64) -> u8 {
    let durability_level = integer_sqrt(effective_durability / 25).min(80);
    let volume_level = (total_volume_mm3 / 500_000).min(20);
    (1_u64
        .saturating_add(durability_level)
        .saturating_add(volume_level))
    .min(100) as u8
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

pub struct PlayerEquipmentView;

impl PlayerEquipmentView {
    pub fn validate(
        data: &[u8],
        equipment: &Pubkey,
        owner: &Pubkey,
    ) -> Result<(), NicechunkBackpackError> {
        if data.len() != PLAYER_EQUIPMENT_LEN
            || data[0..8] != PLAYER_EQUIPMENT_MAGIC
            || read_u16(data, 8) != PLAYER_EQUIPMENT_VERSION
            || data[108] as usize != PLAYER_EQUIPMENT_SLOT_COUNT
            || &data[PLAYER_EQUIPMENT_OWNER_OFFSET..PLAYER_EQUIPMENT_OWNER_OFFSET + 32]
                != owner.as_ref()
        {
            return Err(NicechunkBackpackError::InvalidPlayerEquipment);
        }
        let (expected, _) = Pubkey::find_program_address(
            &[PLAYER_EQUIPMENT_SEED, owner.as_ref()],
            &crate::cluster_config::NICECHUNK_PLAYER_PROGRAM_ID,
        );
        if &expected != equipment {
            return Err(NicechunkBackpackError::InvalidPlayerEquipment);
        }
        Ok(())
    }

    pub fn custodied_slot(
        data: &[u8],
        slot: u8,
    ) -> Result<Option<BackpackSlotRecord>, NicechunkBackpackError> {
        if slot as usize >= PLAYER_EQUIPMENT_SLOT_COUNT {
            return Err(NicechunkBackpackError::InvalidEquipmentSlot);
        }
        let offset = PLAYER_EQUIPMENT_SLOTS_OFFSET + slot as usize * PLAYER_EQUIPMENT_SLOT_LEN;
        if data[offset + PLAYER_EQUIPMENT_RECORD_STATE_OFFSET] != 1
            || data[offset + PLAYER_EQUIPMENT_RECORD_FLAGS_OFFSET] & PLAYER_EQUIPMENT_FLAG_CUSTODY
                == 0
        {
            return Ok(None);
        }
        BackpackSlotRecord::unpack(
            &data[offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET
                ..offset + PLAYER_EQUIPMENT_RECORD_BACKPACK_SLOT_OFFSET + BACKPACK_SLOT_RECORD_LEN],
        )
        .map(Some)
    }
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
        Ok(forging_level_from_xp(read_u64(
            data,
            PLAYER_PROFILE_FORGING_XP_OFFSET,
        )))
    }

    pub fn owner_and_global_config(
        data: &[u8],
    ) -> Result<(Pubkey, Pubkey), NicechunkBackpackError> {
        if data.len() != PLAYER_PROFILE_LEN || data[0..8] != PLAYER_PROFILE_MAGIC {
            return Err(NicechunkBackpackError::InvalidPlayerProfile);
        }
        Ok((
            Pubkey::new_from_array(
                data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
                    .try_into()
                    .map_err(|_| NicechunkBackpackError::InvalidPlayerProfile)?,
            ),
            Pubkey::new_from_array(
                data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
                    .try_into()
                    .map_err(|_| NicechunkBackpackError::InvalidPlayerProfile)?,
            ),
        ))
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

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey, NicechunkBackpackError> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(NicechunkBackpackError::InvalidMaterialPhysicsData)?
        .try_into()
        .map_err(|_| NicechunkBackpackError::InvalidMaterialPhysicsData)?;
    Ok(Pubkey::new_from_array(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::program_error::ProgramError;

    fn empty_backpack(owner: &Pubkey, capacity: u8) -> Vec<u8> {
        let mut data = vec![0_u8; BackpackAccount::LEN];
        BackpackAccount::pack_empty(
            &mut data,
            &BackpackInitArgs {
                bump: 251,
                backpack_id: 7,
                owner,
                capacity,
                created_slot: 10,
                created_at: 20,
            },
        )
        .unwrap();
        data
    }

    fn material_slot(durability_current: u32, durability_max: u32) -> BackpackSlotRecord {
        let mut record = BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_ITEM,
            category: BACKPACK_ITEM_CATEGORY_MATERIAL,
            flags: 0,
            quantity: 1,
            resource: BackpackResourceRecord::default(),
            item_code: 1008,
            item_id: 88,
            item_pda: Pubkey::new_unique(),
            volume_mm3: 600_000,
            durability_current,
            durability_max,
            grade: 4,
            item_level: 24,
            quality_bps: 7_000,
            metadata: 0,
        };
        record.set_mass_grams(1_620);
        record
    }

    fn blueprint_slot(item_id: u64) -> BackpackSlotRecord {
        let mut record = BackpackSlotRecord {
            kind: BACKPACK_SLOT_KIND_ITEM,
            category: BACKPACK_ITEM_CATEGORY_BLUEPRINT,
            flags: BACKPACK_ITEM_FLAG_UNIQUE,
            quantity: 1,
            resource: BackpackResourceRecord::default(),
            item_code: BACKPACK_BLUEPRINT_ITEM_CODE,
            item_id,
            item_pda: Pubkey::new_unique(),
            volume_mm3: 1,
            durability_current: 1,
            durability_max: 1,
            grade: 1,
            item_level: 1,
            quality_bps: 10_000,
            metadata: 0,
        };
        record.set_mass_grams(0);
        record
    }

    fn material_physics(global_config: &Pubkey, records: &[MaterialPhysicsRecord]) -> Vec<u8> {
        let authority = Pubkey::new_unique();
        let mut data = vec![0_u8; MaterialPhysicsState::LEN];
        MaterialPhysicsState::pack_empty(&mut data, 250, &authority, global_config, 10, 20)
            .unwrap();
        MaterialPhysicsState::replace_records(&mut data, global_config, &authority, records, 11)
            .unwrap();
        data
    }

    fn packed_slot(record: &BackpackSlotRecord) -> [u8; BACKPACK_SLOT_RECORD_LEN] {
        let mut data = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record.pack(&mut data).unwrap();
        data
    }

    fn forge_single_material(durability_current: u32, durability_max: u32) -> ForgeOutcome {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        let material = material_slot(durability_current, durability_max);
        BackpackAccount::append_item(&mut data, &owner, &material, 11).unwrap();
        BackpackAccount::forge_equipment_from_materials(
            &mut data,
            &owner,
            &[0],
            901,
            0x7a1d_c0de,
            &Pubkey::new_unique(),
            3,
            12,
        )
        .unwrap()
    }

    #[test]
    fn append_item_rejects_zero_integrity_material() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        let material = material_slot(0, 1_200);
        let error = BackpackAccount::append_item(&mut data, &owner, &material, 11).unwrap_err();

        assert!(matches!(
            error,
            ProgramError::Custom(code) if code == NicechunkBackpackError::InvalidInventoryItem as u32
        ));
    }

    #[test]
    fn issued_blueprint_expands_a_full_backpack_without_removing_items() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();

        BackpackAccount::append_issued_item(&mut data, &owner, &blueprint_slot(901), 12).unwrap();

        assert_eq!(data[BackpackAccount::CAPACITY_OFFSET], 2);
        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 2);
        assert_eq!(
            BackpackAccount::slot_at(&data, 0).unwrap().category,
            BACKPACK_ITEM_CATEGORY_MATERIAL
        );
        let blueprint = BackpackAccount::slot_at(&data, 1).unwrap();
        assert_eq!(blueprint.category, BACKPACK_ITEM_CATEGORY_BLUEPRINT);
        assert_eq!(blueprint.item_code, BACKPACK_BLUEPRINT_ITEM_CODE);
        assert_eq!(blueprint.item_id, 901);
    }

    #[test]
    fn equipment_transfer_removal_frees_backpack_capacity() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 2);
        let first = material_slot(1_200, 1_200);
        let second = blueprint_slot(901);
        BackpackAccount::append_item(&mut data, &owner, &first, 11).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &second, 12).unwrap();

        BackpackAccount::remove_resource_at(&mut data, &owner, 0, 13).unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 1);
        assert_eq!(
            packed_slot(&BackpackAccount::slot_at(&data, 0).unwrap()),
            packed_slot(&second)
        );
        let replacement = blueprint_slot(902);
        BackpackAccount::append_item(&mut data, &owner, &replacement, 14).unwrap();
        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 2);
        assert_eq!(
            packed_slot(&BackpackAccount::slot_at(&data, 1).unwrap()),
            packed_slot(&replacement)
        );
    }

    #[test]
    fn equipment_replacement_returns_previous_item_into_the_incoming_slot() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);
        let incoming = blueprint_slot(901);
        let previous_equipment = blueprint_slot(902);
        BackpackAccount::append_item(&mut data, &owner, &incoming, 11).unwrap();

        BackpackAccount::replace_slot_at(&mut data, &owner, 0, &previous_equipment, 12).unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 1);
        assert_eq!(
            packed_slot(&BackpackAccount::slot_at(&data, 0).unwrap()),
            packed_slot(&previous_equipment)
        );
    }

    #[test]
    fn full_backpack_rejects_unequip_without_mutating_inventory() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();
        let before = data.clone();

        let error =
            BackpackAccount::append_item(&mut data, &owner, &blueprint_slot(901), 12).unwrap_err();

        assert!(matches!(
            error,
            ProgramError::Custom(code) if code == NicechunkBackpackError::BackpackFull as u32
        ));
        assert_eq!(data, before);
    }

    #[test]
    fn blueprint_item_account_keeps_global_identity_and_owner() {
        let owner = Pubkey::new_unique();
        let issuer = Pubkey::new_unique();
        let mut data = vec![0_u8; BlueprintItemAccount::LEN];

        BlueprintItemAccount::pack(&mut data, 252, 902, &owner, &issuer, 77).unwrap();

        BlueprintItemAccount::validate(&data).unwrap();
        assert_eq!(read_u64(&data, BlueprintItemAccount::ITEM_ID_OFFSET), 902);
        assert_eq!(
            &data[BlueprintItemAccount::OWNER_OFFSET..BlueprintItemAccount::OWNER_OFFSET + 32],
            owner.as_ref()
        );
        assert_eq!(
            &data[BlueprintItemAccount::ISSUER_OFFSET..BlueprintItemAccount::ISSUER_OFFSET + 32],
            issuer.as_ref()
        );
    }

    #[test]
    fn block_resource_preserves_generic_decoration_metadata() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        let record = BackpackResourceRecord {
            world_x: 590,
            world_y: 14_472,
            world_z: 302,
        };

        BackpackAccount::append_resource_with_volume_and_metadata(
            &mut data,
            &owner,
            &record,
            1_000_000,
            0x0001_0002,
            11,
        )
        .unwrap();

        let slot = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(slot.resource.world_x, record.world_x);
        assert_eq!(slot.resource.world_y, record.world_y);
        assert_eq!(slot.resource.world_z, record.world_z);
        assert_eq!(slot.volume_mm3, 1_000_000);
        assert_eq!(slot.metadata, 0x0001_0002);
    }

    #[test]
    fn worn_material_contributes_less_than_full_integrity_material() {
        let full = forge_single_material(1_200, 1_200);
        let worn = forge_single_material(600, 1_200);

        assert!(worn.durability_max < full.durability_max);
        assert!(worn.item_level <= full.item_level);
    }

    #[test]
    fn forge_persists_design_hash_in_output_metadata() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();

        BackpackAccount::forge_equipment_from_materials(
            &mut data,
            &owner,
            &[0],
            901,
            0x1234_abcd,
            &Pubkey::new_unique(),
            3,
            12,
        )
        .unwrap();

        let output = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(output.category, BACKPACK_ITEM_CATEGORY_FORGED);
        assert_eq!(output.metadata, 0x1234_abcd);
    }

    #[test]
    fn forged_item_level_comes_from_material_integrity() {
        let strong = forge_single_material(8_000, 8_000);
        let weak = forge_single_material(800, 800);

        assert!(strong.item_level > weak.item_level);
    }

    #[test]
    fn forging_xp_depends_only_on_final_grade() {
        let outcome = forge_single_material(1_200, 1_200);

        assert_eq!(
            outcome.gained_xp,
            outcome.grade as u64 * outcome.grade as u64 * 25
        );
    }

    #[test]
    fn verified_forge_rejects_any_material_parameter_deficit_without_consuming_slots() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();

        let error = BackpackAccount::forge_equipment_from_verified_materials(
            &mut data,
            &owner,
            &[0],
            901,
            0x1234_abcd,
            &Pubkey::new_unique(),
            3,
            12,
            ForgeMaterialRequirements {
                required_volume_mm3: 600_001,
                required_effective_durability: 840,
                output_mass_grams: 1_000,
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ProgramError::Custom(code)
                if code == NicechunkBackpackError::InsufficientForgeMaterialParameters as u32
        ));
        assert_eq!(
            BackpackAccount::slot_at(&data, 0).unwrap().category,
            BACKPACK_ITEM_CATEGORY_MATERIAL
        );
    }

    #[test]
    fn verified_forge_accepts_material_parameters_equal_to_or_above_requirements() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();

        BackpackAccount::forge_equipment_from_verified_materials(
            &mut data,
            &owner,
            &[0],
            901,
            0x1234_abcd,
            &Pubkey::new_unique(),
            3,
            12,
            ForgeMaterialRequirements {
                required_volume_mm3: 600_000,
                required_effective_durability: 840,
                output_mass_grams: 1_000,
            },
        )
        .unwrap();

        assert_eq!(
            BackpackAccount::slot_at(&data, 0).unwrap().category,
            BACKPACK_ITEM_CATEGORY_FORGED
        );
    }

    #[test]
    fn verified_forge_rejects_a_durability_deficit_even_when_volume_is_exact() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();

        let error = BackpackAccount::forge_equipment_from_verified_materials(
            &mut data,
            &owner,
            &[0],
            901,
            0x1234_abcd,
            &Pubkey::new_unique(),
            3,
            12,
            ForgeMaterialRequirements {
                required_volume_mm3: 600_000,
                required_effective_durability: 841,
                output_mass_grams: 1_000,
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ProgramError::Custom(code)
                if code == NicechunkBackpackError::InsufficientForgeMaterialParameters as u32
        ));
    }

    #[test]
    fn verified_ncf1_header_derives_chain_requirements_and_raw_design_hash() {
        let code = hex_bytes("e09600bb8b2cb2cb2cb2cb2cb2c000");
        let (design_hash, requirements) = verified_forge_design(&code).unwrap();

        assert_eq!(design_hash, 0x5c09_3cc3);
        assert_eq!(requirements.required_volume_mm3, 3_000_000);
        assert_eq!(requirements.required_effective_durability, 3_110);
        assert!(verified_forge_design(&code[..13]).is_err());
    }

    #[test]
    fn material_physics_uses_sorted_lookup_and_ceil_rounding() {
        let global_config = Pubkey::new_unique();
        let data = material_physics(
            &global_config,
            &[
                MaterialPhysicsRecord {
                    material_id: 1,
                    density_kg_m3: 1_000,
                },
                MaterialPhysicsRecord {
                    material_id: 1008,
                    density_kg_m3: 2_700,
                },
            ],
        );

        assert_eq!(
            MaterialPhysicsState::density_kg_m3(&data, &global_config, 1008),
            Ok(2_700)
        );
        assert_eq!(
            MaterialPhysicsState::mass_grams(&data, &global_config, 1, 1_000_000),
            Ok(1_000)
        );
        assert_eq!(
            MaterialPhysicsState::mass_grams(&data, &global_config, 1008, 1),
            Ok(1)
        );
        assert_eq!(
            MaterialPhysicsState::density_kg_m3(&data, &global_config, 44),
            Err(NicechunkBackpackError::MissingMaterialPhysicsRecord)
        );
    }

    #[test]
    fn material_physics_rejects_unsorted_or_duplicate_records_without_mutation() {
        let global_config = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mut data = vec![0_u8; MaterialPhysicsState::LEN];
        MaterialPhysicsState::pack_empty(&mut data, 250, &authority, &global_config, 10, 20)
            .unwrap();
        let before = data.clone();

        let error = MaterialPhysicsState::replace_records(
            &mut data,
            &global_config,
            &authority,
            &[
                MaterialPhysicsRecord {
                    material_id: 2,
                    density_kg_m3: 1_000,
                },
                MaterialPhysicsRecord {
                    material_id: 2,
                    density_kg_m3: 2_000,
                },
            ],
            11,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ProgramError::Custom(code)
                if code == NicechunkBackpackError::InvalidMaterialPhysicsRecord as u32
        ));
        assert_eq!(data, before);
    }

    #[test]
    fn material_physics_update_adopts_the_current_treasury_authority() {
        let global_config = Pubkey::new_unique();
        let previous_authority = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        let mut data = vec![0_u8; MaterialPhysicsState::LEN];
        MaterialPhysicsState::pack_empty(
            &mut data,
            250,
            &previous_authority,
            &global_config,
            10,
            20,
        )
        .unwrap();

        MaterialPhysicsState::replace_records(
            &mut data,
            &global_config,
            &current_authority,
            &[MaterialPhysicsRecord {
                material_id: 1,
                density_kg_m3: 1_200,
            }],
            11,
        )
        .unwrap();

        let state = MaterialPhysicsState::validate(&data, &global_config).unwrap();
        assert_eq!(state.authority, current_authority);
        assert_eq!(state.revision, 1);
    }

    #[test]
    fn legacy_backpack_mass_migration_is_exact_and_atomic() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let physics = material_physics(
            &global_config,
            &[
                MaterialPhysicsRecord {
                    material_id: 1,
                    density_kg_m3: 1_000,
                },
                MaterialPhysicsRecord {
                    material_id: 1008,
                    density_kg_m3: 2_700,
                },
            ],
        );
        let mut data = empty_backpack(&owner, 4);
        BackpackAccount::append_resource_with_volume_metadata_and_mass(
            &mut data,
            &owner,
            &BackpackResourceRecord {
                world_x: 4,
                world_y: (1_u16 << BACKPACK_PACKED_Y_BITS) as i16,
                world_z: 5,
            },
            1_000_000,
            0,
            1_000,
            11,
        )
        .unwrap();
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 12).unwrap();
        for index in 0..2 {
            let offset = BackpackAccount::RECORDS_OFFSET + index * BACKPACK_SLOT_RECORD_LEN + 2;
            let flags = read_u16(&data, offset) & !BACKPACK_ITEM_FLAG_MASS_VALID;
            data[offset..offset + 2].copy_from_slice(&flags.to_le_bytes());
        }
        data[BackpackAccount::FLAGS_OFFSET] &= !BACKPACK_FLAG_TOTAL_MASS_INITIALIZED;
        data[BackpackAccount::TOTAL_MASS_GRAMS_OFFSET
            ..BackpackAccount::TOTAL_MASS_GRAMS_OFFSET + 8]
            .fill(0);

        BackpackAccount::migrate_mass(&mut data, &owner, &physics, &global_config, 13).unwrap();

        assert_eq!(BackpackAccount::total_mass_grams(&data), Ok(2_620));
        assert_eq!(
            BackpackAccount::slot_at(&data, 0).unwrap().mass_grams(),
            Some(1_000)
        );
        assert_eq!(
            BackpackAccount::slot_at(&data, 1).unwrap().mass_grams(),
            Some(1_620)
        );
    }

    #[test]
    fn failed_mass_migration_leaves_legacy_backpack_unchanged() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let physics = material_physics(
            &global_config,
            &[MaterialPhysicsRecord {
                material_id: 1,
                density_kg_m3: 1_000,
            }],
        );
        let mut data = empty_backpack(&owner, 2);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();
        let flags_offset = BackpackAccount::RECORDS_OFFSET + 2;
        let flags = read_u16(&data, flags_offset) & !BACKPACK_ITEM_FLAG_MASS_VALID;
        data[flags_offset..flags_offset + 2].copy_from_slice(&flags.to_le_bytes());
        data[BackpackAccount::FLAGS_OFFSET] &= !BACKPACK_FLAG_TOTAL_MASS_INITIALIZED;
        let before = data.clone();

        assert!(
            BackpackAccount::migrate_mass(&mut data, &owner, &physics, &global_config, 12).is_err()
        );
        assert_eq!(data, before);
    }

    #[test]
    fn append_replace_remove_and_lossy_batch_keep_total_mass_exact() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 3);
        let block = BackpackResourceRecord {
            world_x: 1,
            world_y: 2,
            world_z: 3,
        };
        BackpackAccount::append_resources_lossy_with_physics(
            &mut data,
            &owner,
            &[block, block, block, block],
            &[1, 1, 1, 1],
            &[],
            &[100, 200, 300, 400],
            11,
        )
        .unwrap();
        assert_eq!(BackpackAccount::total_mass_grams(&data), Ok(600));

        let mut replacement = blueprint_slot(900);
        replacement.set_mass_grams(50);
        BackpackAccount::replace_slot_at(&mut data, &owner, 1, &replacement, 12).unwrap();
        assert_eq!(BackpackAccount::total_mass_grams(&data), Ok(450));

        BackpackAccount::remove_resources_at(&mut data, &owner, &[0, 2], 13).unwrap();
        assert_eq!(BackpackAccount::total_mass_grams(&data), Ok(50));
        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 1);
        assert_eq!(BackpackAccount::slot_at(&data, 0).unwrap().item_id, 900);
    }

    #[test]
    fn failed_append_and_replace_do_not_partially_mutate_backpack() {
        let owner = Pubkey::new_unique();
        let mut append_data = empty_backpack(&owner, 2);
        append_data[BackpackAccount::TOTAL_MASS_GRAMS_OFFSET
            ..BackpackAccount::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&u64::MAX.to_le_bytes());
        let before_append = append_data.clone();
        assert!(BackpackAccount::append_item(
            &mut append_data,
            &owner,
            &material_slot(1_200, 1_200),
            11,
        )
        .is_err());
        assert_eq!(append_data, before_append);

        let mut replace_data = empty_backpack(&owner, 2);
        BackpackAccount::append_item(&mut replace_data, &owner, &material_slot(1_200, 1_200), 11)
            .unwrap();
        replace_data[BackpackAccount::TOTAL_MASS_GRAMS_OFFSET
            ..BackpackAccount::TOTAL_MASS_GRAMS_OFFSET + 8]
            .fill(0);
        let before_replace = replace_data.clone();
        assert!(BackpackAccount::replace_slot_at(
            &mut replace_data,
            &owner,
            0,
            &blueprint_slot(901),
            12,
        )
        .is_err());
        assert_eq!(replace_data, before_replace);
    }

    #[test]
    fn mining_snapshot_uses_pre_reward_mass_once_per_action() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        let mut carried = material_slot(1_200, 1_200);
        carried.set_mass_grams(25_000);
        BackpackAccount::append_item(&mut data, &owner, &carried, 11).unwrap();

        BackpackAccount::record_mining_action(&mut data, &owner, 7, 12).unwrap();
        assert_eq!(BackpackAccount::last_mine_pre_mass_grams(&data), Ok(25_000));
        assert_eq!(BackpackAccount::mine_sequence(&data), Ok(1));
        BackpackAccount::append_resource_with_volume_metadata_and_mass(
            &mut data,
            &owner,
            &BackpackResourceRecord::default(),
            1_000_000,
            0,
            1_000,
            13,
        )
        .unwrap();

        BackpackAccount::record_mining_action(&mut data, &owner, 7, 14).unwrap();
        assert_eq!(BackpackAccount::last_mine_pre_mass_grams(&data), Ok(25_000));
        assert_eq!(BackpackAccount::mine_sequence(&data), Ok(1));

        BackpackAccount::record_mining_action(&mut data, &owner, 8, 15).unwrap();
        assert_eq!(BackpackAccount::last_mine_pre_mass_grams(&data), Ok(26_000));
        assert_eq!(BackpackAccount::mine_sequence(&data), Ok(2));
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = core::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }
}
