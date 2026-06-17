use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug)]
#[repr(u32)]
pub enum NicechunkGuardianError {
    InvalidInstruction = 6400,
    InvalidAccountCount = 6401,
    InvalidPayer = 6402,
    InvalidWritableAccount = 6403,
    InvalidSystemProgram = 6404,
    InvalidSystemAccount = 6405,
    InvalidRegistryPda = 6406,
    RegistryAlreadyInitialized = 6407,
    InvalidRegistryOwner = 6408,
    InvalidRegistryData = 6409,
    InvalidGlobalConfigOwner = 6410,
    InvalidNckMint = 6411,
    InvalidTokenProgram = 6412,
    InvalidTokenAccount = 6413,
    InvalidTreasuryAuthority = 6414,
    InvalidGuardianRegionPda = 6415,
    GuardianRegionAlreadyActive = 6416,
    InvalidGuardianRegionData = 6417,
    InvalidGuardianOwner = 6418,
    InvalidOperatorAuthority = 6419,
    InvalidHost = 6420,
    InvalidPort = 6421,
    MissingAdjacentGuardian = 6422,
    InvalidAdjacentGuardian = 6423,
    NoGenesisPermission = 6424,
    GenesisAlreadyRegistered = 6425,
    GuardianNotActive = 6426,
    GuardianStillFresh = 6427,
    PackSizeMismatch = 6428,
}

impl From<NicechunkGuardianError> for ProgramError {
    fn from(error: NicechunkGuardianError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkGuardianError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
