use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum NicechunkChunkError {
    InvalidInstruction = 6300,
    InvalidAccountCount = 6301,
    InvalidPayer = 6302,
    InvalidWritableAccount = 6303,
    InvalidSystemProgram = 6304,
    InvalidSystemAccount = 6305,
    InvalidGlobalConfig = 6310,
    InvalidGlobalConfigOwner = 6311,
    InvalidGlobalConfigData = 6312,
    InvalidPlayerProgram = 6313,
    InvalidPlayerProfile = 6314,
    InvalidPlayerAuthority = 6315,
    InvalidBlockCoordinate = 6316,
    InvalidPlayerSession = 6322,
    InvalidSessionAuthority = 6323,
    PlayerSessionExpired = 6324,
    SessionActionNotAllowed = 6325,
    GeneratedBlockMismatch = 6326,
    InvalidChunkBrokenPda = 6327,
    InvalidChunkBrokenData = 6328,
    BlockAlreadyMined = 6329,
    UnmineableBlock = 6330,
    ChunkBrokenCapacityExceeded = 6331,
    InvalidPackedCoordinate = 6332,
    InvalidResourceDropTablePda = 6333,
    InvalidResourceDropTableData = 6334,
    InvalidBackpackProgram = 6335,
    InvalidBackpackOwner = 6336,
    InvalidPlayerProgress = 6337,
    InvalidPlayerProgressData = 6338,
    InvalidCivilizationProgram = 6339,
    InvalidCivilizationAccount = 6340,
    InvalidCivilizationRule = 6341,
    InvalidCivilizationTally = 6342,
    InvalidCivilizationReceipt = 6343,
    CivilizationTargetMismatch = 6344,
    CivilizationPatchHashMismatch = 6345,
    CivilizationThresholdNotMet = 6346,
    InvalidSurfaceDecorationTablePda = 6347,
    InvalidSurfaceDecorationTableData = 6348,
    SurfaceDecorationMismatch = 6349,
    InvalidRuleTableAuthority = 6350,
    InvalidFoundationPda = 6351,
    InvalidFoundationData = 6352,
    InvalidFoundationChunkPda = 6353,
    InvalidFoundationChunkData = 6354,
    FoundationOverlap = 6355,
    FoundationChunkCapacityExceeded = 6356,
    FoundationProtected = 6357,
    InvalidBuildingPda = 6358,
    InvalidBuildingData = 6359,
    BuildingAlreadyExists = 6360,
    BuildingUploadIncomplete = 6361,
    BuildingHashMismatch = 6362,
    BuildingDoesNotFit = 6363,
    InvalidBuildSitePda = 6364,
    InvalidBuildSiteData = 6365,
    InvalidBuildingManifestPda = 6368,
    InvalidBuildingManifestData = 6369,
    InvalidBuildingShardPda = 6370,
    InvalidBuildingShardData = 6371,
    BuildingRevisionConflict = 6372,
    InvalidNcm3 = 6373,
    InvalidBuildingProgram = 6374,
    InvalidBuildingAuthority = 6375,
    InvalidFoundationRegistration = 6376,
    InvalidBatchMine = 6377,
    BatchMineCrossChunk = 6378,
    InvalidRangeMine = 6379,
}

impl From<NicechunkChunkError> for ProgramError {
    fn from(error: NicechunkChunkError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkChunkError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
