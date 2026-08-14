use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum NicechunkBuildingError {
    InvalidInstruction = 6500,
    InvalidAccountCount = 6501,
    InvalidWritableAccount = 6502,
    InvalidSystemProgram = 6503,
    InvalidSystemAccount = 6504,
    InvalidGlobalConfigOwner = 6505,
    InvalidGlobalConfigData = 6506,
    InvalidPlayerProgram = 6507,
    InvalidPlayerProfile = 6508,
    InvalidPlayerAuthority = 6509,
    InvalidPlayerSession = 6510,
    InvalidSessionAuthority = 6511,
    PlayerSessionExpired = 6512,
    SessionActionNotAllowed = 6513,
    InvalidBuildSitePda = 6514,
    InvalidBuildSiteData = 6515,
    BuildSiteIndexingIncomplete = 6516,
    InvalidChunkProgram = 6517,
    InvalidChunkAuthority = 6518,
    InvalidBuildingPda = 6521,
    InvalidBuildingData = 6522,
    BuildingAlreadyExists = 6523,
    BuildingUploadIncomplete = 6524,
    BuildingHashMismatch = 6525,
    BuildingDoesNotFit = 6526,
    InvalidBuildingManifestPda = 6527,
    InvalidBuildingManifestData = 6528,
    InvalidBuildingShardPda = 6529,
    InvalidBuildingShardData = 6530,
    BuildingRevisionConflict = 6531,
    InvalidNcm3 = 6532,
    InvalidGuardianProgram = 6533,
    InvalidGuardianBlueprintAuthority = 6534,
    InvalidGuardianBlueprintPublisher = 6535,
    InvalidMarketProgram = 6536,
    InvalidLandContractAuthority = 6537,
    InvalidMarketUser = 6538,
    InvalidLandContractCount = 6539,
    BuildSiteNotCancelable = 6540,
}

impl From<NicechunkBuildingError> for ProgramError {
    fn from(error: NicechunkBuildingError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkBuildingError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
