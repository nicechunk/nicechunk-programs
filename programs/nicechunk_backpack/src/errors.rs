use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug)]
#[repr(u32)]
pub enum NicechunkBackpackError {
    InvalidInstruction = 6400,
    InvalidAccountCount = 6401,
    InvalidPayer = 6402,
    InvalidWritableAccount = 6403,
    InvalidSystemProgram = 6404,
    InvalidSystemAccount = 6405,
    InvalidBackpackPda = 6406,
    BackpackAlreadyInitialized = 6407,
    InvalidBackpackOwner = 6408,
    InvalidBackpackData = 6409,
    InvalidBackpackCapacity = 6410,
    BackpackFull = 6411,
    InvalidPlayerProgram = 6412,
    InvalidPlayerProfile = 6413,
    InvalidPlayerSession = 6414,
    InvalidSessionAuthority = 6415,
    PlayerSessionExpired = 6416,
    SessionActionNotAllowed = 6417,
    PackSizeMismatch = 6418,
}

impl From<NicechunkBackpackError> for ProgramError {
    fn from(error: NicechunkBackpackError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkBackpackError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
