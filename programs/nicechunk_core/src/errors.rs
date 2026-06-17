use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[repr(u32)]
pub enum NicechunkError {
    InvalidInstruction = 6000,
    InvalidAccountCount = 6001,
    InvalidPayer = 6002,
    InvalidGlobalConfigPda = 6003,
    GlobalConfigAlreadyInitialized = 6004,
    InvalidNckMint = 6005,
    InvalidNckDecimals = 6006,
    InvalidNckGenesisSupply = 6007,
    InvalidNckAuthority = 6008,
    InvalidSystemProgram = 6009,
    InvalidGlobalConfigOwner = 6010,
    InvalidGlobalConfigData = 6011,
    InvalidWritableAccount = 6012,
    InvalidSystemAccount = 6013,
    InvalidGlobalConfigFunding = 6014,
    PackSizeMismatch = 6015,
}

impl From<NicechunkError> for ProgramError {
    fn from(error: NicechunkError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
