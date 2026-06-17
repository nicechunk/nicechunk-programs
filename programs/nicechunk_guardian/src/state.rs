use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkGuardianError;

pub const GUARDIAN_REGISTRY_MAGIC: [u8; 8] = *b"NCKGDR01";
pub const GUARDIAN_REGION_MAGIC: [u8; 8] = *b"NCKGRG01";
pub const GUARDIAN_REGISTRY_VERSION: u16 = 1;
pub const GUARDIAN_REGION_VERSION: u16 = 1;
pub const GUARDIAN_REGISTRY_SEED: &[u8] = b"guardian-registry";
pub const GUARDIAN_REGION_SEED: &[u8] = b"guardian-region";
pub const GUARDIAN_TREASURY_AUTHORITY_SEED: &[u8] = b"guardian-treasury";
pub const REGION_SIZE_CHUNKS: i32 = 100;
pub const REGION_SIZE_CHUNKS_U16: u16 = 100;
pub const GUARDIAN_STAKE_AMOUNT: u64 = 100_000_000_000;
pub const GUARDIAN_SLASH_AMOUNT: u64 = GUARDIAN_STAKE_AMOUNT / 10;
pub const PROOF_INTERVAL_SECONDS: i64 = 3_600;
pub const MAX_HOST_LEN: usize = 64;
pub const REGION_STATUS_EMPTY: u8 = 0;
pub const REGION_STATUS_ACTIVE: u8 = 1;
pub const REGION_STATUS_REMOVED: u8 = 2;

pub struct GuardianRegistry;

impl GuardianRegistry {
    pub const LEN: usize = 160;
    pub const GLOBAL_CONFIG_OFFSET: usize = 12;
    pub const TREASURY_TOKEN_OFFSET: usize = 76;
    pub const ACTIVE_COUNT_OFFSET: usize = 116;
    pub const GENESIS_REGISTERED_OFFSET: usize = 124;

    #[allow(clippy::too_many_arguments)]
    pub fn pack(
        dst: &mut [u8],
        bump: u8,
        treasury_bump: u8,
        global_config: &Pubkey,
        nck_mint: &Pubkey,
        treasury_token: &Pubkey,
        created_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidRegistryData.into());
        }

        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&GUARDIAN_REGISTRY_MAGIC)?;
        writer.u16(GUARDIAN_REGISTRY_VERSION)?;
        writer.u8(bump)?;
        writer.u8(treasury_bump)?;
        writer.pubkey(global_config)?;
        writer.pubkey(nck_mint)?;
        writer.pubkey(treasury_token)?;
        writer.u64(0)?;
        writer.u64(0)?;
        writer.u8(0)?;
        writer.u8(0)?;
        writer.u16(REGION_SIZE_CHUNKS_U16)?;
        writer.u64(GUARDIAN_STAKE_AMOUNT)?;
        writer.u64(GUARDIAN_SLASH_AMOUNT)?;
        writer.u64(created_slot)?;
        writer.i64(created_at)?;

        if writer.offset != Self::LEN {
            return Err(NicechunkGuardianError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate(data: &[u8], global_config: &Pubkey) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != GUARDIAN_REGISTRY_MAGIC {
            return Err(NicechunkGuardianError::InvalidRegistryData.into());
        }
        if &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
            != global_config.as_ref()
        {
            return Err(NicechunkGuardianError::InvalidRegistryData.into());
        }
        Ok(())
    }

    pub fn treasury_token(data: &[u8]) -> Result<Pubkey, NicechunkGuardianError> {
        if data.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidRegistryData);
        }
        Ok(Pubkey::new_from_array(
            data[Self::TREASURY_TOKEN_OFFSET..Self::TREASURY_TOKEN_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkGuardianError::InvalidRegistryData)?,
        ))
    }

    pub fn genesis_registered(data: &[u8]) -> Result<bool, NicechunkGuardianError> {
        if data.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidRegistryData);
        }
        Ok(data[Self::GENESIS_REGISTERED_OFFSET] == 1)
    }

    pub fn add_active(data: &mut [u8], is_genesis: bool) -> ProgramResult {
        if data.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidRegistryData.into());
        }
        let active = read_u64(data, Self::ACTIVE_COUNT_OFFSET)
            .checked_add(1)
            .ok_or(NicechunkGuardianError::InvalidRegistryData)?;
        let total = read_u64(data, Self::ACTIVE_COUNT_OFFSET + 8)
            .checked_add(1)
            .ok_or(NicechunkGuardianError::InvalidRegistryData)?;
        data[Self::ACTIVE_COUNT_OFFSET..Self::ACTIVE_COUNT_OFFSET + 8]
            .copy_from_slice(&active.to_le_bytes());
        data[Self::ACTIVE_COUNT_OFFSET + 8..Self::ACTIVE_COUNT_OFFSET + 16]
            .copy_from_slice(&total.to_le_bytes());
        if is_genesis {
            data[Self::GENESIS_REGISTERED_OFFSET] = 1;
        }
        Ok(())
    }

    pub fn remove_active(data: &mut [u8]) -> ProgramResult {
        if data.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidRegistryData.into());
        }
        let active = read_u64(data, Self::ACTIVE_COUNT_OFFSET).saturating_sub(1);
        data[Self::ACTIVE_COUNT_OFFSET..Self::ACTIVE_COUNT_OFFSET + 8]
            .copy_from_slice(&active.to_le_bytes());
        Ok(())
    }
}

pub struct GuardianRegionInitArgs<'a> {
    pub bump: u8,
    pub status: u8,
    pub region_x: i32,
    pub region_y: i32,
    pub owner: &'a Pubkey,
    pub operator: &'a Pubkey,
    pub global_config: &'a Pubkey,
    pub host: &'a [u8],
    pub port: u16,
    pub use_tls: bool,
    pub created_slot: u64,
    pub created_at: i64,
}

pub struct GuardianRegion;

impl GuardianRegion {
    pub const LEN: usize = 256;
    pub const STATUS_OFFSET: usize = 11;
    pub const REGION_X_OFFSET: usize = 12;
    pub const REGION_Y_OFFSET: usize = 16;
    pub const OWNER_OFFSET: usize = 36;
    pub const OPERATOR_OFFSET: usize = 68;
    pub const GLOBAL_CONFIG_OFFSET: usize = 100;
    pub const HOST_LEN_OFFSET: usize = 132;
    pub const HOST_OFFSET: usize = 133;
    pub const PORT_OFFSET: usize = 197;
    pub const USE_TLS_OFFSET: usize = 199;
    pub const STAKE_AMOUNT_OFFSET: usize = 200;
    pub const TOTAL_SLASHED_OFFSET: usize = 208;
    pub const PENALTY_COUNT_OFFSET: usize = 216;
    pub const REGISTERED_AT_OFFSET: usize = 220;
    pub const LAST_PROOF_AT_OFFSET: usize = 228;
    pub const PENALTY_CURSOR_AT_OFFSET: usize = 236;
    pub const PROOF_COUNT_OFFSET: usize = 244;
    pub const UPDATED_SLOT_OFFSET: usize = 252;

    pub fn pack(dst: &mut [u8], args: &GuardianRegionInitArgs) -> ProgramResult {
        if dst.len() != Self::LEN || args.host.len() > MAX_HOST_LEN {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData.into());
        }

        let mut host_buf = [0_u8; MAX_HOST_LEN];
        host_buf[..args.host.len()].copy_from_slice(args.host);

        let min_chunk_x = args.region_x.saturating_mul(REGION_SIZE_CHUNKS);
        let min_chunk_y = args.region_y.saturating_mul(REGION_SIZE_CHUNKS);
        let max_chunk_x = min_chunk_x.saturating_add(REGION_SIZE_CHUNKS - 1);
        let max_chunk_y = min_chunk_y.saturating_add(REGION_SIZE_CHUNKS - 1);

        let mut writer = ByteWriter { dst, offset: 0 };
        writer.bytes(&GUARDIAN_REGION_MAGIC)?;
        writer.u16(GUARDIAN_REGION_VERSION)?;
        writer.u8(args.bump)?;
        writer.u8(args.status)?;
        writer.i32(args.region_x)?;
        writer.i32(args.region_y)?;
        writer.i32(min_chunk_x)?;
        writer.i32(min_chunk_y)?;
        writer.i32(max_chunk_x)?;
        writer.i32(max_chunk_y)?;
        writer.pubkey(args.owner)?;
        writer.pubkey(args.operator)?;
        writer.pubkey(args.global_config)?;
        writer.u8(args.host.len() as u8)?;
        writer.bytes(&host_buf)?;
        writer.u16(args.port)?;
        writer.u8(u8::from(args.use_tls))?;
        writer.u64(GUARDIAN_STAKE_AMOUNT)?;
        writer.u64(0)?;
        writer.u32(0)?;
        writer.i64(args.created_at)?;
        writer.i64(args.created_at)?;
        writer.i64(args.created_at)?;
        writer.u64(0)?;
        writer.u32(args.created_slot as u32)?;

        if writer.offset != Self::LEN {
            return Err(NicechunkGuardianError::PackSizeMismatch.into());
        }
        Ok(())
    }

    pub fn validate_active(
        data: &[u8],
        global_config: &Pubkey,
        region_x: i32,
        region_y: i32,
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != GUARDIAN_REGION_MAGIC {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData.into());
        }
        if data[Self::STATUS_OFFSET] != REGION_STATUS_ACTIVE {
            return Err(NicechunkGuardianError::GuardianNotActive.into());
        }
        if read_i32(data, Self::REGION_X_OFFSET) != region_x
            || read_i32(data, Self::REGION_Y_OFFSET) != region_y
            || &data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
                != global_config.as_ref()
        {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData.into());
        }
        Ok(())
    }

    pub fn status(data: &[u8]) -> Result<u8, NicechunkGuardianError> {
        if data.len() != Self::LEN || data[0..8] != GUARDIAN_REGION_MAGIC {
            return Ok(REGION_STATUS_EMPTY);
        }
        Ok(data[Self::STATUS_OFFSET])
    }

    pub fn operator(data: &[u8]) -> Result<Pubkey, NicechunkGuardianError> {
        if data.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData);
        }
        Ok(Pubkey::new_from_array(
            data[Self::OPERATOR_OFFSET..Self::OPERATOR_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkGuardianError::InvalidGuardianRegionData)?,
        ))
    }

    pub fn owner(data: &[u8]) -> Result<Pubkey, NicechunkGuardianError> {
        if data.len() != Self::LEN {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData);
        }
        Ok(Pubkey::new_from_array(
            data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32]
                .try_into()
                .map_err(|_| NicechunkGuardianError::InvalidGuardianRegionData)?,
        ))
    }

    pub fn update_endpoint(
        data: &mut [u8],
        owner: &Pubkey,
        host: &[u8],
        port: u16,
        use_tls: bool,
        slot: u64,
    ) -> ProgramResult {
        if data.len() != Self::LEN || data[0..8] != GUARDIAN_REGION_MAGIC {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData.into());
        }
        if data[Self::STATUS_OFFSET] != REGION_STATUS_ACTIVE {
            return Err(NicechunkGuardianError::GuardianNotActive.into());
        }
        if &Self::owner(data)? != owner {
            return Err(NicechunkGuardianError::InvalidGuardianOwner.into());
        }
        if host.len() > MAX_HOST_LEN {
            return Err(NicechunkGuardianError::InvalidHost.into());
        }

        let mut host_buf = [0_u8; MAX_HOST_LEN];
        host_buf[..host.len()].copy_from_slice(host);
        data[Self::HOST_LEN_OFFSET] = host.len() as u8;
        data[Self::HOST_OFFSET..Self::HOST_OFFSET + MAX_HOST_LEN].copy_from_slice(&host_buf);
        data[Self::PORT_OFFSET..Self::PORT_OFFSET + 2].copy_from_slice(&port.to_le_bytes());
        data[Self::USE_TLS_OFFSET] = u8::from(use_tls);
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 4]
            .copy_from_slice(&(slot as u32).to_le_bytes());
        Ok(())
    }

    pub fn settle(data: &mut [u8], now: i64) -> Result<bool, NicechunkGuardianError> {
        if data.len() != Self::LEN || data[0..8] != GUARDIAN_REGION_MAGIC {
            return Err(NicechunkGuardianError::InvalidGuardianRegionData);
        }
        if data[Self::STATUS_OFFSET] != REGION_STATUS_ACTIVE {
            return Ok(false);
        }

        let cursor = read_i64(data, Self::PENALTY_CURSOR_AT_OFFSET);
        if now <= cursor + PROOF_INTERVAL_SECONDS {
            return Ok(false);
        }

        let missed = ((now - cursor) / PROOF_INTERVAL_SECONDS) as u64;
        let slash_amount = missed.saturating_mul(GUARDIAN_SLASH_AMOUNT);
        let stake = read_u64(data, Self::STAKE_AMOUNT_OFFSET);
        let slash = slash_amount.min(stake);
        let next_stake = stake.saturating_sub(slash);
        let total_slashed = read_u64(data, Self::TOTAL_SLASHED_OFFSET).saturating_add(slash);
        let penalty_count = read_u32(data, Self::PENALTY_COUNT_OFFSET)
            .saturating_add(u32::try_from(missed).unwrap_or(u32::MAX));
        let next_cursor = cursor.saturating_add(
            i64::try_from(missed)
                .unwrap_or(i64::MAX)
                .saturating_mul(PROOF_INTERVAL_SECONDS),
        );

        data[Self::STAKE_AMOUNT_OFFSET..Self::STAKE_AMOUNT_OFFSET + 8]
            .copy_from_slice(&next_stake.to_le_bytes());
        data[Self::TOTAL_SLASHED_OFFSET..Self::TOTAL_SLASHED_OFFSET + 8]
            .copy_from_slice(&total_slashed.to_le_bytes());
        data[Self::PENALTY_COUNT_OFFSET..Self::PENALTY_COUNT_OFFSET + 4]
            .copy_from_slice(&penalty_count.to_le_bytes());
        data[Self::PENALTY_CURSOR_AT_OFFSET..Self::PENALTY_CURSOR_AT_OFFSET + 8]
            .copy_from_slice(&next_cursor.to_le_bytes());

        if next_stake == 0 {
            data[Self::STATUS_OFFSET] = REGION_STATUS_REMOVED;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn proof(data: &mut [u8], operator: &Pubkey, now: i64, slot: u64) -> ProgramResult {
        Self::validate_active(
            data,
            &Pubkey::new_from_array(
                data[Self::GLOBAL_CONFIG_OFFSET..Self::GLOBAL_CONFIG_OFFSET + 32]
                    .try_into()
                    .map_err(|_| NicechunkGuardianError::InvalidGuardianRegionData)?,
            ),
            read_i32(data, Self::REGION_X_OFFSET),
            read_i32(data, Self::REGION_Y_OFFSET),
        )?;
        if &Self::operator(data)? != operator {
            return Err(NicechunkGuardianError::InvalidOperatorAuthority.into());
        }
        let proof_count = read_u64(data, Self::PROOF_COUNT_OFFSET).saturating_add(1);
        data[Self::LAST_PROOF_AT_OFFSET..Self::LAST_PROOF_AT_OFFSET + 8]
            .copy_from_slice(&now.to_le_bytes());
        data[Self::PENALTY_CURSOR_AT_OFFSET..Self::PENALTY_CURSOR_AT_OFFSET + 8]
            .copy_from_slice(&now.to_le_bytes());
        data[Self::PROOF_COUNT_OFFSET..Self::PROOF_COUNT_OFFSET + 8]
            .copy_from_slice(&proof_count.to_le_bytes());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 4]
            .copy_from_slice(&(slot as u32).to_le_bytes());
        Ok(())
    }
}

pub fn validate_host(host: &[u8]) -> ProgramResult {
    if host.is_empty() || host.len() > MAX_HOST_LEN {
        return Err(NicechunkGuardianError::InvalidHost.into());
    }
    for byte in host {
        let valid = byte.is_ascii_alphanumeric()
            || matches!(*byte, b'.' | b'-' | b'_' | b':' | b'[' | b']');
        if !valid {
            return Err(NicechunkGuardianError::InvalidHost.into());
        }
    }
    Ok(())
}

pub fn validate_port(port: u16) -> ProgramResult {
    if port == 0 {
        return Err(NicechunkGuardianError::InvalidPort.into());
    }
    Ok(())
}

pub fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn read_i64(data: &[u8], offset: usize) -> i64 {
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

pub fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn read_u64(data: &[u8], offset: usize) -> u64 {
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

struct ByteWriter<'a> {
    dst: &'a mut [u8],
    offset: usize,
}

impl ByteWriter<'_> {
    fn bytes(&mut self, value: &[u8]) -> ProgramResult {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or(NicechunkGuardianError::InvalidGuardianRegionData)?;
        if end > self.dst.len() {
            return Err(NicechunkGuardianError::PackSizeMismatch.into());
        }
        self.dst[self.offset..end].copy_from_slice(value);
        self.offset = end;
        Ok(())
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

    fn i32(&mut self, value: i32) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn pubkey(&mut self, value: &Pubkey) -> ProgramResult {
        self.bytes(value.as_ref())
    }
}
