use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug)]
#[repr(u32)]
pub enum NicechunkMarketError {
    InvalidInstruction = 6600,
    InvalidAccountCount = 6601,
    InvalidSeller = 6602,
    InvalidWritableAccount = 6603,
    InvalidSystemProgram = 6604,
    InvalidSystemAccount = 6605,
    InvalidListingPda = 6606,
    ListingAlreadyInitialized = 6607,
    InvalidListingOwner = 6608,
    InvalidListingData = 6609,
    InvalidCategory = 6610,
    InvalidCurrency = 6611,
    InvalidSourceKind = 6612,
    InvalidPrice = 6613,
    InvalidQuantity = 6614,
    ListingNotActive = 6615,
    UnauthorizedSeller = 6616,
    PackSizeMismatch = 6617,
    InvalidBuyer = 6618,
    UnsupportedCurrency = 6619,
    InvalidNckMint = 6620,
    InvalidTokenProgram = 6621,
    InvalidTokenAccount = 6622,
}

impl From<NicechunkMarketError> for ProgramError {
    fn from(error: NicechunkMarketError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkMarketError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
