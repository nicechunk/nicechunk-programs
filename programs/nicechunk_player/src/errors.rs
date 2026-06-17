use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[repr(u32)]
pub enum NicechunkPlayerError {
    InvalidInstruction = 6200,
    InvalidAccountCount = 6201,
    InvalidPayer = 6202,
    InvalidWritableAccount = 6203,
    InvalidSystemProgram = 6204,
    InvalidSystemAccount = 6205,
    InvalidPlayerProfilePda = 6206,
    PlayerProfileAlreadyInitialized = 6207,
    InvalidPlayerProfileOwner = 6208,
    InvalidPlayerProfileData = 6209,
    InvalidGlobalConfig = 6210,
    InvalidGlobalConfigOwner = 6211,
    InvalidGlobalConfigData = 6212,
    InvalidWorldBounds = 6213,
    InvalidEquipmentSlot = 6214,
    PackSizeMismatch = 6215,
    InvalidPlayerAuthority = 6216,
    InvalidPlayerSessionPda = 6217,
    InvalidPlayerSessionData = 6218,
    InvalidSessionAuthority = 6219,
    InvalidBackpackProgram = 6220,
    InvalidBackpackData = 6221,
    InvalidBackpackOwner = 6222,
    PlayerBackpackAlreadyBound = 6223,
}

impl From<NicechunkPlayerError> for ProgramError {
    fn from(error: NicechunkPlayerError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkPlayerError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
