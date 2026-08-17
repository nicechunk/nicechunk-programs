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
    InvalidCurrency = 6611,
    InvalidPrice = 6613,
    ListingNotActive = 6615,
    UnauthorizedSeller = 6616,
    PackSizeMismatch = 6617,
    InvalidBuyer = 6618,
    UnsupportedCurrency = 6619,
    InvalidNckMint = 6620,
    InvalidTokenProgram = 6621,
    InvalidTokenAccount = 6622,
    InvalidBackpackProgram = 6623,
    InvalidBackpackData = 6624,
    InvalidEscrowInventory = 6625,
    InvalidMarketAuthority = 6626,
    InvalidTreasury = 6627,
    InvalidFee = 6628,
    InvalidPlayerProgram = 6629,
    InvalidPlayerProfile = 6630,
    InvalidEquipmentSource = 6631,
    InvalidMaterialPhysics = 6632,
    NonTransferableItem = 6633,
    InvalidMarketUser = 6634,
    ActiveListingLimitReached = 6635,
    InvalidActiveListingCount = 6636,
    MarketAlreadyJoined = 6637,
    InvalidContractType = 6638,
    InvalidContractQuantity = 6639,
    ContractBalanceOverflow = 6640,
    InsufficientLandContracts = 6641,
    InvalidContractAuthority = 6642,
    InsufficientReservedLandContracts = 6643,
    InvalidGlobalConfig = 6644,
    InvalidSwapState = 6645,
    InvalidSwapAuthority = 6646,
    InvalidSwapSolVault = 6647,
    InvalidSwapNckVault = 6648,
    SwapAlreadyInitialized = 6649,
    InvalidSwapConfig = 6650,
    SwapPaused = 6651,
    SwapConfigRevisionMismatch = 6652,
    SwapDeadlineExpired = 6653,
    SwapAmountOutTooLow = 6654,
    SwapAmountOutsideLimits = 6655,
    InsufficientSwapLiquidity = 6656,
    SwapMathOverflow = 6657,
    UnauthorizedSwapAdmin = 6658,
    InvalidSwapAmount = 6659,
    SwapStateOverflow = 6660,
    SwapMustBePaused = 6661,
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
