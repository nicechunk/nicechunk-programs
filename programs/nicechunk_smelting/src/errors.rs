use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug)]
#[repr(u32)]
pub enum NicechunkSmeltingError {
    InvalidInstruction = 6800,
    InvalidAccountCount = 6801,
    InvalidPayer = 6802,
    InvalidWritableAccount = 6803,
    InvalidSystemProgram = 6804,
    InvalidSystemAccount = 6805,
    InvalidRecipeTablePda = 6806,
    RecipeTableAlreadyInitialized = 6807,
    InvalidRecipeTableOwner = 6808,
    InvalidRecipeTableData = 6809,
    InvalidRecipe = 6810,
    RecipeNotFound = 6811,
    RecipeTableFull = 6812,
    UnauthorizedAuthority = 6813,
    PackSizeMismatch = 6814,
    InvalidBackpackProgram = 6815,
    InvalidBackpackOwner = 6816,
    InvalidBackpackData = 6817,
    InvalidInputIndex = 6818,
    InputRecipeMismatch = 6819,
    BackpackCapacityExceeded = 6820,
    InvalidSmeltingAuthority = 6821,
    FuelHeatTooLow = 6822,
    InvalidPlayerProgress = 6823,
    InvalidPlayerProgressData = 6824,
    InvalidCivilizationProgram = 6825,
    InvalidCivilizationAccount = 6826,
    InvalidCivilizationRule = 6827,
    InvalidCivilizationTally = 6828,
    InvalidCivilizationReceipt = 6829,
    CivilizationTargetMismatch = 6830,
    CivilizationPatchHashMismatch = 6831,
    CivilizationThresholdNotMet = 6832,
}

impl From<NicechunkSmeltingError> for ProgramError {
    fn from(error: NicechunkSmeltingError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkSmeltingError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
