use solana_program::{entrypoint::ProgramResult, hash::hashv, pubkey::Pubkey};

use crate::{errors::NicechunkBuildingError, state::GlobalConfigView};

pub const BUILD_SITE_MAGIC: [u8; 8] = *b"NCKSITE3";
pub const BUILD_SITE_VERSION: u8 = 3;
pub const BUILD_SITE_SEED: &[u8] = b"build-site-v3";
pub const BUILD_SITE_LEN: usize = 160;
pub const BUILD_SITE_STATUS_INDEXING: u8 = 0;
pub const BUILD_SITE_STATUS_ACTIVE: u8 = 1;
pub const BUILD_SITE_STATUS_CANCELING: u8 = 2;
pub const CHUNK_AUTHORITY_SEED: &[u8] = b"chunk-authority-v2";
pub const LAND_CONTRACT_TYPE_BLANK: u8 = 1;
pub const MAX_LAND_CONTRACTS_PER_SITE: u32 = 4_096;

pub const BUILDING_MANIFEST_MAGIC: [u8; 8] = *b"NCKBLD03";
pub const BUILDING_MANIFEST_VERSION: u8 = 3;
pub const BUILDING_MANIFEST_SEED: &[u8] = b"building-v3";
pub const BUILDING_MANIFEST_LEN: usize = 160;
pub const BUILDING_STATUS_UPLOADING: u8 = 0;
pub const BUILDING_STATUS_ACTIVE: u8 = 1;

pub const BUILDING_SHARD_MAGIC: [u8; 8] = *b"NCKBDT02";
pub const BUILDING_SHARD_VERSION: u8 = 2;
pub const BUILDING_SHARD_SEED: &[u8] = b"building-data-v2";
pub const BUILDING_SHARD_HEADER_LEN: usize = 64;
pub const BUILDING_SHARD_PAYLOAD_LEN: usize = 8_192;
pub const BUILDING_MAX_PAYLOAD_LEN: usize = 65_535;
pub const BUILDING_MAX_SHARDS: usize = 8;
pub const BUILDING_MAX_WRITE_LEN: usize = 700;

const FOUNDATION_CHUNK_SIZE: u32 = 16;
const NCM3_MAX_DIMENSION: u32 = 256;
const NCM3_MAX_COMMANDS: u32 = 4_096;
const NCM3_OPERATION_BUDGET: u64 = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateBuildSiteArgs {
    pub foundation_id: u64,
    pub min_x: i32,
    pub surface_y: i16,
    pub min_z: i32,
    pub width: u32,
    pub depth: u32,
}

impl CreateBuildSiteArgs {
    pub const LEN: usize = 26;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkBuildingError> {
        if data.len() != Self::LEN {
            return Err(NicechunkBuildingError::InvalidBuildSiteData);
        }
        Ok(Self {
            foundation_id: read_u64(data, 0),
            min_x: read_i32(data, 8),
            surface_y: read_i16(data, 12),
            min_z: read_i32(data, 14),
            width: read_u32(data, 18),
            depth: read_u32(data, 22),
        })
    }

    pub fn validate(&self, config: &GlobalConfigView) -> Result<(), NicechunkBuildingError> {
        let chunk_size = u32::from(config.chunk_size);
        if self.foundation_id == 0
            || chunk_size != FOUNDATION_CHUNK_SIZE
            || self.width < chunk_size
            || self.depth < chunk_size
            || self.width % chunk_size != 0
            || self.depth % chunk_size != 0
            || self.min_x.rem_euclid(i32::from(config.chunk_size)) != 0
            || self.min_z.rem_euclid(i32::from(config.chunk_size)) != 0
            || self.surface_y <= config.min_build_y
            || self.surface_y > config.max_build_y
            || self.max_x().is_none()
            || self.max_z().is_none()
            || self.required_land_contracts().is_err()
        {
            return Err(NicechunkBuildingError::InvalidBuildSiteData);
        }
        Ok(())
    }

    pub fn max_x(&self) -> Option<i32> {
        checked_axis_end(self.min_x, self.width)
    }

    pub fn max_z(&self) -> Option<i32> {
        checked_axis_end(self.min_z, self.depth)
    }

    pub fn required_land_contracts(&self) -> Result<u32, NicechunkBuildingError> {
        if self.width < FOUNDATION_CHUNK_SIZE
            || self.depth < FOUNDATION_CHUNK_SIZE
            || self.width % FOUNDATION_CHUNK_SIZE != 0
            || self.depth % FOUNDATION_CHUNK_SIZE != 0
        {
            return Err(NicechunkBuildingError::InvalidBuildSiteData);
        }
        (self.width / FOUNDATION_CHUNK_SIZE)
            .checked_mul(self.depth / FOUNDATION_CHUNK_SIZE)
            .filter(|count| *count > 0 && *count <= MAX_LAND_CONTRACTS_PER_SITE)
            .ok_or(NicechunkBuildingError::InvalidBuildSiteData)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildSiteView {
    pub status: u8,
    pub contract_type: u8,
    pub land_contract_count: u32,
    pub owner: Pubkey,
    pub global_config: Pubkey,
    pub foundation_id: u64,
    pub min_x: i32,
    pub min_z: i32,
    pub surface_y: i16,
    pub width: u32,
    pub depth: u32,
    pub created_slot: u64,
    pub active_revision: u32,
    pub pending_revision: u32,
    pub updated_slot: u64,
    pub registered_chunks: u64,
    pub total_chunks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundationIndexOperation {
    Upsert = 0,
    Remove = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundationIndexStep {
    pub operation: FoundationIndexOperation,
    pub foundation: CreateBuildSiteArgs,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

pub struct BuildSiteState;

impl BuildSiteState {
    pub fn pack(
        dst: &mut [u8],
        bump: u8,
        owner: &Pubkey,
        global_config: &Pubkey,
        args: &CreateBuildSiteArgs,
        created_slot: u64,
    ) -> ProgramResult {
        if dst.len() != BUILD_SITE_LEN || foundation_chunk_span(args).is_err() {
            return Err(NicechunkBuildingError::InvalidBuildSiteData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&BUILD_SITE_MAGIC);
        dst[8] = BUILD_SITE_VERSION;
        dst[9] = bump;
        dst[10] = BUILD_SITE_STATUS_INDEXING;
        dst[11] = LAND_CONTRACT_TYPE_BLANK;
        dst[12..16].copy_from_slice(&args.required_land_contracts()?.to_le_bytes());
        dst[16..48].copy_from_slice(owner.as_ref());
        dst[48..80].copy_from_slice(global_config.as_ref());
        dst[80..88].copy_from_slice(&args.foundation_id.to_le_bytes());
        dst[88..92].copy_from_slice(&args.min_x.to_le_bytes());
        dst[92..96].copy_from_slice(&args.min_z.to_le_bytes());
        dst[96..98].copy_from_slice(&args.surface_y.to_le_bytes());
        dst[100..104].copy_from_slice(&args.width.to_le_bytes());
        dst[104..108].copy_from_slice(&args.depth.to_le_bytes());
        dst[108..116].copy_from_slice(&created_slot.to_le_bytes());
        dst[124..132].copy_from_slice(&created_slot.to_le_bytes());
        dst[140..148].copy_from_slice(&foundation_chunk_count(args)?.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8]) -> Result<BuildSiteView, NicechunkBuildingError> {
        if data.len() != BUILD_SITE_LEN
            || data[0..8] != BUILD_SITE_MAGIC
            || data[8] != BUILD_SITE_VERSION
            || data[10] > BUILD_SITE_STATUS_CANCELING
        {
            return Err(NicechunkBuildingError::InvalidBuildSiteData);
        }
        let view = BuildSiteView {
            status: data[10],
            contract_type: data[11],
            land_contract_count: read_u32(data, 12),
            owner: read_pubkey(data, 16, NicechunkBuildingError::InvalidBuildSiteData)?,
            global_config: read_pubkey(data, 48, NicechunkBuildingError::InvalidBuildSiteData)?,
            foundation_id: read_u64(data, 80),
            min_x: read_i32(data, 88),
            min_z: read_i32(data, 92),
            surface_y: read_i16(data, 96),
            width: read_u32(data, 100),
            depth: read_u32(data, 104),
            created_slot: read_u64(data, 108),
            active_revision: read_u32(data, 116),
            pending_revision: read_u32(data, 120),
            updated_slot: read_u64(data, 124),
            registered_chunks: read_u64(data, 132),
            total_chunks: read_u64(data, 140),
        };
        let args = Self::active_args(&view);
        let active_chunks = foundation_chunk_count(&args)?;
        if view.foundation_id == 0
            || view.contract_type != LAND_CONTRACT_TYPE_BLANK
            || view.width < FOUNDATION_CHUNK_SIZE
            || view.depth < FOUNDATION_CHUNK_SIZE
            || view.width % FOUNDATION_CHUNK_SIZE != 0
            || view.depth % FOUNDATION_CHUNK_SIZE != 0
            || view.min_x.rem_euclid(FOUNDATION_CHUNK_SIZE as i32) != 0
            || view.min_z.rem_euclid(FOUNDATION_CHUNK_SIZE as i32) != 0
            || view.land_contract_count != args.required_land_contracts()?
            || data[98..100].iter().any(|byte| *byte != 0)
            || checked_axis_end(view.min_x, view.width).is_none()
            || checked_axis_end(view.min_z, view.depth).is_none()
            || view.pending_revision != 0
                && view.pending_revision != view.active_revision.saturating_add(1)
            || view.registered_chunks > view.total_chunks
            || data[148..160].iter().any(|byte| *byte != 0)
        {
            return Err(NicechunkBuildingError::InvalidBuildSiteData.into());
        }
        match view.status {
            BUILD_SITE_STATUS_INDEXING => {
                if view.active_revision != 0
                    || view.pending_revision != 0
                    || view.total_chunks != active_chunks
                    || view.registered_chunks == view.total_chunks
                {
                    return Err(NicechunkBuildingError::InvalidBuildSiteData);
                }
            }
            BUILD_SITE_STATUS_ACTIVE => {
                if view.total_chunks != active_chunks || view.registered_chunks != view.total_chunks
                {
                    return Err(NicechunkBuildingError::InvalidBuildSiteData);
                }
            }
            BUILD_SITE_STATUS_CANCELING => {
                if view.active_revision != 0
                    || view.pending_revision != 0
                    || view.total_chunks != active_chunks
                {
                    return Err(NicechunkBuildingError::InvalidBuildSiteData);
                }
            }
            _ => return Err(NicechunkBuildingError::InvalidBuildSiteData),
        }
        Ok(view)
    }

    pub fn advance_indexing(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        count: u64,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        let registered = view
            .registered_chunks
            .checked_add(count)
            .ok_or(NicechunkBuildingError::InvalidBuildSiteData)?;
        if view.owner != *owner
            || view.global_config != *global_config
            || count == 0
            || registered > view.total_chunks
        {
            return Err(NicechunkBuildingError::InvalidBuildSiteData.into());
        }
        data[132..140].copy_from_slice(&registered.to_le_bytes());
        data[124..132].copy_from_slice(&updated_slot.to_le_bytes());
        if registered != view.total_chunks {
            return Ok(());
        }
        if view.status != BUILD_SITE_STATUS_INDEXING {
            return Err(NicechunkBuildingError::InvalidBuildSiteData.into());
        }
        Self::finish_indexing(data, &view)
    }

    fn finish_indexing(data: &mut [u8], view: &BuildSiteView) -> ProgramResult {
        let active = Self::active_args_from_data(
            data,
            view.foundation_id,
            view.min_x,
            view.min_z,
            view.surface_y,
        );
        let active_chunks = foundation_chunk_count(&active)?;
        data[10] = BUILD_SITE_STATUS_ACTIVE;
        data[132..140].copy_from_slice(&active_chunks.to_le_bytes());
        data[140..148].copy_from_slice(&active_chunks.to_le_bytes());
        data[148..160].fill(0);
        Ok(())
    }

    pub fn index_step(
        view: &BuildSiteView,
        index: u64,
    ) -> Result<FoundationIndexStep, NicechunkBuildingError> {
        if index >= view.total_chunks {
            return Err(NicechunkBuildingError::InvalidBuildSiteData);
        }
        if view.status != BUILD_SITE_STATUS_INDEXING {
            return Err(NicechunkBuildingError::InvalidBuildSiteData);
        }
        let foundation = Self::active_args(view);
        let operation = FoundationIndexOperation::Upsert;
        let chunk = foundation_chunk_at(&foundation, index)?;
        Ok(FoundationIndexStep {
            operation,
            foundation,
            chunk_x: chunk.0,
            chunk_z: chunk.1,
        })
    }

    pub fn chunk_at(
        view: &BuildSiteView,
        index: u64,
    ) -> Result<(i32, i32), NicechunkBuildingError> {
        foundation_chunk_at(&Self::active_args(view), index)
    }

    pub fn begin_canceling(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        if view.owner != *owner
            || view.global_config != *global_config
            || view.status != BUILD_SITE_STATUS_INDEXING
                && view.status != BUILD_SITE_STATUS_CANCELING
        {
            return Err(NicechunkBuildingError::BuildSiteNotCancelable.into());
        }
        data[10] = BUILD_SITE_STATUS_CANCELING;
        data[124..132].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn rewind_indexing(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        count: u64,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        let registered = view
            .registered_chunks
            .checked_sub(count)
            .ok_or(NicechunkBuildingError::InvalidBuildSiteData)?;
        if view.owner != *owner
            || view.global_config != *global_config
            || view.status != BUILD_SITE_STATUS_CANCELING
            || count == 0
        {
            return Err(NicechunkBuildingError::InvalidBuildSiteData.into());
        }
        data[132..140].copy_from_slice(&registered.to_le_bytes());
        data[124..132].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn matches_creation(
        data: &[u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        args: &CreateBuildSiteArgs,
    ) -> Result<bool, NicechunkBuildingError> {
        let view = Self::validate(data)?;
        Ok(view.status != BUILD_SITE_STATUS_CANCELING
            && view.owner == *owner
            && view.global_config == *global_config
            && view.foundation_id == args.foundation_id
            && view.min_x == args.min_x
            && view.min_z == args.min_z
            && view.surface_y == args.surface_y
            && view.width == args.width
            && view.depth == args.depth)
    }

    pub fn active_args(view: &BuildSiteView) -> CreateBuildSiteArgs {
        CreateBuildSiteArgs {
            foundation_id: view.foundation_id,
            min_x: view.min_x,
            surface_y: view.surface_y,
            min_z: view.min_z,
            width: view.width,
            depth: view.depth,
        }
    }

    fn active_args_from_data(
        data: &[u8],
        foundation_id: u64,
        min_x: i32,
        min_z: i32,
        surface_y: i16,
    ) -> CreateBuildSiteArgs {
        CreateBuildSiteArgs {
            foundation_id,
            min_x,
            min_z,
            surface_y,
            width: read_u32(data, 100),
            depth: read_u32(data, 104),
        }
    }

    pub fn begin_building(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        revision: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        if &view.owner != owner
            || &view.global_config != global_config
            || view.status != BUILD_SITE_STATUS_ACTIVE
            || view.pending_revision != 0
            || revision == 0
            || revision != view.active_revision.saturating_add(1)
        {
            return Err(NicechunkBuildingError::BuildingRevisionConflict.into());
        }
        data[120..124].copy_from_slice(&revision.to_le_bytes());
        data[124..132].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn activate_building(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        revision: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        if &view.owner != owner
            || &view.global_config != global_config
            || view.pending_revision != revision
            || revision == 0
        {
            return Err(NicechunkBuildingError::BuildingRevisionConflict.into());
        }
        data[116..120].copy_from_slice(&revision.to_le_bytes());
        data[120..124].copy_from_slice(&0_u32.to_le_bytes());
        data[124..132].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn cancel_building(
        data: &mut [u8],
        owner: &Pubkey,
        global_config: &Pubkey,
        revision: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        if &view.owner != owner
            || &view.global_config != global_config
            || revision == 0
            || view.pending_revision != revision
        {
            return Err(NicechunkBuildingError::BuildingRevisionConflict.into());
        }
        data[120..124].copy_from_slice(&0_u32.to_le_bytes());
        data[124..132].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

fn foundation_chunk_count(args: &CreateBuildSiteArgs) -> Result<u64, NicechunkBuildingError> {
    Ok(u64::from(args.required_land_contracts()?))
}

fn foundation_chunk_at(
    args: &CreateBuildSiteArgs,
    index: u64,
) -> Result<(i32, i32), NicechunkBuildingError> {
    let (min_chunk_x, min_chunk_z, span_x, span_z) = foundation_chunk_span(args)?;
    let count = span_x
        .checked_mul(span_z)
        .ok_or(NicechunkBuildingError::InvalidBuildSiteData)?;
    if index >= count {
        return Err(NicechunkBuildingError::InvalidBuildSiteData);
    }
    let x = i32::try_from(index % span_x)
        .ok()
        .and_then(|offset| min_chunk_x.checked_add(offset))
        .ok_or(NicechunkBuildingError::InvalidBuildSiteData)?;
    let z = i32::try_from(index / span_x)
        .ok()
        .and_then(|offset| min_chunk_z.checked_add(offset))
        .ok_or(NicechunkBuildingError::InvalidBuildSiteData)?;
    Ok((x, z))
}

fn foundation_chunk_span(
    args: &CreateBuildSiteArgs,
) -> Result<(i32, i32, u64, u64), NicechunkBuildingError> {
    if args.min_x.rem_euclid(FOUNDATION_CHUNK_SIZE as i32) != 0
        || args.min_z.rem_euclid(FOUNDATION_CHUNK_SIZE as i32) != 0
        || args.width < FOUNDATION_CHUNK_SIZE
        || args.depth < FOUNDATION_CHUNK_SIZE
        || args.width % FOUNDATION_CHUNK_SIZE != 0
        || args.depth % FOUNDATION_CHUNK_SIZE != 0
    {
        return Err(NicechunkBuildingError::InvalidBuildSiteData);
    }
    let min_chunk_x = args.min_x.div_euclid(FOUNDATION_CHUNK_SIZE as i32);
    let min_chunk_z = args.min_z.div_euclid(FOUNDATION_CHUNK_SIZE as i32);
    let span_x = u64::from(args.width / FOUNDATION_CHUNK_SIZE);
    let span_z = u64::from(args.depth / FOUNDATION_CHUNK_SIZE);
    if span_x == 0 || span_z == 0 {
        return Err(NicechunkBuildingError::InvalidBuildSiteData);
    }
    Ok((min_chunk_x, min_chunk_z, span_x, span_z))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeginBuildingArgs {
    pub foundation_id: u64,
    pub revision: u32,
    pub quarter_turns: u8,
    pub payload_len: u32,
    pub expected_hash: [u8; 32],
    pub offset_x: i32,
    pub offset_z: i32,
}

impl BeginBuildingArgs {
    pub const LEN: usize = 57;

    pub fn unpack(data: &[u8]) -> Result<Self, NicechunkBuildingError> {
        if data.len() != Self::LEN {
            return Err(NicechunkBuildingError::InvalidBuildingData);
        }
        let payload_len = read_u32(data, 13);
        if read_u32(data, 8) == 0
            || data[12] > 3
            || payload_len == 0
            || payload_len as usize > BUILDING_MAX_PAYLOAD_LEN
        {
            return Err(NicechunkBuildingError::InvalidBuildingData);
        }
        let mut expected_hash = [0_u8; 32];
        expected_hash.copy_from_slice(&data[17..49]);
        Ok(Self {
            foundation_id: read_u64(data, 0),
            revision: read_u32(data, 8),
            quarter_turns: data[12],
            payload_len,
            expected_hash,
            offset_x: read_i32(data, 49),
            offset_z: read_i32(data, 53),
        })
    }
}

pub fn building_axis_fits(foundation_size: u32, building_size: u32, offset: i32) -> bool {
    if building_size == 0 || building_size > foundation_size {
        return false;
    }
    let centered_origin = i64::from((foundation_size - building_size) / 2);
    let shifted_origin = centered_origin + i64::from(offset);
    shifted_origin >= 0 && shifted_origin + i64::from(building_size) <= i64::from(foundation_size)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildingManifestView {
    pub status: u8,
    pub quarter_turns: u8,
    pub shard_count: u8,
    pub uploaded_bitmap: u16,
    pub owner: Pubkey,
    pub global_config: Pubkey,
    pub foundation_id: u64,
    pub revision: u32,
    pub payload_len: u32,
    pub expected_hash: [u8; 32],
    pub size_x: u16,
    pub size_y: u16,
    pub size_z: u16,
    pub created_slot: u64,
    pub updated_slot: u64,
    pub offset_x: i32,
    pub offset_z: i32,
}

pub struct BuildingManifestState;

impl BuildingManifestState {
    pub fn pack_upload(
        dst: &mut [u8],
        bump: u8,
        owner: &Pubkey,
        global_config: &Pubkey,
        args: &BeginBuildingArgs,
        created_slot: u64,
    ) -> ProgramResult {
        if dst.len() != BUILDING_MANIFEST_LEN {
            return Err(NicechunkBuildingError::InvalidBuildingManifestData.into());
        }
        let shard_count = building_shard_count(args.payload_len)?;
        dst.fill(0);
        dst[0..8].copy_from_slice(&BUILDING_MANIFEST_MAGIC);
        dst[8] = BUILDING_MANIFEST_VERSION;
        dst[9] = bump;
        dst[10] = BUILDING_STATUS_UPLOADING;
        dst[11] = args.quarter_turns;
        dst[12] = shard_count;
        dst[14..16].copy_from_slice(&0_u16.to_le_bytes());
        dst[16..48].copy_from_slice(owner.as_ref());
        dst[48..80].copy_from_slice(global_config.as_ref());
        dst[80..88].copy_from_slice(&args.foundation_id.to_le_bytes());
        dst[88..92].copy_from_slice(&args.revision.to_le_bytes());
        dst[92..96].copy_from_slice(&args.payload_len.to_le_bytes());
        dst[96..128].copy_from_slice(&args.expected_hash);
        dst[136..144].copy_from_slice(&created_slot.to_le_bytes());
        dst[144..152].copy_from_slice(&created_slot.to_le_bytes());
        dst[152..156].copy_from_slice(&args.offset_x.to_le_bytes());
        dst[156..160].copy_from_slice(&args.offset_z.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8]) -> Result<BuildingManifestView, NicechunkBuildingError> {
        if data.len() != BUILDING_MANIFEST_LEN
            || data[0..8] != BUILDING_MANIFEST_MAGIC
            || data[8] != BUILDING_MANIFEST_VERSION
            || data[10] > BUILDING_STATUS_ACTIVE
            || data[11] > 3
        {
            return Err(NicechunkBuildingError::InvalidBuildingManifestData);
        }
        let payload_len = read_u32(data, 92);
        let shard_count = building_shard_count(payload_len)?;
        if data[12] != shard_count {
            return Err(NicechunkBuildingError::InvalidBuildingManifestData);
        }
        let uploaded_bitmap = read_u16(data, 14);
        let valid_bitmap = (1_u16 << shard_count) - 1;
        if uploaded_bitmap & !valid_bitmap != 0 {
            return Err(NicechunkBuildingError::InvalidBuildingManifestData);
        }
        let mut expected_hash = [0_u8; 32];
        expected_hash.copy_from_slice(&data[96..128]);
        let view = BuildingManifestView {
            status: data[10],
            quarter_turns: data[11],
            shard_count,
            uploaded_bitmap,
            owner: read_pubkey(
                data,
                16,
                NicechunkBuildingError::InvalidBuildingManifestData,
            )?,
            global_config: read_pubkey(
                data,
                48,
                NicechunkBuildingError::InvalidBuildingManifestData,
            )?,
            foundation_id: read_u64(data, 80),
            revision: read_u32(data, 88),
            payload_len,
            expected_hash,
            size_x: read_u16(data, 128),
            size_y: read_u16(data, 130),
            size_z: read_u16(data, 132),
            created_slot: read_u64(data, 136),
            updated_slot: read_u64(data, 144),
            offset_x: read_i32(data, 152),
            offset_z: read_i32(data, 156),
        };
        let has_dimensions = view.size_x > 0 && view.size_y > 0 && view.size_z > 0;
        if view.foundation_id == 0
            || view.revision == 0
            || view.size_x > NCM3_MAX_DIMENSION as u16
            || view.size_y > NCM3_MAX_DIMENSION as u16
            || view.size_z > NCM3_MAX_DIMENSION as u16
            || view.status == BUILDING_STATUS_UPLOADING && has_dimensions
            || view.status == BUILDING_STATUS_ACTIVE && !has_dimensions
        {
            return Err(NicechunkBuildingError::InvalidBuildingManifestData);
        }
        Ok(view)
    }

    pub fn mark_shard_complete(
        data: &mut [u8],
        foundation_id: u64,
        revision: u32,
        shard_index: u8,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        if view.status != BUILDING_STATUS_UPLOADING
            || view.foundation_id != foundation_id
            || view.revision != revision
            || shard_index >= view.shard_count
        {
            return Err(NicechunkBuildingError::InvalidBuildingManifestData.into());
        }
        let bitmap = view.uploaded_bitmap | (1_u16 << shard_index);
        data[14..16].copy_from_slice(&bitmap.to_le_bytes());
        data[144..152].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn activate(
        data: &mut [u8],
        foundation_id: u64,
        revision: u32,
        dimensions: Ncm3Dimensions,
        updated_slot: u64,
    ) -> ProgramResult {
        let view = Self::validate(data)?;
        let complete_bitmap = (1_u16 << view.shard_count) - 1;
        if view.status != BUILDING_STATUS_UPLOADING
            || view.foundation_id != foundation_id
            || view.revision != revision
            || view.uploaded_bitmap != complete_bitmap
        {
            return Err(NicechunkBuildingError::BuildingUploadIncomplete.into());
        }
        data[10] = BUILDING_STATUS_ACTIVE;
        data[128..130].copy_from_slice(&(dimensions.x as u16).to_le_bytes());
        data[130..132].copy_from_slice(&(dimensions.y as u16).to_le_bytes());
        data[132..134].copy_from_slice(&(dimensions.z as u16).to_le_bytes());
        data[144..152].copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

pub struct WriteBuildingShardArgs<'a> {
    pub foundation_id: u64,
    pub revision: u32,
    pub shard_index: u8,
    pub offset: u16,
    pub bytes: &'a [u8],
}

impl<'a> WriteBuildingShardArgs<'a> {
    pub fn unpack(data: &'a [u8]) -> Result<Self, NicechunkBuildingError> {
        if data.len() <= 15 || data.len() - 15 > BUILDING_MAX_WRITE_LEN {
            return Err(NicechunkBuildingError::InvalidBuildingShardData);
        }
        Ok(Self {
            foundation_id: read_u64(data, 0),
            revision: read_u32(data, 8),
            shard_index: data[12],
            offset: read_u16(data, 13),
            bytes: &data[15..],
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildingShardView {
    pub shard_index: u8,
    pub payload_len: u16,
    pub uploaded_len: u16,
    pub global_config: Pubkey,
    pub foundation_id: u64,
    pub revision: u32,
}

pub struct BuildingShardState;

impl BuildingShardState {
    pub fn len_for_payload(payload_len: usize) -> Result<usize, NicechunkBuildingError> {
        if payload_len == 0 || payload_len > BUILDING_SHARD_PAYLOAD_LEN {
            return Err(NicechunkBuildingError::InvalidBuildingShardData);
        }
        BUILDING_SHARD_HEADER_LEN
            .checked_add(payload_len)
            .ok_or(NicechunkBuildingError::InvalidBuildingShardData)
    }

    pub fn pack_empty(
        dst: &mut [u8],
        bump: u8,
        global_config: &Pubkey,
        foundation_id: u64,
        revision: u32,
        shard_index: u8,
        payload_len: usize,
    ) -> ProgramResult {
        if dst.len() != Self::len_for_payload(payload_len)? {
            return Err(NicechunkBuildingError::InvalidBuildingShardData.into());
        }
        dst.fill(0);
        dst[0..8].copy_from_slice(&BUILDING_SHARD_MAGIC);
        dst[8] = BUILDING_SHARD_VERSION;
        dst[9] = bump;
        dst[10] = shard_index;
        dst[12..14].copy_from_slice(&(payload_len as u16).to_le_bytes());
        dst[14..16].copy_from_slice(&0_u16.to_le_bytes());
        dst[16..48].copy_from_slice(global_config.as_ref());
        dst[48..56].copy_from_slice(&foundation_id.to_le_bytes());
        dst[56..60].copy_from_slice(&revision.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8]) -> Result<BuildingShardView, NicechunkBuildingError> {
        if data.len() < BUILDING_SHARD_HEADER_LEN
            || data[0..8] != BUILDING_SHARD_MAGIC
            || data[8] != BUILDING_SHARD_VERSION
        {
            return Err(NicechunkBuildingError::InvalidBuildingShardData);
        }
        let payload_len = read_u16(data, 12);
        let uploaded_len = read_u16(data, 14);
        if payload_len == 0
            || payload_len as usize > BUILDING_SHARD_PAYLOAD_LEN
            || uploaded_len > payload_len
            || data.len() != Self::len_for_payload(payload_len as usize)?
        {
            return Err(NicechunkBuildingError::InvalidBuildingShardData);
        }
        Ok(BuildingShardView {
            shard_index: data[10],
            payload_len,
            uploaded_len,
            global_config: read_pubkey(data, 16, NicechunkBuildingError::InvalidBuildingShardData)?,
            foundation_id: read_u64(data, 48),
            revision: read_u32(data, 56),
        })
    }

    pub fn append(
        data: &mut [u8],
        foundation_id: u64,
        revision: u32,
        shard_index: u8,
        offset: u16,
        bytes: &[u8],
    ) -> Result<bool, solana_program::program_error::ProgramError> {
        let view = Self::validate(data)?;
        if view.foundation_id != foundation_id
            || view.revision != revision
            || view.shard_index != shard_index
            || view.uploaded_len != offset
        {
            return Err(NicechunkBuildingError::InvalidBuildingShardData.into());
        }
        let end = usize::from(offset)
            .checked_add(bytes.len())
            .ok_or(NicechunkBuildingError::InvalidBuildingShardData)?;
        if end > usize::from(view.payload_len) {
            return Err(NicechunkBuildingError::InvalidBuildingShardData.into());
        }
        let start = BUILDING_SHARD_HEADER_LEN + usize::from(offset);
        data[start..start + bytes.len()].copy_from_slice(bytes);
        data[14..16].copy_from_slice(&(end as u16).to_le_bytes());
        Ok(end == usize::from(view.payload_len))
    }

    pub fn payload(data: &[u8]) -> Result<&[u8], NicechunkBuildingError> {
        let view = Self::validate(data)?;
        if view.uploaded_len != view.payload_len {
            return Err(NicechunkBuildingError::BuildingUploadIncomplete);
        }
        Ok(&data[BUILDING_SHARD_HEADER_LEN..])
    }
}

pub fn building_shard_count(payload_len: u32) -> Result<u8, NicechunkBuildingError> {
    if payload_len == 0 || payload_len as usize > BUILDING_MAX_PAYLOAD_LEN {
        return Err(NicechunkBuildingError::InvalidBuildingData);
    }
    let count =
        (payload_len as usize + BUILDING_SHARD_PAYLOAD_LEN - 1) / BUILDING_SHARD_PAYLOAD_LEN;
    if count == 0 || count > BUILDING_MAX_SHARDS {
        return Err(NicechunkBuildingError::InvalidBuildingData);
    }
    Ok(count as u8)
}

pub fn building_shard_payload_len(
    payload_len: u32,
    shard_index: u8,
) -> Result<usize, NicechunkBuildingError> {
    let count = building_shard_count(payload_len)?;
    if shard_index >= count {
        return Err(NicechunkBuildingError::InvalidBuildingShardData);
    }
    let start = usize::from(shard_index) * BUILDING_SHARD_PAYLOAD_LEN;
    Ok((payload_len as usize - start).min(BUILDING_SHARD_PAYLOAD_LEN))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ncm3Dimensions {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

pub fn hash_payload_slices(slices: &[&[u8]]) -> [u8; 32] {
    hashv(slices).to_bytes()
}

pub fn validate_ncm3_payload(slices: &[&[u8]]) -> Result<Ncm3Dimensions, NicechunkBuildingError> {
    let mut reader = PayloadReader::new(slices);
    if reader.read_byte()? != 1 {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    let dimensions = Ncm3Dimensions {
        x: reader.read_var()?,
        y: reader.read_var()?,
        z: reader.read_var()?,
    };
    if !(1..=NCM3_MAX_DIMENSION).contains(&dimensions.x)
        || !(1..=NCM3_MAX_DIMENSION).contains(&dimensions.y)
        || !(1..=NCM3_MAX_DIMENSION).contains(&dimensions.z)
    {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    let command_count = reader.read_var()?;
    if command_count > NCM3_MAX_COMMANDS {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    let mut operation_budget = 0_u64;
    for _ in 0..command_count {
        let opcode = reader.read_byte()?;
        match opcode {
            1 => {
                read_material(&mut reader)?;
                let values = read_vars::<6>(&mut reader)?;
                validate_origin(values[0], values[1], values[2], dimensions)?;
                let w = plus_one_bounded(values[3], dimensions.x)?;
                let h = plus_one_bounded(values[4], dimensions.y)?;
                let d = plus_one_bounded(values[5], dimensions.z)?;
                validate_cuboid_bounds(values[0], values[1], values[2], w, h, d, dimensions)?;
                add_budget(
                    &mut operation_budget,
                    u64::from(w) * u64::from(h) * u64::from(d),
                )?;
            }
            2 => {
                read_material(&mut reader)?;
                let values = read_vars::<10>(&mut reader)?;
                validate_origin(values[0], values[1], values[2], dimensions)?;
                let w = plus_one_bounded(values[3], dimensions.x)?;
                let h = plus_one_bounded(values[4], dimensions.y)?;
                let d = plus_one_bounded(values[5], dimensions.z)?;
                let count = plus_one_bounded(values[6], 512)?;
                let mut steps = [0_i32; 3];
                for (index, encoded) in values[7..10].iter().enumerate() {
                    let step = decode_signed_var(*encoded);
                    if step.unsigned_abs() > 256 {
                        return Err(NicechunkBuildingError::InvalidNcm3);
                    }
                    steps[index] = step;
                }
                validate_repeat_axis(values[0], w, steps[0], count, dimensions.x)?;
                validate_repeat_axis(values[1], h, steps[1], count, dimensions.y)?;
                validate_repeat_axis(values[2], d, steps[2], count, dimensions.z)?;
                add_budget(
                    &mut operation_budget,
                    u64::from(w) * u64::from(h) * u64::from(d) * u64::from(count),
                )?;
            }
            3 | 6 | 7 | 8 | 9 | 10 => {
                read_material(&mut reader)?;
                let values = read_vars::<5>(&mut reader)?;
                validate_origin(values[0], values[1], values[2], dimensions)?;
                let width = plus_one_bounded(values[3], dimensions.x)?;
                let depth = plus_one_bounded(values[4], dimensions.z)?;
                let layers = if matches!(opcode, 8 | 9 | 10) {
                    depth.div_ceil(2)
                } else {
                    width.div_ceil(2)
                };
                validate_cuboid_bounds(
                    values[0], values[1], values[2], width, layers, depth, dimensions,
                )?;
                let operations = match opcode {
                    3 => u64::from(layers) * 2 * u64::from(depth),
                    6 => u64::from(layers) * 4,
                    7 => u64::from(layers) * u64::from(width) * u64::from(depth),
                    8 => u64::from(layers) * 2 * u64::from(width),
                    9 => u64::from(layers) * 4,
                    _ => u64::from(layers) * u64::from(width) * u64::from(depth),
                };
                add_budget(&mut operation_budget, operations)?;
            }
            4 => {
                read_material(&mut reader)?;
                read_material(&mut reader)?;
                let values = read_vars::<5>(&mut reader)?;
                validate_origin(values[0], values[1], values[2], dimensions)?;
                let height = values[3];
                let crown = values[4];
                if !(2..=64_u32.min(dimensions.y)).contains(&height) || !(1..=16).contains(&crown) {
                    return Err(NicechunkBuildingError::InvalidNcm3);
                }
                let trunk_height = height.saturating_sub(crown).max(2);
                validate_tree_bounds(values[0], values[1], values[2], height, crown, dimensions)?;
                let operations = u64::from(trunk_height) * 4
                    + u64::from(crown) * 8 * u64::from(crown.saturating_mul(2).saturating_add(2))
                    + 4;
                add_budget(&mut operation_budget, operations)?;
            }
            5 => {
                read_material(&mut reader)?;
                let values = read_vars::<6>(&mut reader)?;
                validate_origin(values[0], values[1], values[2], dimensions)?;
                let length = plus_one_bounded(values[3], 256)?;
                if values[4] > 1 || !(1..=64).contains(&values[5]) {
                    return Err(NicechunkBuildingError::InvalidNcm3);
                }
                let (width, depth) = if values[4] == 0 {
                    (length, 1)
                } else {
                    (1, length)
                };
                validate_cuboid_bounds(
                    values[0], values[1], values[2], width, 5, depth, dimensions,
                )?;
                let posts = length.div_ceil(values[5]).saturating_add(1);
                add_budget(
                    &mut operation_budget,
                    u64::from(length) * 2 + u64::from(posts) * 5,
                )?;
            }
            _ => return Err(NicechunkBuildingError::InvalidNcm3),
        }
    }
    if !reader.done() {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    Ok(dimensions)
}

struct PayloadReader<'a> {
    slices: &'a [&'a [u8]],
    slice_index: usize,
    byte_index: usize,
}

impl<'a> PayloadReader<'a> {
    fn new(slices: &'a [&'a [u8]]) -> Self {
        Self {
            slices,
            slice_index: 0,
            byte_index: 0,
        }
    }

    fn read_byte(&mut self) -> Result<u8, NicechunkBuildingError> {
        while self.slice_index < self.slices.len() {
            let slice = self.slices[self.slice_index];
            if self.byte_index < slice.len() {
                let value = slice[self.byte_index];
                self.byte_index += 1;
                return Ok(value);
            }
            self.slice_index += 1;
            self.byte_index = 0;
        }
        Err(NicechunkBuildingError::InvalidNcm3)
    }

    fn read_var(&mut self) -> Result<u32, NicechunkBuildingError> {
        let mut value = 0_u32;
        let mut shift = 0_u32;
        loop {
            let byte = self.read_byte()?;
            let group = u32::from(byte & 0x7f);
            if shift == 28 && group > 0x0f {
                return Err(NicechunkBuildingError::InvalidNcm3);
            }
            value |= group
                .checked_shl(shift)
                .ok_or(NicechunkBuildingError::InvalidNcm3)?;
            if byte & 0x80 == 0 {
                if shift > 0 && group == 0 {
                    return Err(NicechunkBuildingError::InvalidNcm3);
                }
                return Ok(value);
            }
            shift = shift.saturating_add(7);
            if shift > 28 {
                return Err(NicechunkBuildingError::InvalidNcm3);
            }
        }
    }

    fn done(&self) -> bool {
        let mut slice_index = self.slice_index;
        let mut byte_index = self.byte_index;
        while slice_index < self.slices.len() {
            if byte_index < self.slices[slice_index].len() {
                return false;
            }
            slice_index += 1;
            byte_index = 0;
        }
        true
    }
}

fn read_material(reader: &mut PayloadReader<'_>) -> Result<u32, NicechunkBuildingError> {
    let material = reader.read_var()?;
    if (1..=52).contains(&material)
        || (55..=77).contains(&material)
        || (96..=101).contains(&material)
    {
        Ok(material)
    } else {
        Err(NicechunkBuildingError::InvalidNcm3)
    }
}

fn read_vars<const N: usize>(
    reader: &mut PayloadReader<'_>,
) -> Result<[u32; N], NicechunkBuildingError> {
    let mut result = [0_u32; N];
    for value in &mut result {
        *value = reader.read_var()?;
    }
    Ok(result)
}

fn validate_origin(
    x: u32,
    y: u32,
    z: u32,
    dimensions: Ncm3Dimensions,
) -> Result<(), NicechunkBuildingError> {
    if x >= dimensions.x || y >= dimensions.y || z >= dimensions.z {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    Ok(())
}

fn plus_one_bounded(value: u32, maximum: u32) -> Result<u32, NicechunkBuildingError> {
    let value = value
        .checked_add(1)
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    if value == 0 || value > maximum {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    Ok(value)
}

fn validate_cuboid_bounds(
    x: u32,
    y: u32,
    z: u32,
    width: u32,
    height: u32,
    depth: u32,
    dimensions: Ncm3Dimensions,
) -> Result<(), NicechunkBuildingError> {
    if !axis_extent_fits(x, width, dimensions.x)
        || !axis_extent_fits(y, height, dimensions.y)
        || !axis_extent_fits(z, depth, dimensions.z)
    {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    Ok(())
}

fn axis_extent_fits(start: u32, length: u32, limit: u32) -> bool {
    length > 0 && start.checked_add(length).is_some_and(|end| end <= limit)
}

fn validate_repeat_axis(
    start: u32,
    length: u32,
    step: i32,
    count: u32,
    limit: u32,
) -> Result<(), NicechunkBuildingError> {
    let first = i64::from(start);
    let last = i64::from(step)
        .checked_mul(i64::from(count.saturating_sub(1)))
        .and_then(|offset| first.checked_add(offset))
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    let maximum_exclusive = first
        .max(last)
        .checked_add(i64::from(length))
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    if first.min(last) < 0 || maximum_exclusive > i64::from(limit) {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    Ok(())
}

fn validate_tree_bounds(
    x: u32,
    y: u32,
    z: u32,
    height: u32,
    crown: u32,
    dimensions: Ncm3Dimensions,
) -> Result<(), NicechunkBuildingError> {
    let diameter = crown
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    let min_x = x
        .checked_sub(crown)
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    let min_z = z
        .checked_sub(crown)
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    let vertical_extent = height.max(
        crown
            .checked_add(1)
            .ok_or(NicechunkBuildingError::InvalidNcm3)?,
    );
    validate_cuboid_bounds(
        min_x,
        y,
        min_z,
        diameter,
        vertical_extent,
        diameter,
        dimensions,
    )
}

fn decode_signed_var(value: u32) -> i32 {
    if value & 1 == 0 {
        (value / 2) as i32
    } else {
        -((value / 2) as i32) - 1
    }
}

fn add_budget(total: &mut u64, amount: u64) -> Result<(), NicechunkBuildingError> {
    *total = total
        .checked_add(amount)
        .ok_or(NicechunkBuildingError::InvalidNcm3)?;
    if *total > NCM3_OPERATION_BUDGET {
        return Err(NicechunkBuildingError::InvalidNcm3);
    }
    Ok(())
}

fn checked_axis_end(start: i32, length: u32) -> Option<i32> {
    if length == 0 {
        return None;
    }
    let end = i64::from(start).checked_add(i64::from(length).checked_sub(1)?)?;
    i32::try_from(end).ok()
}

fn read_pubkey(
    data: &[u8],
    offset: usize,
    error: NicechunkBuildingError,
) -> Result<Pubkey, NicechunkBuildingError> {
    Pubkey::try_from(&data[offset..offset + 32]).map_err(|_| error)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_site_records_contracts_and_keeps_reserved_bytes_zero() {
        let owner = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let args = CreateBuildSiteArgs {
            foundation_id: 9,
            min_x: -32,
            surface_y: 101,
            min_z: 16,
            width: 48,
            depth: 32,
        };
        let mut data = vec![0_u8; BUILD_SITE_LEN];
        BuildSiteState::pack(&mut data, 200, &owner, &config, &args, 10).unwrap();
        assert_eq!(data[11], LAND_CONTRACT_TYPE_BLANK);
        assert_eq!(read_u32(&data, 12), 6);
        assert_eq!(&data[98..100], &[0; 2]);
        assert_eq!(&data[148..160], &[0; 12]);
        assert_eq!(BuildSiteState::validate(&data).unwrap().width, args.width);
    }

    #[test]
    fn build_site_creation_replay_matches_only_the_original_binding() {
        let owner = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let args = CreateBuildSiteArgs {
            foundation_id: 42,
            min_x: 736,
            surface_y: 136,
            min_z: 768,
            width: 32,
            depth: 32,
        };
        let mut data = vec![0_u8; BUILD_SITE_LEN];
        BuildSiteState::pack(&mut data, 9, &owner, &config, &args, 10).unwrap();

        assert!(BuildSiteState::matches_creation(&data, &owner, &config, &args).unwrap());
        assert!(
            !BuildSiteState::matches_creation(&data, &Pubkey::new_unique(), &config, &args,)
                .unwrap()
        );
        assert!(!BuildSiteState::matches_creation(
            &data,
            &owner,
            &config,
            &CreateBuildSiteArgs {
                min_x: args.min_x + 1,
                ..args
            },
        )
        .unwrap());
    }

    #[test]
    fn incomplete_land_can_only_roll_back_in_reverse_progress() {
        let owner = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let args = CreateBuildSiteArgs {
            foundation_id: 77,
            min_x: 0,
            surface_y: 100,
            min_z: 0,
            width: 32,
            depth: 32,
        };
        let mut data = vec![0_u8; BUILD_SITE_LEN];
        BuildSiteState::pack(&mut data, 1, &owner, &config, &args, 10).unwrap();
        BuildSiteState::advance_indexing(&mut data, &owner, &config, 2, 11).unwrap();
        BuildSiteState::begin_canceling(&mut data, &owner, &config, 12).unwrap();

        let canceling = BuildSiteState::validate(&data).unwrap();
        assert_eq!(canceling.status, BUILD_SITE_STATUS_CANCELING);
        assert_eq!(canceling.registered_chunks, 2);
        assert!(!BuildSiteState::matches_creation(&data, &owner, &config, &args).unwrap());
        assert!(BuildSiteState::index_step(&canceling, 2).is_err());
        assert_eq!(BuildSiteState::chunk_at(&canceling, 1).unwrap(), (1, 0));

        BuildSiteState::rewind_indexing(&mut data, &owner, &config, 1, 13).unwrap();
        BuildSiteState::rewind_indexing(&mut data, &owner, &config, 1, 14).unwrap();
        let empty = BuildSiteState::validate(&data).unwrap();
        assert_eq!(empty.status, BUILD_SITE_STATUS_CANCELING);
        assert_eq!(empty.registered_chunks, 0);
        assert!(BuildSiteState::rewind_indexing(&mut data, &owner, &config, 1, 15).is_err());
    }

    #[test]
    fn ncm3_validator_accepts_payload_across_shards() {
        // version, 4x5x6 size, one 1x1x1 BOX using material 3 at origin
        let payload = [1_u8, 4, 5, 6, 1, 1, 3, 0, 0, 0, 0, 0, 0];
        let dimensions = validate_ncm3_payload(&[&payload[..5], &payload[5..]]).unwrap();
        assert_eq!(dimensions, Ncm3Dimensions { x: 4, y: 5, z: 6 });
        assert_eq!(
            hash_payload_slices(&[&payload[..5], &payload[5..]]),
            solana_program::hash::hash(&payload).to_bytes()
        );
    }

    #[test]
    fn ncm3_validator_rejects_noncanonical_and_overflowing_varints() {
        let noncanonical_dimension = [1_u8, 0x81, 0, 1, 1, 0];
        assert_eq!(
            validate_ncm3_payload(&[&noncanonical_dimension]),
            Err(NicechunkBuildingError::InvalidNcm3)
        );

        let overflowing_dimension = [1_u8, 0xff, 0xff, 0xff, 0xff, 0x1f, 1, 1, 0];
        assert_eq!(
            validate_ncm3_payload(&[&overflowing_dimension]),
            Err(NicechunkBuildingError::InvalidNcm3)
        );
    }

    #[test]
    fn ncm3_material_allowlist_matches_canonical_building_materials() {
        for material in [1_u8, 52, 55, 77, 96, 101] {
            let payload = [1_u8, 1, 1, 1, 1, 1, material, 0, 0, 0, 0, 0, 0];
            assert!(
                validate_ncm3_payload(&[&payload]).is_ok(),
                "canonical material {material} should be accepted"
            );
        }

        for material in [0_u8, 53, 54, 78, 95, 102] {
            let payload = [1_u8, 1, 1, 1, 1, 1, material, 0, 0, 0, 0, 0, 0];
            assert!(
                validate_ncm3_payload(&[&payload]).is_err(),
                "internal or undefined material {material} should be rejected"
            );
        }
    }

    #[test]
    fn ncm3_validator_accepts_exact_spatial_edges_and_negative_repeats() {
        let cases = [
            ncm3_payload([8, 8, 8], 1, 3, &[6, 5, 4, 1, 2, 3]),
            ncm3_payload(
                [8, 4, 4],
                2,
                3,
                &[6, 0, 0, 1, 0, 0, 3, signed_var(-2), 0, 0],
            ),
            ncm3_payload([8, 4, 8], 3, 3, &[0, 0, 0, 7, 7]),
            ncm3_tree_payload([8, 6, 8], 3, 0, 3, 6, 3),
            ncm3_payload([8, 5, 8], 5, 22, &[0, 0, 0, 7, 0, 2]),
            ncm3_payload([8, 5, 8], 5, 22, &[0, 0, 0, 7, 1, 2]),
        ];
        for payload in cases {
            assert!(
                validate_ncm3_payload(&[&payload]).is_ok(),
                "exact-edge payload should be valid: {payload:?}"
            );
        }
    }

    #[test]
    fn ncm3_validator_rejects_expanded_geometry_outside_declared_dimensions() {
        let cases = [
            ncm3_payload([8, 8, 8], 1, 3, &[7, 0, 0, 1, 0, 0]),
            ncm3_payload([8, 8, 8], 1, 3, &[0, 7, 0, 0, 1, 0]),
            ncm3_payload([8, 8, 8], 1, 3, &[0, 0, 7, 0, 0, 1]),
            ncm3_payload(
                [8, 4, 4],
                2,
                3,
                &[5, 0, 0, 1, 0, 0, 3, signed_var(-2), 0, 0],
            ),
            ncm3_payload([8, 4, 4], 2, 3, &[1, 0, 0, 1, 0, 0, 3, signed_var(2), 0, 0]),
            ncm3_payload([8, 3, 8], 3, 3, &[0, 0, 0, 7, 7]),
            ncm3_payload([8, 3, 8], 8, 3, &[0, 0, 0, 7, 7]),
            ncm3_tree_payload([8, 6, 8], 2, 0, 3, 6, 3),
            ncm3_tree_payload([8, 4, 8], 3, 0, 3, 2, 4),
            ncm3_payload([8, 4, 8], 5, 22, &[0, 0, 0, 7, 0, 2]),
            ncm3_payload([8, 5, 8], 5, 22, &[1, 0, 0, 7, 0, 2]),
            ncm3_payload([8, 5, 8], 5, 22, &[0, 0, 1, 7, 1, 2]),
        ];
        for payload in cases {
            assert_eq!(
                validate_ncm3_payload(&[&payload]),
                Err(NicechunkBuildingError::InvalidNcm3),
                "out-of-bounds payload should be rejected: {payload:?}"
            );
        }
    }

    fn ncm3_payload(dimensions: [u32; 3], opcode: u8, material: u32, values: &[u32]) -> Vec<u8> {
        let mut payload = vec![1];
        for value in dimensions {
            push_var(&mut payload, value);
        }
        push_var(&mut payload, 1);
        payload.push(opcode);
        push_var(&mut payload, material);
        for value in values {
            push_var(&mut payload, *value);
        }
        payload
    }

    fn ncm3_tree_payload(
        dimensions: [u32; 3],
        x: u32,
        y: u32,
        z: u32,
        height: u32,
        crown: u32,
    ) -> Vec<u8> {
        let mut payload = vec![1];
        for value in dimensions {
            push_var(&mut payload, value);
        }
        push_var(&mut payload, 1);
        payload.push(4);
        push_var(&mut payload, 22);
        push_var(&mut payload, 23);
        for value in [x, y, z, height, crown] {
            push_var(&mut payload, value);
        }
        payload
    }

    fn signed_var(value: i32) -> u32 {
        if value < 0 {
            value.unsigned_abs().saturating_mul(2).saturating_sub(1)
        } else {
            (value as u32).saturating_mul(2)
        }
    }

    fn push_var(output: &mut Vec<u8>, mut value: u32) {
        while value > 0x7f {
            output.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }

    #[test]
    fn begin_building_requires_explicit_offsets() {
        let mut shifted = vec![0_u8; BeginBuildingArgs::LEN];
        shifted[0..8].copy_from_slice(&7_u64.to_le_bytes());
        shifted[8..12].copy_from_slice(&1_u32.to_le_bytes());
        shifted[12] = 2;
        shifted[13..17].copy_from_slice(&13_u32.to_le_bytes());
        shifted[17..49].copy_from_slice(&[5_u8; 32]);
        shifted[49..53].copy_from_slice(&(-3_i32).to_le_bytes());
        shifted[53..57].copy_from_slice(&4_i32.to_le_bytes());

        let shifted_args = BeginBuildingArgs::unpack(&shifted).unwrap();
        assert_eq!((shifted_args.offset_x, shifted_args.offset_z), (-3, 4));
        assert!(BeginBuildingArgs::unpack(&shifted[..49]).is_err());
    }

    #[test]
    fn shifted_building_axis_must_remain_inside_foundation() {
        assert!(building_axis_fits(10, 4, 0));
        assert!(building_axis_fits(10, 4, -3));
        assert!(building_axis_fits(10, 4, 3));
        assert!(!building_axis_fits(10, 4, -4));
        assert!(!building_axis_fits(10, 4, 4));
        assert!(building_axis_fits(10, 3, 4));
        assert!(!building_axis_fits(10, 3, 5));
        assert!(!building_axis_fits(3, 4, 0));
    }

    #[test]
    fn manifest_and_shards_track_sequential_upload() {
        let owner = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let args = BeginBuildingArgs {
            foundation_id: 7,
            revision: 1,
            quarter_turns: 0,
            payload_len: 13,
            expected_hash: [5; 32],
            offset_x: -3,
            offset_z: 4,
        };
        let mut manifest = vec![0_u8; BUILDING_MANIFEST_LEN];
        BuildingManifestState::pack_upload(&mut manifest, 1, &owner, &config, &args, 10).unwrap();
        let mut shard = vec![0_u8; BuildingShardState::len_for_payload(13).unwrap()];
        BuildingShardState::pack_empty(&mut shard, 2, &config, 7, 1, 0, 13).unwrap();
        assert!(!BuildingShardState::append(&mut shard, 7, 1, 0, 0, &[1, 2]).unwrap());
        assert!(BuildingShardState::append(&mut shard, 7, 1, 0, 2, &[0; 11]).unwrap());
        BuildingManifestState::mark_shard_complete(&mut manifest, 7, 1, 0, 11).unwrap();
        assert_eq!(
            BuildingManifestState::validate(&manifest)
                .unwrap()
                .uploaded_bitmap,
            1
        );
        let manifest_view = BuildingManifestState::validate(&manifest).unwrap();
        assert_eq!((manifest_view.offset_x, manifest_view.offset_z), (-3, 4));
    }

    #[test]
    fn build_site_can_cancel_only_its_pending_revision() {
        let owner = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let args = CreateBuildSiteArgs {
            foundation_id: 42,
            min_x: 16,
            surface_y: 20,
            min_z: 32,
            width: 16,
            depth: 16,
        };
        let mut data = vec![0_u8; BUILD_SITE_LEN];
        BuildSiteState::pack(&mut data, 1, &owner, &config, &args, 10).unwrap();
        let total_chunks = BuildSiteState::validate(&data).unwrap().total_chunks;
        BuildSiteState::advance_indexing(&mut data, &owner, &config, total_chunks, 11).unwrap();
        BuildSiteState::begin_building(&mut data, &owner, &config, 1, 11).unwrap();

        assert!(BuildSiteState::cancel_building(&mut data, &owner, &config, 2, 12).is_err());
        assert!(
            BuildSiteState::cancel_building(&mut data, &Pubkey::new_unique(), &config, 1, 12,)
                .is_err()
        );

        BuildSiteState::cancel_building(&mut data, &owner, &config, 1, 13).unwrap();
        let view = BuildSiteState::validate(&data).unwrap();
        assert_eq!(view.active_revision, 0);
        assert_eq!(view.pending_revision, 0);
        assert_eq!(view.updated_slot, 13);
    }

    #[test]
    fn build_site_indexes_chunks_in_stable_row_major_order() {
        let owner = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let args = CreateBuildSiteArgs {
            foundation_id: 42,
            min_x: 736,
            surface_y: 136,
            min_z: 768,
            width: 32,
            depth: 32,
        };
        let mut data = vec![0_u8; BUILD_SITE_LEN];
        BuildSiteState::pack(&mut data, 1, &owner, &config, &args, 10).unwrap();
        let site = BuildSiteState::validate(&data).unwrap();
        assert_eq!(site.total_chunks, 4);
        assert_eq!(BuildSiteState::chunk_at(&site, 0).unwrap(), (46, 48));
        assert_eq!(BuildSiteState::chunk_at(&site, 1).unwrap(), (47, 48));
        assert_eq!(BuildSiteState::chunk_at(&site, 2).unwrap(), (46, 49));
        assert_eq!(BuildSiteState::chunk_at(&site, 3).unwrap(), (47, 49));
    }

    #[test]
    fn build_site_rejects_partial_chunks() {
        let args = CreateBuildSiteArgs {
            foundation_id: 91,
            min_x: 0,
            surface_y: 100,
            min_z: 0,
            width: 17,
            depth: 16,
        };
        assert_eq!(
            args.required_land_contracts(),
            Err(NicechunkBuildingError::InvalidBuildSiteData)
        );
    }

    #[test]
    fn build_site_caps_one_parcel_at_four_thousand_ninety_six_contracts() {
        let maximum = CreateBuildSiteArgs {
            foundation_id: 92,
            min_x: 0,
            surface_y: 100,
            min_z: 0,
            width: FOUNDATION_CHUNK_SIZE * MAX_LAND_CONTRACTS_PER_SITE,
            depth: FOUNDATION_CHUNK_SIZE,
        };
        assert_eq!(
            maximum.required_land_contracts(),
            Ok(MAX_LAND_CONTRACTS_PER_SITE)
        );

        let oversized = CreateBuildSiteArgs {
            foundation_id: 93,
            width: maximum.width + FOUNDATION_CHUNK_SIZE,
            ..maximum
        };
        assert_eq!(
            oversized.required_land_contracts(),
            Err(NicechunkBuildingError::InvalidBuildSiteData)
        );
    }
}
