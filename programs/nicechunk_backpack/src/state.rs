use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkBackpackError;

pub const BACKPACK_MAGIC: [u8; 8] = *b"NCKBPK01";
pub const BACKPACK_VERSION: u16 = 4;
pub const BACKPACK_SEED: &[u8] = b"backpack";
pub const BACKPACK_DEFAULT_CAPACITY: u8 = 50;
pub const BACKPACK_MAX_CAPACITY: u8 = 99;
pub const BACKPACK_STACK_LIMIT: u32 = 99;
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
pub const BACKPACK_FLAG_MASS_STATE_VALID: u8 = 1;
pub const MATERIAL_PHYSICS_MAGIC: [u8; 8] = *b"NCKPHY02";
pub const MATERIAL_PHYSICS_VERSION: u8 = 2;
pub const MATERIAL_PHYSICS_SEED: &[u8] = b"material-physics-v2";
pub const MATERIAL_PHYSICS_HEADER_LEN: usize = 16;
pub const MATERIAL_PHYSICS_RULE_LEN: usize = 8;
pub const MATERIAL_PHYSICS_MAX_RULES: usize = 128;
pub const MATERIAL_PHYSICS_LEN: usize =
    MATERIAL_PHYSICS_HEADER_LEN + MATERIAL_PHYSICS_MAX_RULES * MATERIAL_PHYSICS_RULE_LEN;
pub const MATERIAL_PHYSICS_ITEM_KEY_MASK: u16 = 1 << 15;
pub const FORGED_ITEM_MAGIC: [u8; 8] = *b"NCKFGI01";
pub const FORGED_ITEM_VERSION: u16 = 1;
pub const FORGED_ITEM_SEED: &[u8] = b"forged-item-v1";
pub const FORGED_ITEM_CODE_MAX_BYTES: usize = 640;
pub const FORGED_ITEM_HEADER_LEN: usize = 96;
pub const FORGED_ITEM_LEN: usize = 752;
pub const SESSION_ACTION_BREAK_BLOCK: u8 = 1;
pub const DURABILITY_BPS_DENOMINATOR: u64 = 10_000;
pub const MAX_FORGING_INPUTS: usize = 24;
pub const MAX_VERIFIED_FORGE_CODE_BYTES: usize = 640;
const NCF1_VERSION: u32 = 15;
const NCF1_ATTRIBUTE_COUNT: usize = 12;
const NCF1_V15_VOLUME_MANTISSA_BITS: u32 = 13;
const NCF1_V15_VOLUME_MANTISSA_MASK: u32 = (1 << NCF1_V15_VOLUME_MANTISSA_BITS) - 1;
const NCF1_V15_ATTRIBUTE_REFERENCE_VOLUME_MM3: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaterialPhysicsRule {
    pub key: u16,
    pub density_kg_m3: u16,
    pub standard_volume_mm3: u32,
}

impl MaterialPhysicsRule {
    pub fn block_key(block_id: u16) -> Result<u16, NicechunkBackpackError> {
        if block_id == 0 || block_id & MATERIAL_PHYSICS_ITEM_KEY_MASK != 0 {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRule);
        }
        Ok(block_id)
    }

    pub fn item_key(item_code: u16) -> Result<u16, NicechunkBackpackError> {
        if item_code == 0 || item_code & MATERIAL_PHYSICS_ITEM_KEY_MASK != 0 {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRule);
        }
        Ok(item_code | MATERIAL_PHYSICS_ITEM_KEY_MASK)
    }

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkBackpackError> {
        if data.len() != MATERIAL_PHYSICS_RULE_LEN {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRule);
        }
        let rule = Self {
            key: read_u16(data, 0),
            density_kg_m3: read_u16(data, 2),
            standard_volume_mm3: read_u32(data, 4),
        };
        rule.validate()?;
        Ok(rule)
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        if dst.len() != MATERIAL_PHYSICS_RULE_LEN {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRule.into());
        }
        self.validate()?;
        dst[0..2].copy_from_slice(&self.key.to_le_bytes());
        dst[2..4].copy_from_slice(&self.density_kg_m3.to_le_bytes());
        dst[4..8].copy_from_slice(&self.standard_volume_mm3.to_le_bytes());
        Ok(())
    }

    pub fn mass_grams(&self, volume_mm3: u32) -> Result<u32, NicechunkBackpackError> {
        if volume_mm3 == 0 {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRule);
        }
        let numerator = (volume_mm3 as u64)
            .checked_mul(self.density_kg_m3 as u64)
            .ok_or(NicechunkBackpackError::BackpackMassOverflow)?;
        let rounded = numerator
            .checked_add(500_000)
            .ok_or(NicechunkBackpackError::BackpackMassOverflow)?
            / 1_000_000;
        u32::try_from(rounded).map_err(|_| NicechunkBackpackError::BackpackMassOverflow)
    }

    fn validate(&self) -> Result<(), NicechunkBackpackError> {
        let id = self.key & !MATERIAL_PHYSICS_ITEM_KEY_MASK;
        if id == 0 || self.density_kg_m3 == 0 || self.standard_volume_mm3 == 0 {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsRule);
        }
        Ok(())
    }
}

pub struct MaterialPhysicsTableState;

impl MaterialPhysicsTableState {
    pub const LEN: usize = MATERIAL_PHYSICS_LEN;
    pub const REVISION_OFFSET: usize = 12;

    pub fn validate_payload(payload: &[u8]) -> Result<usize, NicechunkBackpackError> {
        if payload.len() < 5 {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
        }
        if read_u32(payload, 0) == 0 {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
        }
        let count = payload[4] as usize;
        if count == 0
            || count > MATERIAL_PHYSICS_MAX_RULES
            || payload.len() != 5 + count * MATERIAL_PHYSICS_RULE_LEN
        {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
        }
        Self::validate_sorted_rules(&payload[5..], count)?;
        Ok(count)
    }

    pub fn pack_payload(dst: &mut [u8], bump: u8, payload: &[u8]) -> ProgramResult {
        let count = Self::validate_payload(payload)?;
        if dst.len() != Self::LEN {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&MATERIAL_PHYSICS_MAGIC);
        dst[8] = MATERIAL_PHYSICS_VERSION;
        dst[9] = bump;
        dst[10] = count as u8;
        dst[Self::REVISION_OFFSET..Self::REVISION_OFFSET + 4].copy_from_slice(&payload[0..4]);
        let byte_len = count * MATERIAL_PHYSICS_RULE_LEN;
        dst[MATERIAL_PHYSICS_HEADER_LEN..MATERIAL_PHYSICS_HEADER_LEN + byte_len]
            .copy_from_slice(&payload[5..]);
        Ok(())
    }

    pub fn validate_header(data: &[u8]) -> Result<usize, NicechunkBackpackError> {
        if data.len() != Self::LEN
            || data[0..8] != MATERIAL_PHYSICS_MAGIC
            || data[8] != MATERIAL_PHYSICS_VERSION
        {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
        }
        let count = data[10] as usize;
        if count == 0 || count > MATERIAL_PHYSICS_MAX_RULES {
            return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
        }
        Ok(count)
    }

    pub fn block_rule(
        data: &[u8],
        block_id: u16,
    ) -> Result<MaterialPhysicsRule, NicechunkBackpackError> {
        MaterialPhysicsTableView::new(data)?.block_rule(block_id)
    }

    pub fn item_rule(
        data: &[u8],
        item_code: u16,
    ) -> Result<MaterialPhysicsRule, NicechunkBackpackError> {
        MaterialPhysicsTableView::new(data)?.item_rule(item_code)
    }

    pub fn apply_mass(
        data: &[u8],
        slot: &mut BackpackSlotRecord,
    ) -> Result<u32, NicechunkBackpackError> {
        MaterialPhysicsTableView::new(data)?.apply_mass(slot)
    }

    pub fn validate_mass(
        data: &[u8],
        slot: &BackpackSlotRecord,
    ) -> Result<u32, NicechunkBackpackError> {
        MaterialPhysicsTableView::new(data)?.validate_mass(slot)
    }

    fn validate_sorted_rules(data: &[u8], count: usize) -> Result<(), NicechunkBackpackError> {
        let mut previous = 0_u16;
        for index in 0..count {
            let offset = index * MATERIAL_PHYSICS_RULE_LEN;
            let rule =
                MaterialPhysicsRule::unpack(&data[offset..offset + MATERIAL_PHYSICS_RULE_LEN])?;
            if index > 0 && rule.key <= previous {
                return Err(NicechunkBackpackError::InvalidMaterialPhysicsData);
            }
            previous = rule.key;
        }
        Ok(())
    }

    pub fn revision(data: &[u8]) -> Result<u32, NicechunkBackpackError> {
        Self::validate_header(data)?;
        Ok(read_u32(data, Self::REVISION_OFFSET))
    }
}

pub struct MaterialPhysicsTableView<'a> {
    data: &'a [u8],
    count: usize,
}

impl<'a> MaterialPhysicsTableView<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, NicechunkBackpackError> {
        Ok(Self {
            data,
            count: MaterialPhysicsTableState::validate_header(data)?,
        })
    }

    pub fn block_rule(&self, block_id: u16) -> Result<MaterialPhysicsRule, NicechunkBackpackError> {
        self.rule(MaterialPhysicsRule::block_key(block_id)?)
    }

    pub fn item_rule(&self, item_code: u16) -> Result<MaterialPhysicsRule, NicechunkBackpackError> {
        self.rule(MaterialPhysicsRule::item_key(item_code)?)
    }

    pub fn block_mass_grams(
        &self,
        block_id: u16,
        volume_mm3: u32,
    ) -> Result<u32, NicechunkBackpackError> {
        self.block_rule(block_id)?.mass_grams(volume_mm3)
    }

    pub fn apply_mass(&self, slot: &mut BackpackSlotRecord) -> Result<u32, NicechunkBackpackError> {
        let mass_grams = match slot.kind {
            BACKPACK_SLOT_KIND_BLOCK => self
                .block_rule(slot.block_id()?)?
                .mass_grams(slot.volume_mm3)?,
            BACKPACK_SLOT_KIND_ITEM if slot.category == BACKPACK_ITEM_CATEGORY_MATERIAL => self
                .item_rule(slot.item_code)?
                .mass_grams(slot.volume_mm3)?,
            BACKPACK_SLOT_KIND_ITEM
                if matches!(
                    slot.category,
                    BACKPACK_ITEM_CATEGORY_FORGED | BACKPACK_ITEM_CATEGORY_BLUEPRINT
                ) =>
            {
                slot.mass_grams()?
            }
            _ => return Err(NicechunkBackpackError::InvalidInventoryItem),
        };
        slot.set_mass_grams(mass_grams)
            .map_err(|_| NicechunkBackpackError::InvalidInventoryItem)?;
        Ok(mass_grams)
    }

    pub fn validate_mass(&self, slot: &BackpackSlotRecord) -> Result<u32, NicechunkBackpackError> {
        let stored = slot.mass_grams()?;
        let mut verified = *slot;
        let expected = self.apply_mass(&mut verified)?;
        if stored != expected {
            return Err(NicechunkBackpackError::InvalidBackpackMassState);
        }
        Ok(stored)
    }

    fn rule(&self, key: u16) -> Result<MaterialPhysicsRule, NicechunkBackpackError> {
        let mut low = 0_usize;
        let mut high = self.count;
        while low < high {
            let middle = low + (high - low) / 2;
            let offset = MATERIAL_PHYSICS_HEADER_LEN + middle * MATERIAL_PHYSICS_RULE_LEN;
            match read_u16(self.data, offset).cmp(&key) {
                core::cmp::Ordering::Less => low = middle + 1,
                core::cmp::Ordering::Greater => high = middle,
                core::cmp::Ordering::Equal => {
                    return MaterialPhysicsRule::unpack(
                        &self.data[offset..offset + MATERIAL_PHYSICS_RULE_LEN],
                    )
                }
            }
        }
        Err(NicechunkBackpackError::InvalidMaterialPhysicsRule)
    }
}

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
    let version = read_bits(code, &mut bit_offset, 4)?;
    if version != NCF1_VERSION {
        return Err(NicechunkBackpackError::InvalidForgeMaterialRequirements);
    }
    let mass_grams = (read_bits(code, &mut bit_offset, 16)? as u64).saturating_mul(5);
    let encoded_volume = read_bits(code, &mut bit_offset, 16)?;
    let volume_mm3 = decode_ncf1_v15_volume_mm3(encoded_volume);
    let mut attributes = [0_u64; NCF1_ATTRIBUTE_COUNT];
    for attribute in attributes.iter_mut() {
        let compact = read_bits(code, &mut bit_offset, 6)? as u64;
        *attribute = compact.saturating_mul(100).saturating_add(31) / 63;
    }
    if mass_grams == 0 || volume_mm3 == 0 {
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
    let volume_requirement = integer_sqrt(volume_mm3 / 1_000).saturating_mul(18);
    let base_attribute_requirement = material_score.saturating_mul(126).saturating_add(24) / 25;
    // Smelting metadata defines durability per 1,000,000 mm3. Scale attribute
    // demand by the same physical amount for sub-unit materials.
    let attribute_requirement = base_attribute_requirement
        .saturating_mul(volume_mm3.min(NCF1_V15_ATTRIBUTE_REFERENCE_VOLUME_MM3))
        / NCF1_V15_ATTRIBUTE_REFERENCE_VOLUME_MM3;
    let requirements = ForgeMaterialRequirements {
        required_volume_mm3: volume_mm3,
        required_effective_durability: mass_requirement
            .saturating_add(volume_requirement)
            .saturating_add(attribute_requirement)
            .max(1),
        output_mass_grams: mass_grams.min(u32::MAX as u64) as u32,
    };
    requirements.validate()?;
    Ok((fnv1a32(code), requirements))
}

fn decode_ncf1_v15_volume_mm3(encoded_volume: u32) -> u64 {
    let exponent = encoded_volume >> NCF1_V15_VOLUME_MANTISSA_BITS;
    let mantissa = encoded_volume & NCF1_V15_VOLUME_MANTISSA_MASK;
    (mantissa as u64) << exponent.saturating_mul(4)
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
    pub total_mass_grams: u64,
}

impl ForgeMaterialCapacity {
    pub fn satisfies(&self, requirements: &ForgeMaterialRequirements) -> bool {
        self.total_volume_mm3 >= requirements.required_volume_mm3
            && self.total_effective_durability >= requirements.required_effective_durability
            && self.total_mass_grams >= requirements.output_mass_grams as u64
    }
}

pub const PLAYER_PROFILE_LEN: usize = 773;
pub const PLAYER_PROFILE_MAGIC: [u8; 8] = *b"NCKPLY01";
pub const PLAYER_PROFILE_VERSION: u16 = 7;
pub const PLAYER_PROFILE_INITIALIZED_OFFSET: usize = 11;
pub const PLAYER_PROFILE_OWNER_OFFSET: usize = 12;
pub const PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET: usize = 44;
pub const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET: usize = 102;
pub const PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT: usize = 9;
pub const PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET: usize = 393;

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
pub const PLAYER_SESSION_VERSION: u16 = 1;
pub const PLAYER_SESSION_INITIALIZED_OFFSET: usize = 11;
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
        writer.u8(BACKPACK_FLAG_MASS_STATE_VALID)?;
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
        if version != BACKPACK_VERSION
            || data[11] != 1
            || data[Self::FLAGS_OFFSET] & BACKPACK_FLAG_MASS_STATE_VALID == 0
        {
            return Err(NicechunkBackpackError::InvalidBackpackData);
        }
        validate_capacity(data[Self::CAPACITY_OFFSET])?;
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        if item_count > data[Self::CAPACITY_OFFSET] {
            return Err(NicechunkBackpackError::InvalidBackpackData);
        }
        Ok(())
    }

    pub fn total_mass_grams(data: &[u8]) -> Result<u64, NicechunkBackpackError> {
        Self::validate(data)?;
        Ok(read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET))
    }

    pub fn last_mine_pre_mass_grams(data: &[u8]) -> Result<u64, NicechunkBackpackError> {
        Self::validate(data)?;
        Ok(read_u64(data, Self::LAST_MINE_PRE_MASS_GRAMS_OFFSET))
    }

    pub fn mine_sequence(data: &[u8]) -> Result<u64, NicechunkBackpackError> {
        Self::validate(data)?;
        Ok(read_u64(data, Self::MINE_SEQUENCE_OFFSET))
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
        if read_u64(data, Self::LAST_MINE_ACTION_ID_OFFSET) != action_id {
            let total_mass = read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET);
            let next_sequence = read_u64(data, Self::MINE_SEQUENCE_OFFSET)
                .checked_add(1)
                .ok_or(NicechunkBackpackError::BackpackMassOverflow)?;
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

    fn add_total_mass(data: &mut [u8], mass_grams: u32) -> ProgramResult {
        let next = read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET)
            .checked_add(mass_grams as u64)
            .ok_or(NicechunkBackpackError::BackpackMassOverflow)?;
        data[Self::TOTAL_MASS_GRAMS_OFFSET..Self::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&next.to_le_bytes());
        Ok(())
    }

    fn subtract_total_mass(data: &mut [u8], mass_grams: u32) -> ProgramResult {
        let next = read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET)
            .checked_sub(mass_grams as u64)
            .ok_or(NicechunkBackpackError::InvalidBackpackMassState)?;
        data[Self::TOTAL_MASS_GRAMS_OFFSET..Self::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&next.to_le_bytes());
        Ok(())
    }

    fn try_store_slot(
        data: &mut [u8],
        incoming: &BackpackSlotRecord,
    ) -> Result<bool, solana_program::program_error::ProgramError> {
        Self::retire_legacy_blueprints(data)?;
        let mut packed = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        incoming.pack(&mut packed)?;

        let item_count = data[Self::ITEM_COUNT_OFFSET];
        let capacity = data[Self::CAPACITY_OFFSET];
        if incoming.kind == BACKPACK_SLOT_KIND_BLOCK && incoming.quantity == 1 {
            for index in 0..item_count {
                let existing = Self::slot_at(data, index)?;
                if let Some((merged, None)) = existing.merged_resource_stack(incoming)? {
                    let offset = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
                    merged.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
                    return Ok(true);
                }
            }
        }
        if item_count < capacity
            && (incoming.kind != BACKPACK_SLOT_KIND_BLOCK || incoming.quantity == 1)
        {
            let offset = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
            data[offset..offset + BACKPACK_SLOT_RECORD_LEN].copy_from_slice(&packed);
            data[Self::ITEM_COUNT_OFFSET] = item_count.saturating_add(1);
            return Ok(true);
        }

        let mut candidate = Vec::with_capacity(item_count as usize + 1);
        for index in 0..item_count {
            candidate.push(Self::slot_at(data, index)?);
        }
        Self::push_compacted_slot(&mut candidate, *incoming)?;
        if candidate.len() <= capacity as usize {
            Self::write_slots(data, item_count, &candidate)?;
            return Ok(true);
        }

        // Compact fragmented resource stacks in a candidate buffer so a failed
        // insert never mutates the account.
        let mut compacted = Vec::with_capacity(item_count as usize + 1);
        for index in 0..item_count {
            Self::push_compacted_slot(&mut compacted, Self::slot_at(data, index)?)?;
        }
        Self::push_compacted_slot(&mut compacted, *incoming)?;
        if compacted.len() > capacity as usize {
            return Ok(false);
        }

        Self::write_slots(data, item_count, &compacted)?;
        Ok(true)
    }

    fn retire_legacy_blueprints(data: &mut [u8]) -> ProgramResult {
        let item_count = data[Self::ITEM_COUNT_OFFSET];
        let mut retained = Vec::with_capacity(item_count as usize);
        let mut retired_mass = 0_u64;
        for index in 0..item_count {
            let slot = Self::slot_at(data, index)?;
            if slot.is_retired_blueprint() {
                retired_mass = retired_mass
                    .checked_add(u64::from(slot.mass_grams()?))
                    .ok_or(NicechunkBackpackError::BackpackMassOverflow)?;
            } else {
                retained.push(slot);
            }
        }
        if retained.len() == item_count as usize {
            return Ok(());
        }
        let next_mass = read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET)
            .checked_sub(retired_mass)
            .ok_or(NicechunkBackpackError::InvalidBackpackMassState)?;
        Self::write_slots(data, item_count, &retained)?;
        data[Self::TOTAL_MASS_GRAMS_OFFSET..Self::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&next_mass.to_le_bytes());
        Ok(())
    }

    fn write_slots(
        data: &mut [u8],
        previous_count: u8,
        slots: &[BackpackSlotRecord],
    ) -> ProgramResult {
        for (index, slot) in slots.iter().enumerate() {
            let offset = Self::RECORDS_OFFSET + index * BACKPACK_SLOT_RECORD_LEN;
            slot.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
        }
        for index in slots.len()..previous_count as usize {
            let offset = Self::RECORDS_OFFSET + index * BACKPACK_SLOT_RECORD_LEN;
            data[offset..offset + BACKPACK_SLOT_RECORD_LEN].fill(0);
        }
        data[Self::ITEM_COUNT_OFFSET] = slots.len() as u8;
        Ok(())
    }

    fn push_compacted_slot(
        slots: &mut Vec<BackpackSlotRecord>,
        incoming: BackpackSlotRecord,
    ) -> ProgramResult {
        let mut remaining = Some(incoming);
        for existing in slots.iter_mut() {
            let Some(candidate) = remaining else {
                break;
            };
            if let Some((merged, remainder)) = existing.merged_resource_stack(&candidate)? {
                *existing = merged;
                remaining = remainder;
            }
        }
        if let Some(remainder) = remaining {
            slots.push(remainder);
        }
        Ok(())
    }

    fn set_updated_slot(data: &mut [u8], updated_slot: u64) {
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
    }

    pub fn validate_owner(data: &[u8], owner: &Pubkey) -> ProgramResult {
        Self::validate(data)?;
        if &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkBackpackError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn append_resource_with_volume(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackResourceRecord,
        volume_mm3: u32,
        mass_grams: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::append_resource_with_volume_and_metadata(
            data,
            owner,
            record,
            volume_mm3,
            0,
            mass_grams,
            updated_slot,
        )
    }

    pub fn append_resource_with_volume_and_metadata(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackResourceRecord,
        volume_mm3: u32,
        metadata: u32,
        mass_grams: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let mut slot = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
            *record, volume_mm3, metadata,
        );
        slot.set_mass_grams(mass_grams)?;
        if !Self::try_store_slot(data, &slot)? {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        Self::add_total_mass(data, mass_grams)?;
        Self::set_updated_slot(data, updated_slot);
        Ok(())
    }

    pub fn append_resources_lossy_with_volumes_and_metadata(
        data: &mut [u8],
        owner: &Pubkey,
        records: &[BackpackResourceRecord],
        volumes_mm3: &[u32],
        metadata: &[u32],
        masses_grams: &[u32],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if records.len() != volumes_mm3.len()
            || records.len() != metadata.len()
            || records.len() != masses_grams.len()
        {
            return Err(NicechunkBackpackError::InvalidInstruction.into());
        }
        if records.is_empty() {
            return Ok(());
        }

        let mut stored_any = false;
        for (index, record) in records.iter().enumerate() {
            let mut slot = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
                *record,
                volumes_mm3[index],
                metadata[index],
            );
            slot.set_mass_grams(masses_grams[index])?;
            if Self::try_store_slot(data, &slot)? {
                Self::add_total_mass(data, masses_grams[index])?;
                stored_any = true;
            }
        }
        if stored_any {
            Self::set_updated_slot(data, updated_slot);
        }
        Ok(())
    }

    pub fn append_item(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackSlotRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        if record.is_retired_blueprint() {
            Self::retire_legacy_blueprints(data)?;
            Self::set_updated_slot(data, updated_slot);
            return Ok(());
        }
        let mass_grams = record.mass_grams()?;
        if record.kind == BACKPACK_SLOT_KIND_BLOCK && record.quantity == 0 {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }
        if record.kind == BACKPACK_SLOT_KIND_ITEM
            && (record.quantity == 0 || record.item_id == 0 || record.item_pda == Pubkey::default())
        {
            return Err(NicechunkBackpackError::InvalidInventoryItem.into());
        }
        if !Self::try_store_slot(data, record)? {
            return Err(NicechunkBackpackError::BackpackFull.into());
        }
        Self::add_total_mass(data, mass_grams)?;
        Self::set_updated_slot(data, updated_slot);
        Ok(())
    }

    pub fn append_issued_item(
        data: &mut [u8],
        owner: &Pubkey,
        record: &BackpackSlotRecord,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::append_item(data, owner, record, updated_slot)
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
        let removed = BackpackSlotRecord::unpack(&data[start..start + BACKPACK_SLOT_RECORD_LEN])?;
        Self::subtract_total_mass(data, removed.mass_grams()?)?;
        let end = Self::RECORDS_OFFSET + item_count as usize * BACKPACK_SLOT_RECORD_LEN;
        if start + BACKPACK_SLOT_RECORD_LEN < end {
            data.copy_within(start + BACKPACK_SLOT_RECORD_LEN..end, start);
        }
        let last_start =
            Self::RECORDS_OFFSET + (item_count - 1) as usize * BACKPACK_SLOT_RECORD_LEN;
        data[last_start..last_start + BACKPACK_SLOT_RECORD_LEN].fill(0);
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
        if record.is_retired_blueprint() {
            return Self::remove_resource_at(data, owner, index, updated_slot);
        }
        let offset = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        let previous =
            BackpackSlotRecord::unpack(&data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
        let previous_mass = previous.mass_grams()?;
        let replacement_mass = record.mass_grams()?;
        let next_mass = read_u64(data, Self::TOTAL_MASS_GRAMS_OFFSET)
            .checked_sub(previous_mass as u64)
            .and_then(|mass| mass.checked_add(replacement_mass as u64))
            .ok_or(NicechunkBackpackError::InvalidBackpackMassState)?;
        record.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])?;
        data[Self::TOTAL_MASS_GRAMS_OFFSET..Self::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&next_mass.to_le_bytes());
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

    pub fn consume_smelting_resources(
        data: &mut [u8],
        owner: &Pubkey,
        input_quantities: &[u32; BACKPACK_MAX_CAPACITY as usize],
        fuel_indexes: &[bool; BACKPACK_MAX_CAPACITY as usize],
        material_physics: &MaterialPhysicsTableView<'_>,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_owner(data, owner)?;
        let item_count = data[Self::ITEM_COUNT_OFFSET] as usize;
        let mut selected_count = 0_usize;
        for index in 0..BACKPACK_MAX_CAPACITY as usize {
            let input_quantity = input_quantities[index];
            if input_quantity > 0 && fuel_indexes[index] {
                return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
            }
            if input_quantity > 0 || fuel_indexes[index] {
                if index >= item_count {
                    return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
                }
                selected_count = selected_count.saturating_add(1);
            }
        }
        if selected_count == 0 {
            return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
        }

        // Original indexes remain stable while records are processed from high to low.
        for index in (0..item_count).rev() {
            let input_quantity = input_quantities[index];
            if input_quantity == 0 && !fuel_indexes[index] {
                continue;
            }
            let record = Self::slot_at(data, index as u8)?;
            let consumed_quantity = if fuel_indexes[index] {
                1
            } else {
                input_quantity
            };
            if consumed_quantity == 0 || consumed_quantity > record.quantity {
                return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
            }
            if consumed_quantity == record.quantity {
                Self::remove_resource_at(data, owner, index as u8, updated_slot)?;
                continue;
            }
            if record.volume_mm3 <= 1 {
                return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
            }

            let consumed_volume = proportional_consumed_volume_mm3(
                record.volume_mm3,
                record.quantity,
                consumed_quantity,
            )?;
            let remaining_volume = record.volume_mm3.saturating_sub(consumed_volume);
            if remaining_volume == 0 {
                return Err(NicechunkBackpackError::InvalidSmeltingConsumption.into());
            }
            let mut remaining = record;
            remaining.quantity = record.quantity.saturating_sub(consumed_quantity);
            remaining.volume_mm3 = remaining_volume;
            if record.kind == BACKPACK_SLOT_KIND_ITEM {
                remaining.durability_max = scale_nonzero_metadata(
                    record.durability_max,
                    remaining_volume,
                    record.volume_mm3,
                );
                remaining.durability_current = scale_nonzero_metadata(
                    record.durability_current,
                    remaining_volume,
                    record.volume_mm3,
                )
                .min(remaining.durability_max);
            }
            material_physics.apply_mass(&mut remaining)?;
            Self::replace_slot_at(data, owner, index as u8, &remaining, updated_slot)?;
        }
        Ok(())
    }

    pub fn consume_placement_resource(
        data: &mut [u8],
        owner: &Pubkey,
        index: u8,
        expected_slot: &[u8; BACKPACK_SLOT_RECORD_LEN],
        material_physics: &MaterialPhysicsTableView<'_>,
        updated_slot: u64,
    ) -> Result<(u16, u32), solana_program::program_error::ProgramError> {
        Self::validate_owner(data, owner)?;
        if index >= data[Self::ITEM_COUNT_OFFSET] {
            return Err(NicechunkBackpackError::InvalidResourceIndex.into());
        }
        let offset = Self::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
        if &data[offset..offset + BACKPACK_SLOT_RECORD_LEN] != expected_slot {
            return Err(NicechunkBackpackError::PlacementSlotMismatch.into());
        }

        let record = BackpackSlotRecord::unpack(expected_slot)?;
        if record.kind != BACKPACK_SLOT_KIND_BLOCK || record.volume_mm3 == 0 {
            return Err(NicechunkBackpackError::InvalidPlacementConsumption.into());
        }
        let block_id = record.block_id()?;
        material_physics.validate_mass(&record)?;
        let consumed_volume_mm3 =
            proportional_consumed_volume_mm3(record.volume_mm3, record.quantity, 1)
                .map_err(|_| NicechunkBackpackError::InvalidPlacementConsumption)?;

        if record.quantity == 1 {
            Self::remove_resource_at(data, owner, index, updated_slot)?;
        } else {
            let mut remaining = record;
            remaining.quantity = remaining.quantity.saturating_sub(1);
            remaining.volume_mm3 = remaining.volume_mm3.saturating_sub(consumed_volume_mm3);
            if remaining.volume_mm3 == 0 {
                return Err(NicechunkBackpackError::InvalidPlacementConsumption.into());
            }
            material_physics.apply_mass(&mut remaining)?;
            Self::replace_slot_at(data, owner, index, &remaining, updated_slot)?;
        }

        Ok((block_id, consumed_volume_mm3))
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
            let capacity = forge_material_capacity(&materials)?;
            if !capacity.satisfies(&required) {
                return Err(NicechunkBackpackError::InsufficientForgeMaterialParameters.into());
            }
        }

        let mut outcome = calculate_forge_outcome(&materials, forging_level);
        if let Some(required) = requirements {
            outcome.volume_mm3 = required.required_volume_mm3.min(u32::MAX as u64) as u32;
            outcome.mass_grams = required.output_mass_grams;
        } else {
            outcome.mass_grams = materials
                .iter()
                .filter_map(|material| material.mass_grams().ok())
                .fold(0_u32, u32::saturating_add);
        }
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
        output.set_mass_grams(outcome.mass_grams)?;
        Self::append_item(data, owner, &output, updated_slot)?;
        Ok(outcome)
    }
}

fn proportional_consumed_volume_mm3(
    total_volume_mm3: u32,
    total_quantity: u32,
    consumed_quantity: u32,
) -> Result<u32, NicechunkBackpackError> {
    if total_volume_mm3 == 0
        || total_quantity == 0
        || consumed_quantity == 0
        || consumed_quantity > total_quantity
    {
        return Err(NicechunkBackpackError::InvalidSmeltingConsumption);
    }
    if consumed_quantity == total_quantity {
        return Ok(total_volume_mm3);
    }
    let proportional = (total_volume_mm3 as u64)
        .saturating_mul(consumed_quantity as u64)
        .saturating_div(total_quantity as u64);
    Ok(proportional
        .max(1)
        .min(total_volume_mm3.saturating_sub(1) as u64) as u32)
}

fn proportional_stack_value(
    total: u32,
    total_quantity: u32,
    moved_quantity: u32,
) -> Result<u32, NicechunkBackpackError> {
    if total_quantity == 0 || moved_quantity == 0 || moved_quantity > total_quantity {
        return Err(NicechunkBackpackError::InvalidInventoryItem);
    }
    if moved_quantity == total_quantity {
        return Ok(total);
    }
    Ok(((total as u64)
        .saturating_mul(moved_quantity as u64)
        .saturating_div(total_quantity as u64)) as u32)
}

fn scale_nonzero_metadata(value: u32, numerator: u32, denominator: u32) -> u32 {
    (value as u64)
        .saturating_mul(numerator as u64)
        .saturating_div(denominator.max(1) as u64)
        .max(1)
        .min(u32::MAX as u64) as u32
}

pub struct ForgedItemInitArgs<'a> {
    pub bump: u8,
    pub item_id: u64,
    pub creator: &'a Pubkey,
    pub origin_backpack: &'a Pubkey,
    pub design_hash: u32,
    pub code: &'a [u8],
    pub created_slot: u64,
    pub created_at: i64,
}

/// Immutable canonical model data for one forged item.
pub struct ForgedItemAccount;

impl ForgedItemAccount {
    pub const LEN: usize = FORGED_ITEM_LEN;
    pub const ITEM_ID_OFFSET: usize = 12;
    pub const CREATOR_OFFSET: usize = 20;
    pub const ORIGIN_BACKPACK_OFFSET: usize = 52;
    pub const DESIGN_HASH_OFFSET: usize = 84;
    pub const CODE_LENGTH_OFFSET: usize = 88;
    pub const CODE_OFFSET: usize = FORGED_ITEM_HEADER_LEN;
    pub const CREATED_SLOT_OFFSET: usize = 736;
    pub const CREATED_AT_OFFSET: usize = 744;

    pub fn pack(dst: &mut [u8], args: &ForgedItemInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN
            || args.item_id == 0
            || *args.creator == Pubkey::default()
            || *args.origin_backpack == Pubkey::default()
            || args.code.len() < 14
            || args.code.len() > FORGED_ITEM_CODE_MAX_BYTES
        {
            return Err(NicechunkBackpackError::InvalidForgedItemData.into());
        }
        let (design_hash, _) = verified_forge_design(args.code)?;
        if design_hash != args.design_hash {
            return Err(NicechunkBackpackError::InvalidForgedItemData.into());
        }

        dst.fill(0);
        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&FORGED_ITEM_MAGIC)?;
        writer.u16(FORGED_ITEM_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(1)?;
        writer.u64(args.item_id)?;
        writer.pubkey(args.creator)?;
        writer.pubkey(args.origin_backpack)?;
        writer.bytes(&args.design_hash.to_le_bytes())?;
        writer.u16(args.code.len() as u16)?;
        writer.bytes(&[0_u8; 6])?;
        if writer.offset != FORGED_ITEM_HEADER_LEN {
            return Err(NicechunkBackpackError::PackSizeMismatch.into());
        }
        writer.bytes(args.code)?;
        writer.offset = Self::CREATED_SLOT_OFFSET;
        writer.u64(args.created_slot)?;
        writer.i64(args.created_at)?;
        if writer.offset != Self::LEN {
            return Err(NicechunkBackpackError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate(data: &[u8]) -> ProgramResult {
        if data.len() != Self::LEN
            || data[0..8] != FORGED_ITEM_MAGIC
            || read_u16(data, 8) != FORGED_ITEM_VERSION
            || data[11] != 1
            || read_u64(data, Self::ITEM_ID_OFFSET) == 0
            || data[Self::CREATOR_OFFSET..Self::CREATOR_OFFSET + 32]
                .iter()
                .all(|byte| *byte == 0)
            || data[Self::ORIGIN_BACKPACK_OFFSET..Self::ORIGIN_BACKPACK_OFFSET + 32]
                .iter()
                .all(|byte| *byte == 0)
        {
            return Err(NicechunkBackpackError::InvalidForgedItemData.into());
        }
        let code_len = read_u16(data, Self::CODE_LENGTH_OFFSET) as usize;
        if code_len < 14 || code_len > FORGED_ITEM_CODE_MAX_BYTES {
            return Err(NicechunkBackpackError::InvalidForgedItemData.into());
        }
        let code = &data[Self::CODE_OFFSET..Self::CODE_OFFSET + code_len];
        let (design_hash, _) = verified_forge_design(code)?;
        if design_hash != read_u32(data, Self::DESIGN_HASH_OFFSET) {
            return Err(NicechunkBackpackError::InvalidForgedItemData.into());
        }
        Ok(())
    }

    pub fn code(data: &[u8]) -> Result<&[u8], NicechunkBackpackError> {
        Self::validate(data).map_err(|_| NicechunkBackpackError::InvalidForgedItemData)?;
        let code_len = read_u16(data, Self::CODE_LENGTH_OFFSET) as usize;
        Ok(&data[Self::CODE_OFFSET..Self::CODE_OFFSET + code_len])
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
            metadata,
        }
    }

    pub fn block_id(&self) -> Result<u16, NicechunkBackpackError> {
        if self.kind != BACKPACK_SLOT_KIND_BLOCK {
            return Err(NicechunkBackpackError::InvalidInventoryItem);
        }
        let block_id = (self.resource.world_y as u16) >> 9;
        if block_id == 0 {
            return Err(NicechunkBackpackError::InvalidInventoryItem);
        }
        Ok(block_id)
    }

    pub fn is_retired_blueprint(&self) -> bool {
        self.kind == BACKPACK_SLOT_KIND_ITEM
            && self.category == BACKPACK_ITEM_CATEGORY_BLUEPRINT
            && self.item_code == BACKPACK_BLUEPRINT_ITEM_CODE
    }

    pub fn set_mass_grams(&mut self, mass_grams: u32) -> ProgramResult {
        match self.kind {
            BACKPACK_SLOT_KIND_BLOCK => self.durability_current = mass_grams,
            BACKPACK_SLOT_KIND_ITEM => {
                self.resource.world_x = i32::from_le_bytes(mass_grams.to_le_bytes())
            }
            _ => return Err(NicechunkBackpackError::InvalidInventoryItem.into()),
        }
        self.flags |= BACKPACK_ITEM_FLAG_MASS_VALID;
        Ok(())
    }

    pub fn mass_grams(&self) -> Result<u32, NicechunkBackpackError> {
        if self.flags & BACKPACK_ITEM_FLAG_MASS_VALID == 0 {
            return Err(NicechunkBackpackError::InvalidBackpackMassState);
        }
        match self.kind {
            BACKPACK_SLOT_KIND_BLOCK => Ok(self.durability_current),
            BACKPACK_SLOT_KIND_ITEM => Ok(u32::from_le_bytes(self.resource.world_x.to_le_bytes())),
            _ => Err(NicechunkBackpackError::InvalidInventoryItem),
        }
    }

    fn merged_resource_stack(
        &self,
        incoming: &Self,
    ) -> Result<Option<(Self, Option<Self>)>, NicechunkBackpackError> {
        if self.kind != BACKPACK_SLOT_KIND_BLOCK
            || incoming.kind != BACKPACK_SLOT_KIND_BLOCK
            || self.category != incoming.category
            || self.flags != incoming.flags
            || self.metadata != incoming.metadata
            || self.block_id()? != incoming.block_id()?
            || self.quantity >= BACKPACK_STACK_LIMIT
        {
            return Ok(None);
        }
        let moved_quantity = incoming
            .quantity
            .min(BACKPACK_STACK_LIMIT.saturating_sub(self.quantity));
        let moved_volume_mm3 =
            proportional_stack_value(incoming.volume_mm3, incoming.quantity, moved_quantity)?;
        let incoming_mass_grams = incoming.mass_grams()?;
        let moved_mass_grams =
            proportional_stack_value(incoming_mass_grams, incoming.quantity, moved_quantity)?;
        let mut merged = *self;
        merged.quantity = self
            .quantity
            .checked_add(moved_quantity)
            .ok_or(NicechunkBackpackError::InvalidInventoryItem)?;
        merged.volume_mm3 = self
            .volume_mm3
            .checked_add(moved_volume_mm3)
            .ok_or(NicechunkBackpackError::InvalidInventoryItem)?;
        let merged_mass_grams = self
            .mass_grams()?
            .checked_add(moved_mass_grams)
            .ok_or(NicechunkBackpackError::BackpackMassOverflow)?;
        merged
            .set_mass_grams(merged_mass_grams)
            .map_err(|_| NicechunkBackpackError::InvalidInventoryItem)?;

        let remaining_quantity = incoming.quantity.saturating_sub(moved_quantity);
        if remaining_quantity == 0 {
            return Ok(Some((merged, None)));
        }
        let mut remainder = *incoming;
        remainder.quantity = remaining_quantity;
        remainder.volume_mm3 = incoming.volume_mm3.saturating_sub(moved_volume_mm3);
        remainder
            .set_mass_grams(incoming_mass_grams.saturating_sub(moved_mass_grams))
            .map_err(|_| NicechunkBackpackError::InvalidInventoryItem)?;
        Ok(Some((merged, Some(remainder))))
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
        if record.quantity == 0
            || (record.kind == BACKPACK_SLOT_KIND_BLOCK && record.quantity > BACKPACK_STACK_LIMIT)
        {
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
        if self.quantity == 0
            || (self.kind == BACKPACK_SLOT_KIND_BLOCK && self.quantity > BACKPACK_STACK_LIMIT)
        {
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

pub fn forge_material_capacity(
    materials: &[BackpackSlotRecord],
) -> Result<ForgeMaterialCapacity, NicechunkBackpackError> {
    let mut capacity = ForgeMaterialCapacity::default();
    let mut effective_durability_numerator = 0_u64;
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
        capacity.total_mass_grams = capacity
            .total_mass_grams
            .checked_add(material.mass_grams()? as u64)
            .ok_or(NicechunkBackpackError::BackpackMassOverflow)?;
        effective_durability_numerator = effective_durability_numerator
            .saturating_add(durability_current.saturating_mul(quality_bps));
    }
    capacity.total_effective_durability =
        effective_durability_numerator / DURABILITY_BPS_DENOMINATOR;
    Ok(capacity)
}

fn calculate_forge_outcome(materials: &[BackpackSlotRecord], forging_level: u8) -> ForgeOutcome {
    let mut total_volume = 0_u64;
    let mut total_raw_durability = 0_u64;
    let mut effective_durability_numerator = 0_u64;
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
        effective_durability_numerator = effective_durability_numerator
            .saturating_add(current_durability.saturating_mul(quality));
        weighted_grade = weighted_grade.saturating_add(grade as u64 * volume);
        weighted_quality = weighted_quality.saturating_add(quality * volume);
    }

    let total_effective_durability = effective_durability_numerator / DURABILITY_BPS_DENOMINATOR;

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
    let blended_grade = ((material_grade as u16 * 3 + item_level_grade as u16 + 2) / 4) as u8;
    let grade = blended_grade
        .max(1)
        .min(10)
        .min(material_grade.saturating_add(1).min(10))
        .min(weak_grade_cap);

    let material_factor = 90_u64.saturating_add(grade as u64 * 5);
    let level_factor = 100_u64.saturating_add(item_level as u64 / 2);
    let base_candidate = total_effective_durability
        .saturating_mul(material_factor)
        .saturating_mul(level_factor)
        / 10_000;
    let base_material_cap = total_raw_durability.saturating_mul(105) / 100;
    let base_durability = base_candidate.max(1).min(base_material_cap.max(1));
    let durability_bonus_percent = 100_u64.saturating_add(forging_level.min(10) as u64 * 5);
    let durability_max = base_durability
        .saturating_mul(durability_bonus_percent)
        .saturating_div(100)
        .max(1)
        .min(u32::MAX as u64) as u32;

    ForgeOutcome {
        grade,
        item_level,
        durability_max,
        quality_bps,
        volume_mm3: total_volume.max(1).min(u32::MAX as u64) as u32,
        mass_grams: 0,
        gained_xp: 1,
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
            || data[11] != 1
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
        Self::validate_layout(data)?;
        if &data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32] != owner.as_ref() {
            return Err(NicechunkBackpackError::InvalidBackpackOwner.into());
        }
        Ok(())
    }

    pub fn has_equipped_backpack(data: &[u8]) -> Result<bool, NicechunkBackpackError> {
        Self::validate_layout(data)?;
        Ok(data
            [PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET..PLAYER_PROFILE_EQUIPPED_BACKPACK_OFFSET + 32]
            .iter()
            .any(|byte| *byte != 0))
    }

    pub fn owner_and_global_config(
        data: &[u8],
    ) -> Result<(Pubkey, Pubkey), NicechunkBackpackError> {
        Self::validate_layout(data)?;
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

    fn validate_layout(data: &[u8]) -> Result<(), NicechunkBackpackError> {
        if data.len() != PLAYER_PROFILE_LEN
            || data[0..8] != PLAYER_PROFILE_MAGIC
            || read_u16(data, 8) != PLAYER_PROFILE_VERSION
            || data[PLAYER_PROFILE_INITIALIZED_OFFSET] != 1
            || data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] as usize
                != PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT
        {
            return Err(NicechunkBackpackError::InvalidPlayerProfile);
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
        action: u8,
        now: i64,
    ) -> Result<Self, NicechunkBackpackError> {
        if data.len() != PLAYER_SESSION_LEN
            || data[0..8] != PLAYER_SESSION_MAGIC
            || read_u16(data, 8) != PLAYER_SESSION_VERSION
            || data[PLAYER_SESSION_INITIALIZED_OFFSET] != 1
        {
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

    fn player_profile(owner: &Pubkey, global_config: &Pubkey) -> Vec<u8> {
        let mut data = vec![0_u8; PLAYER_PROFILE_LEN];
        data[0..8].copy_from_slice(&PLAYER_PROFILE_MAGIC);
        data[8..10].copy_from_slice(&PLAYER_PROFILE_VERSION.to_le_bytes());
        data[PLAYER_PROFILE_INITIALIZED_OFFSET] = 1;
        data[PLAYER_PROFILE_OWNER_OFFSET..PLAYER_PROFILE_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        data[PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET..PLAYER_PROFILE_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        data[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] =
            PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT as u8;
        data
    }

    fn player_session(
        owner: &Pubkey,
        authority: &Pubkey,
        profile: &Pubkey,
        expires_at: i64,
    ) -> Vec<u8> {
        let mut data = vec![0_u8; PLAYER_SESSION_LEN];
        data[0..8].copy_from_slice(&PLAYER_SESSION_MAGIC);
        data[8..10].copy_from_slice(&PLAYER_SESSION_VERSION.to_le_bytes());
        data[PLAYER_SESSION_INITIALIZED_OFFSET] = 1;
        data[PLAYER_SESSION_OWNER_OFFSET..PLAYER_SESSION_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        data[PLAYER_SESSION_AUTHORITY_OFFSET..PLAYER_SESSION_AUTHORITY_OFFSET + 32]
            .copy_from_slice(authority.as_ref());
        data[PLAYER_SESSION_PROFILE_OFFSET..PLAYER_SESSION_PROFILE_OFFSET + 32]
            .copy_from_slice(profile.as_ref());
        data[PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET..PLAYER_SESSION_ALLOWED_ACTIONS_OFFSET + 2]
            .copy_from_slice(&(1_u16 << SESSION_ACTION_BREAK_BLOCK).to_le_bytes());
        data[PLAYER_SESSION_EXPIRES_AT_OFFSET..PLAYER_SESSION_EXPIRES_AT_OFFSET + 8]
            .copy_from_slice(&expires_at.to_le_bytes());
        data
    }

    #[test]
    fn player_profile_view_requires_the_final_initialized_layout() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let data = player_profile(&owner, &global_config);
        PlayerProfileView::validate_owner(&data, &owner).unwrap();
        PlayerProfileView::owner_and_global_config(&data).unwrap();

        let mut retired = data.clone();
        retired[8..10].copy_from_slice(&(PLAYER_PROFILE_VERSION - 1).to_le_bytes());
        assert!(PlayerProfileView::validate_owner(&retired, &owner).is_err());

        let mut uninitialized = data.clone();
        uninitialized[PLAYER_PROFILE_INITIALIZED_OFFSET] = 0;
        assert!(PlayerProfileView::has_equipped_backpack(&uninitialized).is_err());

        let mut wrong_slot_count = data;
        wrong_slot_count[PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT_OFFSET] =
            (PLAYER_PROFILE_EQUIPMENT_SLOT_COUNT - 1) as u8;
        assert!(PlayerProfileView::owner_and_global_config(&wrong_slot_count).is_err());
    }

    #[test]
    fn player_session_view_requires_the_final_initialized_layout() {
        let owner = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let profile = Pubkey::new_unique();
        let data = player_session(&owner, &authority, &profile, 200);
        PlayerSessionView::validate(&data, &authority, &profile, SESSION_ACTION_BREAK_BLOCK, 100)
            .unwrap();

        let mut retired = data.clone();
        retired[8..10].copy_from_slice(&(PLAYER_SESSION_VERSION + 1).to_le_bytes());
        assert!(PlayerSessionView::validate(
            &retired,
            &authority,
            &profile,
            SESSION_ACTION_BREAK_BLOCK,
            100,
        )
        .is_err());

        let mut uninitialized = data;
        uninitialized[PLAYER_SESSION_INITIALIZED_OFFSET] = 0;
        assert!(PlayerSessionView::validate(
            &uninitialized,
            &authority,
            &profile,
            SESSION_ACTION_BREAK_BLOCK,
            100,
        )
        .is_err());
    }

    fn material_slot(durability_current: u32, durability_max: u32) -> BackpackSlotRecord {
        let mut slot = BackpackSlotRecord {
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
        slot.set_mass_grams(600).unwrap();
        slot
    }

    fn blueprint_slot(item_id: u64) -> BackpackSlotRecord {
        let mut slot = BackpackSlotRecord {
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
        slot.set_mass_grams(0).unwrap();
        slot
    }

    fn block_resource(block_id: u16, coordinate: i32) -> BackpackResourceRecord {
        BackpackResourceRecord {
            world_x: coordinate,
            world_y: ((block_id << 9) | (coordinate as u16 & 0x01ff)) as i16,
            world_z: coordinate.saturating_neg(),
        }
    }

    fn packed_slot(record: &BackpackSlotRecord) -> [u8; BACKPACK_SLOT_RECORD_LEN] {
        let mut data = [0_u8; BACKPACK_SLOT_RECORD_LEN];
        record.pack(&mut data).unwrap();
        data
    }

    fn forge_single_material_at_level(
        durability_current: u32,
        durability_max: u32,
        forging_level: u8,
    ) -> ForgeOutcome {
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
            forging_level,
            12,
        )
        .unwrap()
    }

    fn forge_single_material(durability_current: u32, durability_max: u32) -> ForgeOutcome {
        forge_single_material_at_level(durability_current, durability_max, 3)
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
    fn retired_blueprint_append_is_discarded_without_using_capacity() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);

        BackpackAccount::append_issued_item(&mut data, &owner, &blueprint_slot(901), 12).unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 0);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 0);
        assert_eq!(read_u64(&data, BackpackAccount::UPDATED_SLOT_OFFSET), 12);
    }

    #[test]
    fn next_item_write_compacts_historical_blueprint_slots() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);
        let legacy = blueprint_slot(901);
        data[BackpackAccount::RECORDS_OFFSET
            ..BackpackAccount::RECORDS_OFFSET + BACKPACK_SLOT_RECORD_LEN]
            .copy_from_slice(&packed_slot(&legacy));
        data[BackpackAccount::ITEM_COUNT_OFFSET] = 1;

        let material = material_slot(1_200, 1_200);
        BackpackAccount::append_item(&mut data, &owner, &material, 13).unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 1);
        assert_eq!(
            packed_slot(&BackpackAccount::slot_at(&data, 0).unwrap()),
            packed_slot(&material)
        );
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 600);
    }

    #[test]
    fn retired_equipment_replacement_is_destroyed_instead_of_returned() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);
        BackpackAccount::append_item(&mut data, &owner, &material_slot(1_200, 1_200), 11).unwrap();

        BackpackAccount::replace_slot_at(&mut data, &owner, 0, &blueprint_slot(902), 12).unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 0);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 0);
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
            2_600,
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
    fn mined_resources_fill_a_stack_to_99_before_opening_another_slot() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, BACKPACK_DEFAULT_CAPACITY);

        for coordinate in 0..100 {
            BackpackAccount::append_resource_with_volume_and_metadata(
                &mut data,
                &owner,
                &block_resource(3, coordinate),
                1_000_000,
                7,
                2_600,
                11 + coordinate as u64,
            )
            .unwrap();
        }

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 2);
        let first = BackpackAccount::slot_at(&data, 0).unwrap();
        let second = BackpackAccount::slot_at(&data, 1).unwrap();
        assert_eq!(first.quantity, BACKPACK_STACK_LIMIT);
        assert_eq!(first.volume_mm3, 99_000_000);
        assert_eq!(first.mass_grams().unwrap(), 257_400);
        assert_eq!(second.quantity, 1);
        assert_eq!(second.volume_mm3, 1_000_000);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 260_000);
    }

    #[test]
    fn fifty_full_resource_stacks_hold_4950_items_and_reject_the_next() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, BACKPACK_DEFAULT_CAPACITY);

        for coordinate in 0..(BACKPACK_DEFAULT_CAPACITY as u32 * BACKPACK_STACK_LIMIT) {
            BackpackAccount::append_resource_with_volume_and_metadata(
                &mut data,
                &owner,
                &block_resource(3, coordinate as i32),
                1_000,
                0,
                3,
                11 + coordinate as u64,
            )
            .unwrap();
        }

        assert_eq!(
            data[BackpackAccount::ITEM_COUNT_OFFSET],
            BACKPACK_DEFAULT_CAPACITY
        );
        for index in 0..BACKPACK_DEFAULT_CAPACITY {
            assert_eq!(
                BackpackAccount::slot_at(&data, index).unwrap().quantity,
                BACKPACK_STACK_LIMIT
            );
        }
        let before = data.clone();
        let error = BackpackAccount::append_resource_with_volume_and_metadata(
            &mut data,
            &owner,
            &block_resource(3, 9_999),
            1_000,
            0,
            3,
            20_000,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ProgramError::Custom(code) if code == NicechunkBackpackError::BackpackFull as u32
        ));
        assert_eq!(data, before);
    }

    #[test]
    fn market_resource_stack_uses_headroom_across_full_backpack_slots() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, BACKPACK_DEFAULT_CAPACITY);
        let mass_per_stack = 294_u32;
        for index in 0..BACKPACK_DEFAULT_CAPACITY {
            let mut slot = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
                block_resource(3, index as i32),
                98_000,
                0,
            );
            slot.quantity = 98;
            slot.set_mass_grams(mass_per_stack).unwrap();
            let offset =
                BackpackAccount::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
            slot.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])
                .unwrap();
        }
        data[BackpackAccount::ITEM_COUNT_OFFSET] = BACKPACK_DEFAULT_CAPACITY;
        data[BackpackAccount::TOTAL_MASS_GRAMS_OFFSET
            ..BackpackAccount::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(
                &(mass_per_stack as u64 * BACKPACK_DEFAULT_CAPACITY as u64).to_le_bytes(),
            );

        let mut incoming = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
            block_resource(3, 500),
            2_000,
            0,
        );
        incoming.quantity = 2;
        incoming.set_mass_grams(6).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &incoming, 100).unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 50);
        assert_eq!(BackpackAccount::slot_at(&data, 0).unwrap().quantity, 99);
        assert_eq!(BackpackAccount::slot_at(&data, 1).unwrap().quantity, 99);
        assert_eq!(BackpackAccount::slot_at(&data, 2).unwrap().quantity, 98);
        assert_eq!(
            BackpackAccount::total_mass_grams(&data).unwrap(),
            mass_per_stack as u64 * BACKPACK_DEFAULT_CAPACITY as u64 + 6,
        );
    }

    #[test]
    fn material_quantity_is_not_limited_by_block_stack_capacity() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 2);
        let mut material = material_slot(1_200, 1_200);
        material.quantity = 125;

        BackpackAccount::append_item(&mut data, &owner, &material, 11).unwrap();

        assert_eq!(BackpackAccount::slot_at(&data, 0).unwrap().quantity, 125);
    }

    #[test]
    fn fragmented_resource_records_compact_before_a_new_type_is_added() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        for index in 0..4_u8 {
            let mut slot = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
                block_resource(3, index as i32),
                1_000,
                0,
            );
            slot.set_mass_grams(3).unwrap();
            let offset =
                BackpackAccount::RECORDS_OFFSET + index as usize * BACKPACK_SLOT_RECORD_LEN;
            slot.pack(&mut data[offset..offset + BACKPACK_SLOT_RECORD_LEN])
                .unwrap();
        }
        data[BackpackAccount::ITEM_COUNT_OFFSET] = 4;
        data[BackpackAccount::TOTAL_MASS_GRAMS_OFFSET
            ..BackpackAccount::TOTAL_MASS_GRAMS_OFFSET + 8]
            .copy_from_slice(&12_u64.to_le_bytes());

        BackpackAccount::append_resource_with_volume_and_metadata(
            &mut data,
            &owner,
            &block_resource(4, 100),
            2_000,
            0,
            5,
            20,
        )
        .unwrap();

        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 2);
        let compacted = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(compacted.block_id().unwrap(), 3);
        assert_eq!(compacted.quantity, 4);
        assert_eq!(compacted.volume_mm3, 4_000);
        assert_eq!(compacted.mass_grams().unwrap(), 12);
        assert_eq!(
            BackpackAccount::slot_at(&data, 1)
                .unwrap()
                .block_id()
                .unwrap(),
            4
        );
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 17);
    }

    #[test]
    fn lossy_batch_fills_existing_stack_without_exceeding_99() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 1);
        for coordinate in 0..98 {
            BackpackAccount::append_resource_with_volume_and_metadata(
                &mut data,
                &owner,
                &block_resource(3, coordinate),
                1_000,
                0,
                3,
                11 + coordinate as u64,
            )
            .unwrap();
        }
        let records = [block_resource(3, 100), block_resource(3, 101)];
        BackpackAccount::append_resources_lossy_with_volumes_and_metadata(
            &mut data,
            &owner,
            &records,
            &[1_000, 1_000],
            &[0, 0],
            &[3, 3],
            200,
        )
        .unwrap();

        let slot = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(slot.quantity, BACKPACK_STACK_LIMIT);
        assert_eq!(slot.volume_mm3, 99_000);
        assert_eq!(slot.mass_grams().unwrap(), 297);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 297);
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
    fn every_completed_forging_action_grants_one_xp() {
        let outcome = forge_single_material(1_200, 1_200);

        assert_eq!(outcome.gained_xp, 1);
    }

    #[test]
    fn forging_skill_adds_five_percent_durability_per_level() {
        let base = forge_single_material_at_level(8_000, 8_000, 0);
        let level_one = forge_single_material_at_level(8_000, 8_000, 1);
        let maximum = forge_single_material_at_level(8_000, 8_000, 10);

        assert_eq!(level_one.durability_max, base.durability_max * 105 / 100);
        assert_eq!(maximum.durability_max, base.durability_max * 150 / 100);
        assert_eq!(maximum.grade, base.grade);
    }

    #[test]
    fn forge_capacity_aggregates_fractional_quality_before_rounding() {
        let mut material = material_slot(1, 1);
        material.volume_mm3 = 155;
        material.quality_bps = 8_790;

        let capacity = forge_material_capacity(&[material, material]).unwrap();

        assert_eq!(capacity.total_volume_mm3, 310);
        assert_eq!(capacity.total_effective_durability, 1);
        assert_eq!(capacity.total_mass_grams, 1_200);
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
                output_mass_grams: 600,
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

        let outcome = BackpackAccount::forge_equipment_from_verified_materials(
            &mut data,
            &owner,
            &[0],
            901,
            0x1234_abcd,
            &Pubkey::new_unique(),
            3,
            12,
            ForgeMaterialRequirements {
                required_volume_mm3: 300_000,
                required_effective_durability: 840,
                output_mass_grams: 300,
            },
        )
        .unwrap();

        let forged = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(forged.category, BACKPACK_ITEM_CATEGORY_FORGED);
        assert_eq!(forged.volume_mm3, 300_000);
        assert_eq!(forged.mass_grams().unwrap(), 300);
        assert_eq!(outcome.volume_mm3, 300_000);
        assert_eq!(outcome.mass_grams, 300);
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
                output_mass_grams: 600,
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
    fn verified_forge_rejects_mass_creation_without_consuming_slots() {
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
                required_effective_durability: 840,
                output_mass_grams: 601,
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ProgramError::Custom(code)
                if code == NicechunkBackpackError::InsufficientForgeMaterialParameters as u32
        ));
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 600);
        assert_eq!(
            BackpackAccount::slot_at(&data, 0).unwrap().category,
            BACKPACK_ITEM_CATEGORY_MATERIAL
        );
    }

    #[test]
    fn verified_ncf1_rejects_retired_v14_header() {
        let code = hex_bytes("e09600bb8b2cb2cb2cb2cb2cb2c000");
        assert!(verified_forge_design(&code).is_err());
    }

    #[test]
    fn material_physics_uses_canonical_density_and_cubic_volume() {
        let data = material_physics_fixture();
        let physics = MaterialPhysicsTableView::new(&data).unwrap();

        assert_eq!(
            physics.block_rule(3).unwrap().standard_volume_mm3,
            1_000_000
        );
        assert_eq!(physics.block_mass_grams(3, 1_000_000).unwrap(), 2_600);
        assert_eq!(physics.block_mass_grams(23, 1_000_000).unwrap(), 250);
        assert_eq!(physics.block_mass_grams(49, 100_000).unwrap(), 14);
        assert_eq!(
            physics.item_rule(1010).unwrap().standard_volume_mm3,
            250_000
        );
        assert_eq!(
            physics
                .item_rule(1010)
                .unwrap()
                .mass_grams(250_000)
                .unwrap(),
            625
        );
        assert_eq!(
            physics.item_rule(1015).unwrap().mass_grams(18_600).unwrap(),
            153
        );
    }

    #[test]
    fn backpack_total_mass_tracks_append_remove_replace_and_forge() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        let material = material_slot(1_200, 1_200);
        BackpackAccount::append_item(&mut data, &owner, &material, 11).unwrap();
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 600);

        let replacement = blueprint_slot(901);
        BackpackAccount::replace_slot_at(&mut data, &owner, 0, &replacement, 12).unwrap();
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 0);
        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 0);

        BackpackAccount::append_item(&mut data, &owner, &material, 14).unwrap();
        BackpackAccount::forge_equipment_from_verified_materials(
            &mut data,
            &owner,
            &[0],
            902,
            0x1234_abcd,
            &Pubkey::new_unique(),
            3,
            15,
            ForgeMaterialRequirements {
                required_volume_mm3: 600_000,
                required_effective_durability: 840,
                output_mass_grams: 600,
            },
        )
        .unwrap();
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 600);
        assert_eq!(
            BackpackAccount::slot_at(&data, 0)
                .unwrap()
                .mass_grams()
                .unwrap(),
            600
        );
    }

    #[test]
    fn smelting_partial_stack_consumption_preserves_physical_state() {
        let owner = Pubkey::new_unique();
        let physics_data = material_physics_fixture();
        let physics = MaterialPhysicsTableView::new(&physics_data).unwrap();
        let mut data = empty_backpack(&owner, 2);
        let mut material = material_slot(2_000, 2_400);
        material.item_code = 1010;
        material.quantity = 4;
        material.volume_mm3 = 1_000_000;
        physics.apply_mass(&mut material).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &material, 11).unwrap();

        let mut input_quantities = [0_u32; BACKPACK_MAX_CAPACITY as usize];
        input_quantities[0] = 2;
        BackpackAccount::consume_smelting_resources(
            &mut data,
            &owner,
            &input_quantities,
            &[false; BACKPACK_MAX_CAPACITY as usize],
            &physics,
            12,
        )
        .unwrap();

        let remaining = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(remaining.quantity, 2);
        assert_eq!(remaining.volume_mm3, 500_000);
        assert_eq!(remaining.durability_current, 1_000);
        assert_eq!(remaining.durability_max, 1_200);
        assert_eq!(remaining.mass_grams().unwrap(), 1_250);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 1_250);
        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 1);
    }

    #[test]
    fn placement_consumes_one_exact_block_and_preserves_physical_state() {
        let owner = Pubkey::new_unique();
        let physics_data = material_physics_fixture();
        let physics = MaterialPhysicsTableView::new(&physics_data).unwrap();
        let mut data = empty_backpack(&owner, 2);
        let mut stack = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
            block_resource(3, 0),
            2_500_001,
            0,
        );
        stack.quantity = 4;
        physics.apply_mass(&mut stack).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &stack, 11).unwrap();
        let expected = packed_slot(&BackpackAccount::slot_at(&data, 0).unwrap());

        let consumed = BackpackAccount::consume_placement_resource(
            &mut data, &owner, 0, &expected, &physics, 12,
        )
        .unwrap();

        assert_eq!(consumed, (3, 625_000));
        let remaining = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(remaining.quantity, 3);
        assert_eq!(remaining.volume_mm3, 1_875_001);
        assert_eq!(remaining.mass_grams().unwrap(), 4_875);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 4_875);
    }

    #[test]
    fn placement_rejects_a_stale_slot_without_mutating_the_backpack() {
        let owner = Pubkey::new_unique();
        let physics_data = material_physics_fixture();
        let physics = MaterialPhysicsTableView::new(&physics_data).unwrap();
        let mut data = empty_backpack(&owner, 2);
        let mut resource =
            BackpackSlotRecord::from_block_resource_with_volume(block_resource(14, 0), 1_000_000);
        physics.apply_mass(&mut resource).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &resource, 11).unwrap();
        let before = data.clone();
        let mut stale = packed_slot(&BackpackAccount::slot_at(&data, 0).unwrap());
        stale[60] ^= 1;

        assert!(BackpackAccount::consume_placement_resource(
            &mut data, &owner, 0, &stale, &physics, 12,
        )
        .is_err());
        assert_eq!(data, before);
    }

    #[test]
    fn smelting_consumes_twelve_basalt_units_across_stacked_block_slots() {
        let owner = Pubkey::new_unique();
        let physics_data = material_physics_fixture();
        let physics = MaterialPhysicsTableView::new(&physics_data).unwrap();
        let mut data = empty_backpack(&owner, 2);

        let mut large_stack = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
            block_resource(14, 0),
            30_000_000,
            0,
        );
        large_stack.quantity = 30;
        physics.apply_mass(&mut large_stack).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &large_stack, 11).unwrap();

        let mut small_stack = BackpackSlotRecord::from_block_resource_with_volume_and_metadata(
            block_resource(14, 1),
            1_000_000,
            1,
        );
        physics.apply_mass(&mut small_stack).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &small_stack, 12).unwrap();

        let mut input_quantities = [0_u32; BACKPACK_MAX_CAPACITY as usize];
        input_quantities[0] = 11;
        input_quantities[1] = 1;
        BackpackAccount::consume_smelting_resources(
            &mut data,
            &owner,
            &input_quantities,
            &[false; BACKPACK_MAX_CAPACITY as usize],
            &physics,
            13,
        )
        .unwrap();

        let remaining = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(remaining.kind, BACKPACK_SLOT_KIND_BLOCK);
        assert_eq!(remaining.block_id().unwrap(), 14);
        assert_eq!(remaining.quantity, 19);
        assert_eq!(remaining.volume_mm3, 19_000_000);
        assert_eq!(remaining.mass_grams().unwrap(), 55_100);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 55_100);
        assert_eq!(data[BackpackAccount::ITEM_COUNT_OFFSET], 1);
    }

    #[test]
    fn smelting_consumes_one_unit_from_a_stacked_material_fuel() {
        let owner = Pubkey::new_unique();
        let physics_data = material_physics_fixture();
        let physics = MaterialPhysicsTableView::new(&physics_data).unwrap();
        let mut data = empty_backpack(&owner, 2);
        let mut fuel = material_slot(2_000, 2_400);
        fuel.item_code = 1010;
        fuel.quantity = 4;
        fuel.volume_mm3 = 1_000_000;
        physics.apply_mass(&mut fuel).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &fuel, 11).unwrap();

        let mut fuel_indexes = [false; BACKPACK_MAX_CAPACITY as usize];
        fuel_indexes[0] = true;
        BackpackAccount::consume_smelting_resources(
            &mut data,
            &owner,
            &[0_u32; BACKPACK_MAX_CAPACITY as usize],
            &fuel_indexes,
            &physics,
            12,
        )
        .unwrap();

        let remaining = BackpackAccount::slot_at(&data, 0).unwrap();
        assert_eq!(remaining.quantity, 3);
        assert_eq!(remaining.volume_mm3, 750_000);
        assert_eq!(remaining.durability_current, 1_500);
        assert_eq!(remaining.durability_max, 1_800);
        assert_eq!(remaining.mass_grams().unwrap(), 1_875);
        assert_eq!(BackpackAccount::total_mass_grams(&data).unwrap(), 1_875);
    }

    #[test]
    fn mining_snapshot_uses_pre_reward_mass_once_per_action() {
        let owner = Pubkey::new_unique();
        let mut data = empty_backpack(&owner, 4);
        let mut carried = material_slot(1_200, 1_200);
        carried.set_mass_grams(25_000).unwrap();
        BackpackAccount::append_item(&mut data, &owner, &carried, 11).unwrap();

        BackpackAccount::record_mining_action(&mut data, &owner, 7, 12).unwrap();
        assert_eq!(BackpackAccount::last_mine_pre_mass_grams(&data), Ok(25_000));
        assert_eq!(BackpackAccount::mine_sequence(&data), Ok(1));
        BackpackAccount::append_resource_with_volume_and_metadata(
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

    fn material_physics_fixture() -> Vec<u8> {
        let rules = [
            MaterialPhysicsRule {
                key: 3,
                density_kg_m3: 2_600,
                standard_volume_mm3: 1_000_000,
            },
            MaterialPhysicsRule {
                key: 14,
                density_kg_m3: 2_900,
                standard_volume_mm3: 1_000_000,
            },
            MaterialPhysicsRule {
                key: 23,
                density_kg_m3: 250,
                standard_volume_mm3: 1_000_000,
            },
            MaterialPhysicsRule {
                key: 49,
                density_kg_m3: 140,
                standard_volume_mm3: 1_000_000,
            },
            MaterialPhysicsRule {
                key: MATERIAL_PHYSICS_ITEM_KEY_MASK | 1010,
                density_kg_m3: 2_500,
                standard_volume_mm3: 250_000,
            },
            MaterialPhysicsRule {
                key: MATERIAL_PHYSICS_ITEM_KEY_MASK | 1015,
                density_kg_m3: 8_200,
                standard_volume_mm3: 250_000,
            },
        ];
        let mut payload = Vec::with_capacity(5 + rules.len() * MATERIAL_PHYSICS_RULE_LEN);
        payload.extend_from_slice(&1_u32.to_le_bytes());
        payload.push(rules.len() as u8);
        for rule in rules {
            let mut packed = [0_u8; MATERIAL_PHYSICS_RULE_LEN];
            rule.pack(&mut packed).unwrap();
            payload.extend_from_slice(&packed);
        }
        let mut data = vec![0_u8; MaterialPhysicsTableState::LEN];
        MaterialPhysicsTableState::pack_payload(&mut data, 252, &payload).unwrap();
        data
    }

    #[test]
    fn verified_ncf1_v15_header_supports_two_sub_cm3_materials() {
        let copper_bloom_attributes = [26, 37, 30, 54, 9, 52, 30, 37, 59, 55, 1, 53];
        let code = ncf1_header_code(NCF1_VERSION, 1, 310, copper_bloom_attributes);
        let (_, requirements) = verified_forge_design(&code).unwrap();

        assert_eq!(requirements.required_volume_mm3, 310);
        assert_eq!(requirements.required_effective_durability, 1);
    }

    #[test]
    fn forged_item_persists_one_unique_verified_ncf1_model() {
        let creator = Pubkey::new_unique();
        let backpack = Pubkey::new_unique();
        let code = ncf1_header_code(NCF1_VERSION, 1, 310, [7; NCF1_ATTRIBUTE_COUNT]);
        let (design_hash, _) = verified_forge_design(&code).unwrap();
        let mut data = vec![0_u8; ForgedItemAccount::LEN];
        ForgedItemAccount::pack(
            &mut data,
            &ForgedItemInitArgs {
                bump: 250,
                item_id: 901,
                creator: &creator,
                origin_backpack: &backpack,
                design_hash,
                code: &code,
                created_slot: 123,
                created_at: 456,
            },
        )
        .unwrap();

        ForgedItemAccount::validate(&data).unwrap();
        assert_eq!(ForgedItemAccount::code(&data).unwrap(), code);
        assert_eq!(
            &data[ForgedItemAccount::CREATOR_OFFSET..52],
            creator.as_ref()
        );
        assert_eq!(read_u64(&data, ForgedItemAccount::ITEM_ID_OFFSET), 901);

        let mut retired = data.clone();
        retired[8..10].copy_from_slice(&(FORGED_ITEM_VERSION + 1).to_le_bytes());
        assert!(ForgedItemAccount::validate(&retired).is_err());

        let mut corrupted = data;
        corrupted[ForgedItemAccount::CODE_OFFSET + 2] ^= 1;
        assert!(ForgedItemAccount::validate(&corrupted).is_err());
    }

    #[test]
    fn verified_ncf1_v15_volume_exponent_boundaries_are_exact() {
        let cases = [
            (1, 1_u64),
            (8_191, 8_191),
            ((1 << 13) | 512, 8_192),
            ((1 << 13) | 513, 8_208),
            ((7 << 13) | 8_191, 8_191_u64 << 28),
        ];

        for (encoded_volume, expected_volume_mm3) in cases {
            let code = ncf1_header_code(NCF1_VERSION, 1, encoded_volume, [0; NCF1_ATTRIBUTE_COUNT]);
            let (_, requirements) = verified_forge_design(&code).unwrap();
            assert_eq!(requirements.required_volume_mm3, expected_volume_mm3);
        }
    }

    #[test]
    fn verified_ncf1_v15_rejects_a_zero_volume_mantissa() {
        let code = ncf1_header_code(
            NCF1_VERSION,
            1,
            3 << NCF1_V15_VOLUME_MANTISSA_BITS,
            [0; NCF1_ATTRIBUTE_COUNT],
        );
        assert!(verified_forge_design(&code).is_err());
    }

    fn ncf1_header_code(
        version: u32,
        mass_5g: u32,
        encoded_volume: u32,
        attributes: [u8; NCF1_ATTRIBUTE_COUNT],
    ) -> Vec<u8> {
        let mut code = vec![0_u8; 14];
        let mut bit_offset = 0_usize;
        write_test_bits(&mut code, &mut bit_offset, version, 4);
        write_test_bits(&mut code, &mut bit_offset, mass_5g, 16);
        write_test_bits(&mut code, &mut bit_offset, encoded_volume, 16);
        for attribute in attributes {
            write_test_bits(&mut code, &mut bit_offset, attribute as u32, 6);
        }
        code
    }

    fn write_test_bits(bytes: &mut [u8], bit_offset: &mut usize, value: u32, bit_count: usize) {
        for shift in (0..bit_count).rev() {
            let bit = ((value >> shift) & 1) as u8;
            bytes[*bit_offset / 8] |= bit << (7 - (*bit_offset % 8));
            *bit_offset += 1;
        }
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
