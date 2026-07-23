use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkSkillsError;

pub const SKILL_COUNT: usize = 10;
pub const LEVEL_COUNT: usize = 10;
pub const MAX_SOURCE_RULES: usize = 32;
pub const MAX_GENERIC_SOURCE_RULES: usize = 30;
pub const BURDEN_WORK_CURSOR_INDEX: usize = 30;
pub const BURDEN_SEQUENCE_CURSOR_INDEX: usize = 31;
pub const BURDEN_RULE_RECORD_INDEX: usize = 31;
pub const BURDEN_RULE_MAGIC: [u8; 8] = *b"NCKBRD01";
pub const BURDEN_RULE_VERSION: u16 = 1;

pub const RULE_TABLE_MAGIC: [u8; 8] = *b"NCKXPR01";
pub const RULE_TABLE_VERSION: u16 = 1;
pub const RULE_TABLE_SEED: &[u8] = b"skill-rules-v1";
pub const RULE_TABLE_HEADER_LEN: usize = 912;
pub const RULE_TABLE_RULES_OFFSET: usize = RULE_TABLE_HEADER_LEN;
pub const RULE_RECORD_LEN: usize = 136;
pub const RULE_TABLE_LEN: usize = RULE_TABLE_HEADER_LEN + MAX_SOURCE_RULES * RULE_RECORD_LEN;
pub const RULE_TABLE_AUTHORITY_OFFSET: usize = 12;
pub const RULE_TABLE_GLOBAL_CONFIG_OFFSET: usize = 44;
pub const RULE_TABLE_RULE_COUNT_OFFSET: usize = 76;
pub const RULE_TABLE_SKILL_COUNT_OFFSET: usize = 77;
pub const RULE_TABLE_MINING_DISTANCE_OFFSET: usize = 78;
pub const RULE_TABLE_REVISION_OFFSET: usize = 80;
pub const RULE_TABLE_CREATED_SLOT_OFFSET: usize = 84;
pub const RULE_TABLE_UPDATED_SLOT_OFFSET: usize = 92;
pub const RULE_TABLE_CREATED_AT_OFFSET: usize = 100;
pub const RULE_TABLE_THRESHOLDS_OFFSET: usize = 108;
pub const RULE_TABLE_MINING_XP_OFFSET: usize = 908;
pub const RULE_TABLE_MINING_SKILL_INDEX_OFFSET: usize = 910;
pub const RULE_TABLE_MINING_ENABLED_OFFSET: usize = 911;

pub const PLAYER_SKILLS_MAGIC: [u8; 8] = *b"NCKSKL01";
pub const PLAYER_SKILLS_VERSION: u16 = 1;
pub const PLAYER_SKILLS_SEED: &[u8] = b"player-skills-v1";
pub const PLAYER_SKILLS_LEN: usize = 480;
pub const PLAYER_SKILLS_OWNER_OFFSET: usize = 12;
pub const PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET: usize = 44;
pub const PLAYER_SKILLS_XP_OFFSET: usize = 76;
pub const PLAYER_SKILLS_LEVELS_OFFSET: usize = 156;
pub const PLAYER_SKILLS_CURSOR_MASK_OFFSET: usize = 166;
pub const PLAYER_SKILLS_RULE_REVISION_OFFSET: usize = 172;
pub const PLAYER_SKILLS_CURSORS_OFFSET: usize = 176;
pub const PLAYER_SKILLS_CREATED_SLOT_OFFSET: usize = 432;
pub const PLAYER_SKILLS_UPDATED_SLOT_OFFSET: usize = 440;
pub const PLAYER_SKILLS_CREATED_AT_OFFSET: usize = 448;
pub const PLAYER_SKILLS_LAST_MINE_X_OFFSET: usize = 456;
pub const PLAYER_SKILLS_LAST_MINE_Y_OFFSET: usize = 460;
pub const PLAYER_SKILLS_LAST_MINE_Z_OFFSET: usize = 464;
pub const PLAYER_SKILLS_MINING_FLAGS_OFFSET: usize = 468;
pub const PLAYER_SKILLS_MINING_TRAVEL_COUNT_OFFSET: usize = 472;
pub const PLAYER_SKILLS_FLAG_HAS_LAST_MINE: u8 = 1 << 0;

pub const RULE_FLAG_BACKFILL_ON_FIRST_SYNC: u8 = 1 << 0;
pub const RULE_KNOWN_FLAGS: u8 = RULE_FLAG_BACKFILL_ON_FIRST_SYNC;
pub const SOURCE_SEED_GLOBAL_OWNER: u8 = 0;
pub const SOURCE_SEED_OWNER: u8 = 1;
pub const SOURCE_SEED_MAX_LEN: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRule {
    pub enabled: bool,
    pub metric_width: u8,
    pub flags: u8,
    pub seed_layout: u8,
    pub rule_id: u32,
    pub source_program: Pubkey,
    pub source_magic: [u8; 8],
    pub seed: [u8; SOURCE_SEED_MAX_LEN],
    pub seed_len: u8,
    pub owner_offset: u16,
    pub global_config_offset: u16,
    pub metric_offset: u16,
    pub max_delta_per_sync: u64,
    pub unit_divisor: u32,
    pub xp_per_unit: [u32; SKILL_COUNT],
}

impl SourceRule {
    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkSkillsError> {
        if data.len() != RULE_RECORD_LEN {
            return Err(NicechunkSkillsError::InvalidRule);
        }
        let mut source_magic = [0_u8; 8];
        source_magic.copy_from_slice(&data[40..48]);
        let mut seed = [0_u8; SOURCE_SEED_MAX_LEN];
        seed.copy_from_slice(&data[68..92]);
        let mut xp_per_unit = [0_u32; SKILL_COUNT];
        for (index, value) in xp_per_unit.iter_mut().enumerate() {
            *value = read_u32(data, 92 + index * 4);
        }
        let rule = Self {
            enabled: data[0] != 0,
            metric_width: data[1],
            flags: data[2],
            seed_layout: data[3],
            rule_id: read_u32(data, 4),
            source_program: read_pubkey(data, 8)?,
            source_magic,
            seed,
            seed_len: data[48],
            owner_offset: read_u16(data, 50),
            global_config_offset: read_u16(data, 52),
            metric_offset: read_u16(data, 54),
            max_delta_per_sync: read_u64(data, 56),
            unit_divisor: read_u32(data, 64),
            xp_per_unit,
        };
        rule.validate()?;
        Ok(rule)
    }

    pub fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        self.validate()?;
        if dst.len() != RULE_RECORD_LEN {
            return Err(NicechunkSkillsError::InvalidRule.into());
        }
        dst.fill(0);
        dst[0] = u8::from(self.enabled);
        dst[1] = self.metric_width;
        dst[2] = self.flags;
        dst[3] = self.seed_layout;
        dst[4..8].copy_from_slice(&self.rule_id.to_le_bytes());
        dst[8..40].copy_from_slice(self.source_program.as_ref());
        dst[40..48].copy_from_slice(&self.source_magic);
        dst[48] = self.seed_len;
        dst[50..52].copy_from_slice(&self.owner_offset.to_le_bytes());
        dst[52..54].copy_from_slice(&self.global_config_offset.to_le_bytes());
        dst[54..56].copy_from_slice(&self.metric_offset.to_le_bytes());
        dst[56..64].copy_from_slice(&self.max_delta_per_sync.to_le_bytes());
        dst[64..68].copy_from_slice(&self.unit_divisor.to_le_bytes());
        dst[68..92].copy_from_slice(&self.seed);
        for (index, value) in self.xp_per_unit.iter().enumerate() {
            let offset = 92 + index * 4;
            dst[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), NicechunkSkillsError> {
        if self.rule_id == 0
            || self.source_program == Pubkey::default()
            || self.source_magic.iter().all(|value| *value == 0)
            || self.seed_len == 0
            || self.seed_len as usize > SOURCE_SEED_MAX_LEN
            || !matches!(self.metric_width, 4 | 8)
            || !matches!(
                self.seed_layout,
                SOURCE_SEED_GLOBAL_OWNER | SOURCE_SEED_OWNER
            )
            || self.flags & !RULE_KNOWN_FLAGS != 0
            || self.max_delta_per_sync == 0
            || self.unit_divisor == 0
            || self.xp_per_unit.iter().all(|value| *value == 0)
        {
            return Err(NicechunkSkillsError::InvalidRule);
        }
        Ok(())
    }

    pub fn identity_matches(&self, other: &Self) -> bool {
        self.rule_id == other.rule_id
            && self.metric_width == other.metric_width
            && self.seed_layout == other.seed_layout
            && self.source_program == other.source_program
            && self.source_magic == other.source_magic
            && self.seed_len == other.seed_len
            && self.seed == other.seed
            && self.owner_offset == other.owner_offset
            && self.global_config_offset == other.global_config_offset
            && self.metric_offset == other.metric_offset
    }

    pub fn seed(&self) -> &[u8] {
        &self.seed[..self.seed_len as usize]
    }

    pub fn counter_from_source(&self, data: &[u8]) -> Result<u64, NicechunkSkillsError> {
        let offset = self.metric_offset as usize;
        match self.metric_width {
            4 if offset.checked_add(4).is_some_and(|end| end <= data.len()) => {
                Ok(read_u32(data, offset) as u64)
            }
            8 if offset.checked_add(8).is_some_and(|end| end <= data.len()) => {
                Ok(read_u64(data, offset))
            }
            _ => Err(NicechunkSkillsError::InvalidSourceData),
        }
    }
}

pub struct RuleTableState {
    pub authority: Pubkey,
    pub rule_count: u8,
    pub revision: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiningTravelRule {
    pub enabled: bool,
    pub minimum_distance: u16,
    pub skill_index: u8,
    pub xp_award: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BurdenMiningRule {
    pub enabled: bool,
    pub skill_index: u8,
    pub max_effective_mass_grams: u64,
    pub work_per_xp: u64,
}

impl BurdenMiningRule {
    pub fn validate(&self) -> Result<(), NicechunkSkillsError> {
        if self.enabled
            && (self.skill_index as usize >= SKILL_COUNT
                || self.max_effective_mass_grams == 0
                || self.work_per_xp == 0)
        {
            return Err(NicechunkSkillsError::InvalidBurdenMiningRule);
        }
        Ok(())
    }

    fn pack(&self, dst: &mut [u8]) -> ProgramResult {
        self.validate()?;
        if dst.len() != RULE_RECORD_LEN {
            return Err(NicechunkSkillsError::InvalidBurdenMiningRule.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&BURDEN_RULE_MAGIC);
        dst[8..10].copy_from_slice(&BURDEN_RULE_VERSION.to_le_bytes());
        dst[10] = u8::from(self.enabled);
        dst[11] = self.skill_index;
        dst[12..20].copy_from_slice(&self.max_effective_mass_grams.to_le_bytes());
        dst[20..28].copy_from_slice(&self.work_per_xp.to_le_bytes());
        Ok(())
    }

    fn unpack(data: &[u8]) -> Result<Self, NicechunkSkillsError> {
        if data.len() != RULE_RECORD_LEN
            || data[0..8] != BURDEN_RULE_MAGIC
            || read_u16(data, 8) != BURDEN_RULE_VERSION
        {
            return Err(NicechunkSkillsError::InvalidBurdenMiningRule);
        }
        let rule = Self {
            enabled: data[10] != 0,
            skill_index: data[11],
            max_effective_mass_grams: read_u64(data, 12),
            work_per_xp: read_u64(data, 20),
        };
        rule.validate()?;
        Ok(rule)
    }
}

impl MiningTravelRule {
    pub fn validate(&self) -> Result<(), NicechunkSkillsError> {
        if self.enabled
            && (self.minimum_distance == 0
                || self.skill_index as usize >= SKILL_COUNT
                || self.xp_award == 0)
        {
            return Err(NicechunkSkillsError::InvalidMiningTravelRule);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiningCoordinate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterApplyResult {
    pub changed: bool,
    pub applied_delta: u64,
}

impl RuleTableState {
    pub fn pack_empty(
        dst: &mut [u8],
        bump: u8,
        authority: &Pubkey,
        global_config: &Pubkey,
        created_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != RULE_TABLE_LEN {
            return Err(NicechunkSkillsError::InvalidRuleTableData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&RULE_TABLE_MAGIC);
        dst[8..10].copy_from_slice(&RULE_TABLE_VERSION.to_le_bytes());
        dst[10] = bump;
        dst[11] = 1;
        dst[RULE_TABLE_AUTHORITY_OFFSET..RULE_TABLE_AUTHORITY_OFFSET + 32]
            .copy_from_slice(authority.as_ref());
        dst[RULE_TABLE_GLOBAL_CONFIG_OFFSET..RULE_TABLE_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        dst[RULE_TABLE_RULE_COUNT_OFFSET] = 0;
        dst[RULE_TABLE_SKILL_COUNT_OFFSET] = SKILL_COUNT as u8;
        dst[RULE_TABLE_REVISION_OFFSET..RULE_TABLE_REVISION_OFFSET + 4]
            .copy_from_slice(&0_u32.to_le_bytes());
        dst[RULE_TABLE_CREATED_SLOT_OFFSET..RULE_TABLE_CREATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        dst[RULE_TABLE_UPDATED_SLOT_OFFSET..RULE_TABLE_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        dst[RULE_TABLE_CREATED_AT_OFFSET..RULE_TABLE_CREATED_AT_OFFSET + 8]
            .copy_from_slice(&created_at.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8], global_config: &Pubkey) -> Result<Self, NicechunkSkillsError> {
        if data.len() != RULE_TABLE_LEN
            || data[0..8] != RULE_TABLE_MAGIC
            || read_u16(data, 8) != RULE_TABLE_VERSION
            || data[11] != 1
            || data[RULE_TABLE_SKILL_COUNT_OFFSET] as usize != SKILL_COUNT
            || data[RULE_TABLE_RULE_COUNT_OFFSET] as usize > MAX_GENERIC_SOURCE_RULES
        {
            return Err(NicechunkSkillsError::InvalidRuleTableData);
        }
        if &data[RULE_TABLE_GLOBAL_CONFIG_OFFSET..RULE_TABLE_GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkSkillsError::InvalidRuleTableData);
        }
        Ok(Self {
            authority: read_pubkey(data, RULE_TABLE_AUTHORITY_OFFSET)?,
            rule_count: data[RULE_TABLE_RULE_COUNT_OFFSET],
            revision: read_u32(data, RULE_TABLE_REVISION_OFFSET),
        })
    }

    pub fn validate_authority(
        data: &[u8],
        global_config: &Pubkey,
        authority: &Pubkey,
    ) -> ProgramResult {
        let state = Self::validate(data, global_config)?;
        if &state.authority != authority {
            return Err(NicechunkSkillsError::UnauthorizedAuthority.into());
        }
        Ok(())
    }

    pub fn set_authority(
        data: &mut [u8],
        global_config: &Pubkey,
        authority: &Pubkey,
        new_authority: &Pubkey,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_authority(data, global_config, authority)?;
        data[RULE_TABLE_AUTHORITY_OFFSET..RULE_TABLE_AUTHORITY_OFFSET + 32]
            .copy_from_slice(new_authority.as_ref());
        Self::increment_revision(data, updated_slot)
    }

    pub fn set_thresholds(
        data: &mut [u8],
        global_config: &Pubkey,
        authority: &Pubkey,
        skill_index: usize,
        thresholds: &[u64; LEVEL_COUNT],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_authority(data, global_config, authority)?;
        if skill_index >= SKILL_COUNT {
            return Err(NicechunkSkillsError::InvalidSkillIndex.into());
        }
        validate_thresholds(thresholds)?;
        for (level_index, threshold) in thresholds.iter().enumerate() {
            let offset = threshold_offset(skill_index, level_index);
            data[offset..offset + 8].copy_from_slice(&threshold.to_le_bytes());
        }
        Self::increment_revision(data, updated_slot)
    }

    pub fn set_mining_travel_rule(
        data: &mut [u8],
        global_config: &Pubkey,
        authority: &Pubkey,
        rule: &MiningTravelRule,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_authority(data, global_config, authority)?;
        rule.validate()?;
        data[RULE_TABLE_MINING_DISTANCE_OFFSET..RULE_TABLE_MINING_DISTANCE_OFFSET + 2]
            .copy_from_slice(&rule.minimum_distance.to_le_bytes());
        data[RULE_TABLE_MINING_XP_OFFSET..RULE_TABLE_MINING_XP_OFFSET + 2]
            .copy_from_slice(&rule.xp_award.to_le_bytes());
        data[RULE_TABLE_MINING_SKILL_INDEX_OFFSET] = rule.skill_index;
        data[RULE_TABLE_MINING_ENABLED_OFFSET] = u8::from(rule.enabled);
        Self::increment_revision(data, updated_slot)
    }

    pub fn mining_travel_rule(data: &[u8]) -> Result<MiningTravelRule, NicechunkSkillsError> {
        Self::validate_header(data)?;
        let rule = MiningTravelRule {
            enabled: data[RULE_TABLE_MINING_ENABLED_OFFSET] != 0,
            minimum_distance: read_u16(data, RULE_TABLE_MINING_DISTANCE_OFFSET),
            skill_index: data[RULE_TABLE_MINING_SKILL_INDEX_OFFSET],
            xp_award: read_u16(data, RULE_TABLE_MINING_XP_OFFSET),
        };
        rule.validate()?;
        Ok(rule)
    }

    pub fn set_burden_mining_rule(
        data: &mut [u8],
        global_config: &Pubkey,
        authority: &Pubkey,
        rule: &BurdenMiningRule,
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate_authority(data, global_config, authority)?;
        let offset = rule_offset(BURDEN_RULE_RECORD_INDEX);
        rule.pack(&mut data[offset..offset + RULE_RECORD_LEN])?;
        Self::increment_revision(data, updated_slot)
    }

    pub fn burden_mining_rule(
        data: &[u8],
    ) -> Result<Option<BurdenMiningRule>, NicechunkSkillsError> {
        Self::validate_header(data)?;
        let offset = rule_offset(BURDEN_RULE_RECORD_INDEX);
        let record = &data[offset..offset + RULE_RECORD_LEN];
        if record.iter().all(|value| *value == 0) {
            return Ok(None);
        }
        BurdenMiningRule::unpack(record).map(Some)
    }

    pub fn upsert_rule(
        data: &mut [u8],
        global_config: &Pubkey,
        authority: &Pubkey,
        rule_index: usize,
        rule: &SourceRule,
        updated_slot: u64,
    ) -> ProgramResult {
        let state = Self::validate(data, global_config)?;
        if &state.authority != authority {
            return Err(NicechunkSkillsError::UnauthorizedAuthority.into());
        }
        if rule_index >= MAX_GENERIC_SOURCE_RULES {
            return Err(NicechunkSkillsError::InvalidRuleIndex.into());
        }
        rule.validate()?;
        let offset = rule_offset(rule_index);
        let previous = &data[offset..offset + RULE_RECORD_LEN];
        if previous.iter().any(|value| *value != 0) {
            let previous_rule = SourceRule::unpack(previous)?;
            if !previous_rule.identity_matches(rule) {
                return Err(NicechunkSkillsError::RuleIdentityImmutable.into());
            }
        }
        rule.pack(&mut data[offset..offset + RULE_RECORD_LEN])?;
        data[RULE_TABLE_RULE_COUNT_OFFSET] = state.rule_count.max((rule_index + 1) as u8);
        Self::increment_revision(data, updated_slot)
    }

    pub fn rule(data: &[u8], rule_index: usize) -> Result<SourceRule, NicechunkSkillsError> {
        let state = Self::validate_header(data)?;
        if rule_index >= state.rule_count as usize {
            return Err(NicechunkSkillsError::InvalidRuleIndex);
        }
        let offset = rule_offset(rule_index);
        SourceRule::unpack(&data[offset..offset + RULE_RECORD_LEN])
    }

    pub fn threshold(
        data: &[u8],
        skill_index: usize,
        level_index: usize,
    ) -> Result<u64, NicechunkSkillsError> {
        Self::validate_header(data)?;
        if skill_index >= SKILL_COUNT || level_index >= LEVEL_COUNT {
            return Err(NicechunkSkillsError::InvalidThresholds);
        }
        Ok(read_u64(data, threshold_offset(skill_index, level_index)))
    }

    fn validate_header(data: &[u8]) -> Result<Self, NicechunkSkillsError> {
        if data.len() != RULE_TABLE_LEN
            || data[0..8] != RULE_TABLE_MAGIC
            || read_u16(data, 8) != RULE_TABLE_VERSION
            || data[11] != 1
            || data[RULE_TABLE_RULE_COUNT_OFFSET] as usize > MAX_GENERIC_SOURCE_RULES
        {
            return Err(NicechunkSkillsError::InvalidRuleTableData);
        }
        Ok(Self {
            authority: read_pubkey(data, RULE_TABLE_AUTHORITY_OFFSET)?,
            rule_count: data[RULE_TABLE_RULE_COUNT_OFFSET],
            revision: read_u32(data, RULE_TABLE_REVISION_OFFSET),
        })
    }

    fn increment_revision(data: &mut [u8], updated_slot: u64) -> ProgramResult {
        let revision = read_u32(data, RULE_TABLE_REVISION_OFFSET)
            .checked_add(1)
            .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
        data[RULE_TABLE_REVISION_OFFSET..RULE_TABLE_REVISION_OFFSET + 4]
            .copy_from_slice(&revision.to_le_bytes());
        data[RULE_TABLE_UPDATED_SLOT_OFFSET..RULE_TABLE_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

pub struct PlayerSkillsState {
    pub xp: [u64; SKILL_COUNT],
    pub levels: [u8; SKILL_COUNT],
    pub cursor_mask: u32,
    pub rule_revision: u32,
}

impl PlayerSkillsState {
    pub fn pack_empty(
        dst: &mut [u8],
        bump: u8,
        owner: &Pubkey,
        global_config: &Pubkey,
        created_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != PLAYER_SKILLS_LEN {
            return Err(NicechunkSkillsError::InvalidPlayerSkillsData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&PLAYER_SKILLS_MAGIC);
        dst[8..10].copy_from_slice(&PLAYER_SKILLS_VERSION.to_le_bytes());
        dst[10] = bump;
        dst[11] = 1;
        dst[PLAYER_SKILLS_OWNER_OFFSET..PLAYER_SKILLS_OWNER_OFFSET + 32]
            .copy_from_slice(owner.as_ref());
        dst[PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET..PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET + 32]
            .copy_from_slice(global_config.as_ref());
        dst[PLAYER_SKILLS_CREATED_SLOT_OFFSET..PLAYER_SKILLS_CREATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        dst[PLAYER_SKILLS_UPDATED_SLOT_OFFSET..PLAYER_SKILLS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&created_slot.to_le_bytes());
        dst[PLAYER_SKILLS_CREATED_AT_OFFSET..PLAYER_SKILLS_CREATED_AT_OFFSET + 8]
            .copy_from_slice(&created_at.to_le_bytes());
        Ok(())
    }

    pub fn validate(
        data: &[u8],
        owner: &Pubkey,
        global_config: &Pubkey,
    ) -> Result<Self, NicechunkSkillsError> {
        if data.len() != PLAYER_SKILLS_LEN
            || data[0..8] != PLAYER_SKILLS_MAGIC
            || read_u16(data, 8) != PLAYER_SKILLS_VERSION
            || data[11] != 1
        {
            return Err(NicechunkSkillsError::InvalidPlayerSkillsData);
        }
        if &data[PLAYER_SKILLS_OWNER_OFFSET..PLAYER_SKILLS_OWNER_OFFSET + 32] != owner.as_ref()
            || &data[PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET..PLAYER_SKILLS_GLOBAL_CONFIG_OFFSET + 32]
                != global_config.as_ref()
        {
            return Err(NicechunkSkillsError::InvalidPlayerSkillsData);
        }
        let mut xp = [0_u64; SKILL_COUNT];
        let mut levels = [0_u8; SKILL_COUNT];
        for index in 0..SKILL_COUNT {
            xp[index] = read_u64(data, PLAYER_SKILLS_XP_OFFSET + index * 8);
            levels[index] = data[PLAYER_SKILLS_LEVELS_OFFSET + index];
            if levels[index] > LEVEL_COUNT as u8 {
                return Err(NicechunkSkillsError::InvalidPlayerSkillsData);
            }
        }
        Ok(Self {
            xp,
            levels,
            cursor_mask: read_u32(data, PLAYER_SKILLS_CURSOR_MASK_OFFSET),
            rule_revision: read_u32(data, PLAYER_SKILLS_RULE_REVISION_OFFSET),
        })
    }

    pub fn apply_counter(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        rule_index: usize,
        rule: &SourceRule,
        current_counter: u64,
    ) -> Result<CounterApplyResult, NicechunkSkillsError> {
        if rule_index >= MAX_GENERIC_SOURCE_RULES {
            return Err(NicechunkSkillsError::InvalidRuleIndex);
        }
        let state = Self::validate(data, owner, global_config)?;
        let bit = 1_u32 << rule_index;
        let initialized = state.cursor_mask & bit != 0;
        let cursor_offset = PLAYER_SKILLS_CURSORS_OFFSET + rule_index * 8;
        let previous_cursor = read_u64(data, cursor_offset);
        if initialized && current_counter < previous_cursor {
            return Err(NicechunkSkillsError::SourceCounterRegressed);
        }

        let backfill = rule.flags & RULE_FLAG_BACKFILL_ON_FIRST_SYNC != 0;
        let available_delta = if initialized {
            current_counter.saturating_sub(previous_cursor)
        } else if backfill {
            current_counter
        } else {
            0
        };
        let applied_delta = available_delta.min(rule.max_delta_per_sync);
        let next_cursor = if !initialized && !backfill {
            current_counter
        } else {
            previous_cursor
                .checked_add(applied_delta)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?
        };

        let next_mask = state.cursor_mask | bit;
        data[PLAYER_SKILLS_CURSOR_MASK_OFFSET..PLAYER_SKILLS_CURSOR_MASK_OFFSET + 4]
            .copy_from_slice(&next_mask.to_le_bytes());
        data[cursor_offset..cursor_offset + 8].copy_from_slice(&next_cursor.to_le_bytes());

        if applied_delta == 0 {
            return Ok(CounterApplyResult {
                changed: !initialized || next_cursor != previous_cursor,
                applied_delta,
            });
        }

        for (skill_index, rate) in rule.xp_per_unit.iter().enumerate() {
            if *rate == 0 {
                continue;
            }
            let previous_scaled = (previous_cursor as u128)
                .checked_mul(*rate as u128)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?
                / rule.unit_divisor as u128;
            let next_scaled = (next_cursor as u128)
                .checked_mul(*rate as u128)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?
                / rule.unit_divisor as u128;
            let gained = next_scaled
                .checked_sub(previous_scaled)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
            let gained =
                u64::try_from(gained).map_err(|_| NicechunkSkillsError::ArithmeticOverflow)?;
            let xp_offset = PLAYER_SKILLS_XP_OFFSET + skill_index * 8;
            let next_xp = read_u64(data, xp_offset)
                .checked_add(gained)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
            data[xp_offset..xp_offset + 8].copy_from_slice(&next_xp.to_le_bytes());
        }
        Ok(CounterApplyResult {
            changed: true,
            applied_delta,
        })
    }

    pub fn apply_burden_mining_action(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        rule: BurdenMiningRule,
        pre_mine_mass_grams: u64,
        mine_sequence: u64,
    ) -> Result<CounterApplyResult, NicechunkSkillsError> {
        let state = Self::validate(data, owner, global_config)?;
        rule.validate()?;
        if !rule.enabled || mine_sequence == 0 {
            return Ok(CounterApplyResult {
                changed: false,
                applied_delta: 0,
            });
        }

        let sequence_bit = 1_u32 << BURDEN_SEQUENCE_CURSOR_INDEX;
        let work_bit = 1_u32 << BURDEN_WORK_CURSOR_INDEX;
        let sequence_initialized = state.cursor_mask & sequence_bit != 0;
        let sequence_offset = PLAYER_SKILLS_CURSORS_OFFSET + BURDEN_SEQUENCE_CURSOR_INDEX * 8;
        let previous_sequence = read_u64(data, sequence_offset);
        if sequence_initialized && mine_sequence < previous_sequence {
            return Err(NicechunkSkillsError::SourceCounterRegressed);
        }
        if sequence_initialized && mine_sequence == previous_sequence {
            return Ok(CounterApplyResult {
                changed: false,
                applied_delta: 0,
            });
        }

        let work_offset = PLAYER_SKILLS_CURSORS_OFFSET + BURDEN_WORK_CURSOR_INDEX * 8;
        let previous_work = if state.cursor_mask & work_bit != 0 {
            read_u64(data, work_offset)
        } else {
            0
        };
        let effective_mass = pre_mine_mass_grams.min(rule.max_effective_mass_grams);
        let next_work = previous_work
            .checked_add(effective_mass)
            .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
        let previous_xp_units = previous_work / rule.work_per_xp;
        let next_xp_units = next_work / rule.work_per_xp;
        let gained_xp = next_xp_units
            .checked_sub(previous_xp_units)
            .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
        if gained_xp > 0 {
            let xp_offset = PLAYER_SKILLS_XP_OFFSET + rule.skill_index as usize * 8;
            let next_xp = read_u64(data, xp_offset)
                .checked_add(gained_xp)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
            data[xp_offset..xp_offset + 8].copy_from_slice(&next_xp.to_le_bytes());
        }
        let next_mask = state.cursor_mask | work_bit | sequence_bit;
        data[PLAYER_SKILLS_CURSOR_MASK_OFFSET..PLAYER_SKILLS_CURSOR_MASK_OFFSET + 4]
            .copy_from_slice(&next_mask.to_le_bytes());
        data[work_offset..work_offset + 8].copy_from_slice(&next_work.to_le_bytes());
        data[sequence_offset..sequence_offset + 8].copy_from_slice(&mine_sequence.to_le_bytes());
        Ok(CounterApplyResult {
            changed: true,
            applied_delta: effective_mass,
        })
    }

    pub fn record_mining_coordinate(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        coordinate: MiningCoordinate,
        rule: MiningTravelRule,
        updated_slot: u64,
    ) -> Result<bool, NicechunkSkillsError> {
        Self::validate(data, owner, global_config)?;
        rule.validate()?;
        let has_previous =
            data[PLAYER_SKILLS_MINING_FLAGS_OFFSET] & PLAYER_SKILLS_FLAG_HAS_LAST_MINE != 0;
        let qualifies = rule.enabled
            && has_previous
            && mining_distance_reaches(
                MiningCoordinate {
                    x: read_i32(data, PLAYER_SKILLS_LAST_MINE_X_OFFSET),
                    y: read_i32(data, PLAYER_SKILLS_LAST_MINE_Y_OFFSET),
                    z: read_i32(data, PLAYER_SKILLS_LAST_MINE_Z_OFFSET),
                },
                coordinate,
                rule.minimum_distance,
            );
        if qualifies {
            let skill_index = rule.skill_index as usize;
            let xp_offset = PLAYER_SKILLS_XP_OFFSET + skill_index * 8;
            let next_xp = read_u64(data, xp_offset)
                .checked_add(rule.xp_award as u64)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
            data[xp_offset..xp_offset + 8].copy_from_slice(&next_xp.to_le_bytes());
            let next_count = read_u64(data, PLAYER_SKILLS_MINING_TRAVEL_COUNT_OFFSET)
                .checked_add(1)
                .ok_or(NicechunkSkillsError::ArithmeticOverflow)?;
            data[PLAYER_SKILLS_MINING_TRAVEL_COUNT_OFFSET
                ..PLAYER_SKILLS_MINING_TRAVEL_COUNT_OFFSET + 8]
                .copy_from_slice(&next_count.to_le_bytes());
        }
        data[PLAYER_SKILLS_LAST_MINE_X_OFFSET..PLAYER_SKILLS_LAST_MINE_X_OFFSET + 4]
            .copy_from_slice(&coordinate.x.to_le_bytes());
        data[PLAYER_SKILLS_LAST_MINE_Y_OFFSET..PLAYER_SKILLS_LAST_MINE_Y_OFFSET + 4]
            .copy_from_slice(&coordinate.y.to_le_bytes());
        data[PLAYER_SKILLS_LAST_MINE_Z_OFFSET..PLAYER_SKILLS_LAST_MINE_Z_OFFSET + 4]
            .copy_from_slice(&coordinate.z.to_le_bytes());
        data[PLAYER_SKILLS_MINING_FLAGS_OFFSET] |= PLAYER_SKILLS_FLAG_HAS_LAST_MINE;
        data[PLAYER_SKILLS_UPDATED_SLOT_OFFSET..PLAYER_SKILLS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(qualifies)
    }

    pub fn recompute_levels(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        rule_table_data: &[u8],
        updated_slot: u64,
    ) -> ProgramResult {
        Self::validate(data, owner, global_config)?;
        let rules = RuleTableState::validate(rule_table_data, global_config)?;
        for skill_index in 0..SKILL_COUNT {
            let xp = read_u64(data, PLAYER_SKILLS_XP_OFFSET + skill_index * 8);
            let mut level = 0_u8;
            let mut previous = 0_u64;
            for level_index in 0..LEVEL_COUNT {
                let threshold =
                    RuleTableState::threshold(rule_table_data, skill_index, level_index)?;
                if threshold == 0 || threshold <= previous {
                    return Err(NicechunkSkillsError::InvalidThresholds.into());
                }
                if xp >= threshold {
                    level = (level_index + 1) as u8;
                }
                previous = threshold;
            }
            data[PLAYER_SKILLS_LEVELS_OFFSET + skill_index] = level;
        }
        data[PLAYER_SKILLS_RULE_REVISION_OFFSET..PLAYER_SKILLS_RULE_REVISION_OFFSET + 4]
            .copy_from_slice(&rules.revision.to_le_bytes());
        data[PLAYER_SKILLS_UPDATED_SLOT_OFFSET..PLAYER_SKILLS_UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

fn validate_thresholds(thresholds: &[u64; LEVEL_COUNT]) -> ProgramResult {
    let mut previous = 0_u64;
    for threshold in thresholds {
        if *threshold == 0 || *threshold <= previous {
            return Err(NicechunkSkillsError::InvalidThresholds.into());
        }
        previous = *threshold;
    }
    Ok(())
}

fn threshold_offset(skill_index: usize, level_index: usize) -> usize {
    RULE_TABLE_THRESHOLDS_OFFSET + (skill_index * LEVEL_COUNT + level_index) * 8
}

fn rule_offset(rule_index: usize) -> usize {
    RULE_TABLE_RULES_OFFSET + rule_index * RULE_RECORD_LEN
}

fn mining_distance_reaches(
    previous: MiningCoordinate,
    current: MiningCoordinate,
    minimum_distance: u16,
) -> bool {
    let dx = current.x as i128 - previous.x as i128;
    let dy = current.y as i128 - previous.y as i128;
    let dz = current.z as i128 - previous.z as i128;
    let distance_squared = dx * dx + dy * dy + dz * dz;
    let minimum = minimum_distance as i128;
    distance_squared >= minimum * minimum
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey, NicechunkSkillsError> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(NicechunkSkillsError::InvalidRuleTableData)?
        .try_into()
        .map_err(|_| NicechunkSkillsError::InvalidRuleTableData)?;
    Ok(Pubkey::new_from_array(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rule(backfill: bool) -> SourceRule {
        let mut seed = [0_u8; SOURCE_SEED_MAX_LEN];
        seed[..15].copy_from_slice(b"player-progress");
        SourceRule {
            enabled: true,
            metric_width: 8,
            flags: if backfill {
                RULE_FLAG_BACKFILL_ON_FIRST_SYNC
            } else {
                0
            },
            seed_layout: SOURCE_SEED_GLOBAL_OWNER,
            rule_id: 1,
            source_program: Pubkey::new_unique(),
            source_magic: *b"NCKPRG01",
            seed,
            seed_len: 15,
            owner_offset: 12,
            global_config_offset: 44,
            metric_offset: 76,
            max_delta_per_sync: 1_000,
            unit_divisor: 1,
            xp_per_unit: [115, 18, 0, 0, 0, 5, 44, 9, 18, 22],
        }
    }

    fn initialized_rule_table(global_config: &Pubkey) -> Vec<u8> {
        let authority = Pubkey::new_unique();
        let mut data = vec![0_u8; RULE_TABLE_LEN];
        RuleTableState::pack_empty(&mut data, 1, &authority, global_config, 7, 8).unwrap();
        for skill_index in 0..SKILL_COUNT {
            let thresholds = [
                100, 250, 500, 1_000, 2_000, 4_000, 8_000, 16_000, 32_000, 64_000,
            ];
            RuleTableState::set_thresholds(
                &mut data,
                global_config,
                &authority,
                skill_index,
                &thresholds,
                9,
            )
            .unwrap();
        }
        data
    }

    #[test]
    fn layouts_match_declared_lengths() {
        assert_eq!(
            RULE_TABLE_RULES_OFFSET + MAX_SOURCE_RULES * RULE_RECORD_LEN,
            RULE_TABLE_LEN
        );
        assert_eq!(
            PLAYER_SKILLS_MINING_TRAVEL_COUNT_OFFSET + 8,
            PLAYER_SKILLS_LEN
        );
        assert_eq!(
            PLAYER_SKILLS_CURSORS_OFFSET + MAX_SOURCE_RULES * 8,
            PLAYER_SKILLS_CREATED_SLOT_OFFSET
        );
    }

    #[test]
    fn source_rule_round_trips() {
        let rule = sample_rule(true);
        let mut data = vec![0_u8; RULE_RECORD_LEN];
        rule.pack(&mut data).unwrap();
        assert_eq!(SourceRule::unpack(&data).unwrap(), rule);
    }

    #[test]
    fn first_sync_backfills_then_only_applies_delta() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();
        let rule = sample_rule(true);

        assert_eq!(
            PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 8)
                .unwrap()
                .applied_delta,
            8
        );
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET), 920);

        PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 10).unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET), 1_150);
    }

    #[test]
    fn first_sync_can_start_at_current_counter_without_backfill() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();
        let rule = sample_rule(false);

        PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 8).unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET), 0);
        assert_eq!(read_u64(&data, PLAYER_SKILLS_CURSORS_OFFSET), 8);
    }

    #[test]
    fn source_rule_fractional_progress_is_preserved_across_syncs() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();
        let mut rule = sample_rule(true);
        rule.unit_divisor = 100;
        rule.xp_per_unit = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 50).unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET), 0);
        PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 100).unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET), 1);
        PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 250).unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET), 2);
    }

    #[test]
    fn burden_rule_uses_reserved_record_without_consuming_generic_rule_slots() {
        let global_config = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mut table = vec![0_u8; RULE_TABLE_LEN];
        RuleTableState::pack_empty(&mut table, 1, &authority, &global_config, 2, 3).unwrap();
        let rule = BurdenMiningRule {
            enabled: true,
            skill_index: 1,
            max_effective_mass_grams: 100_000,
            work_per_xp: 100_000,
        };
        RuleTableState::set_burden_mining_rule(&mut table, &global_config, &authority, &rule, 4)
            .unwrap();

        assert_eq!(
            RuleTableState::burden_mining_rule(&table).unwrap(),
            Some(rule)
        );
        assert_eq!(table[RULE_TABLE_RULE_COUNT_OFFSET], 0);
        assert!(RuleTableState::upsert_rule(
            &mut table,
            &global_config,
            &authority,
            BURDEN_WORK_CURSOR_INDEX,
            &sample_rule(true),
            5,
        )
        .is_err());
    }

    #[test]
    fn burden_xp_accumulates_verified_pre_mine_mass_and_caps_each_action() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let rule = BurdenMiningRule {
            enabled: true,
            skill_index: 1,
            max_effective_mass_grams: 100_000,
            work_per_xp: 100_000,
        };
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();

        for sequence in 1..=4 {
            PlayerSkillsState::apply_burden_mining_action(
                &mut data,
                &owner,
                &global_config,
                rule,
                25_000,
                sequence,
            )
            .unwrap();
        }
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET + 8), 1);
        assert_eq!(
            read_u64(
                &data,
                PLAYER_SKILLS_CURSORS_OFFSET + BURDEN_WORK_CURSOR_INDEX * 8,
            ),
            100_000
        );

        PlayerSkillsState::apply_burden_mining_action(
            &mut data,
            &owner,
            &global_config,
            rule,
            50_000,
            5,
        )
        .unwrap();
        PlayerSkillsState::apply_burden_mining_action(
            &mut data,
            &owner,
            &global_config,
            rule,
            50_000,
            6,
        )
        .unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET + 8), 2);

        PlayerSkillsState::apply_burden_mining_action(
            &mut data,
            &owner,
            &global_config,
            rule,
            400_000,
            7,
        )
        .unwrap();
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET + 8), 3);
        let before = data.clone();
        assert!(
            !PlayerSkillsState::apply_burden_mining_action(
                &mut data,
                &owner,
                &global_config,
                rule,
                400_000,
                7,
            )
            .unwrap()
            .changed
        );
        assert_eq!(data, before);
    }

    #[test]
    fn burden_empty_load_gives_no_xp_but_consumes_the_mining_sequence() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let rule = BurdenMiningRule {
            enabled: true,
            skill_index: 1,
            max_effective_mass_grams: 100_000,
            work_per_xp: 100_000,
        };
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();
        let result = PlayerSkillsState::apply_burden_mining_action(
            &mut data,
            &owner,
            &global_config,
            rule,
            0,
            1,
        )
        .unwrap();
        assert!(result.changed);
        assert_eq!(result.applied_delta, 0);
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET + 8), 0);
        assert_eq!(
            read_u64(
                &data,
                PLAYER_SKILLS_CURSORS_OFFSET + BURDEN_SEQUENCE_CURSOR_INDEX * 8,
            ),
            1
        );
    }

    #[test]
    fn level_cache_uses_pda_thresholds() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut skills = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut skills, 1, &owner, &global_config, 2, 3).unwrap();
        skills[PLAYER_SKILLS_XP_OFFSET..PLAYER_SKILLS_XP_OFFSET + 8]
            .copy_from_slice(&1_000_u64.to_le_bytes());
        let rules = initialized_rule_table(&global_config);
        PlayerSkillsState::recompute_levels(&mut skills, &owner, &global_config, &rules, 11)
            .unwrap();
        assert_eq!(skills[PLAYER_SKILLS_LEVELS_OFFSET], 4);
    }

    #[test]
    fn source_counter_regression_is_rejected() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();
        let rule = sample_rule(true);
        PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 8).unwrap();
        assert_eq!(
            PlayerSkillsState::apply_counter(&mut data, &owner, &global_config, 0, &rule, 7,),
            Err(NicechunkSkillsError::SourceCounterRegressed)
        );
    }

    #[test]
    fn mining_travel_awards_only_at_160_blocks_and_updates_previous_coordinate() {
        let owner = Pubkey::new_unique();
        let global_config = Pubkey::new_unique();
        let mut data = vec![0_u8; PLAYER_SKILLS_LEN];
        PlayerSkillsState::pack_empty(&mut data, 1, &owner, &global_config, 2, 3).unwrap();
        let rule = MiningTravelRule {
            enabled: true,
            minimum_distance: 160,
            skill_index: 5,
            xp_award: 1,
        };

        assert!(!PlayerSkillsState::record_mining_coordinate(
            &mut data,
            &owner,
            &global_config,
            MiningCoordinate { x: 0, y: 90, z: 0 },
            rule,
            4,
        )
        .unwrap());
        assert!(!PlayerSkillsState::record_mining_coordinate(
            &mut data,
            &owner,
            &global_config,
            MiningCoordinate {
                x: 159,
                y: 90,
                z: 0
            },
            rule,
            5,
        )
        .unwrap());
        assert!(PlayerSkillsState::record_mining_coordinate(
            &mut data,
            &owner,
            &global_config,
            MiningCoordinate {
                x: 319,
                y: 90,
                z: 0
            },
            rule,
            6,
        )
        .unwrap());
        assert_eq!(read_u64(&data, PLAYER_SKILLS_XP_OFFSET + 5 * 8), 1);
        assert_eq!(read_u64(&data, PLAYER_SKILLS_MINING_TRAVEL_COUNT_OFFSET), 1);
    }

    #[test]
    fn mining_travel_uses_three_dimensional_distance() {
        assert!(mining_distance_reaches(
            MiningCoordinate { x: 0, y: 0, z: 0 },
            MiningCoordinate {
                x: 96,
                y: 128,
                z: 0
            },
            160,
        ));
        assert!(!mining_distance_reaches(
            MiningCoordinate { x: 0, y: 0, z: 0 },
            MiningCoordinate {
                x: 95,
                y: 128,
                z: 0
            },
            160,
        ));
    }
}
