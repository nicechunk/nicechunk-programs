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
    InvalidForgingAuthority = 6224,
    InvalidBackpackItem = 6225,
    InvalidPlayerName = 6226,
    InvalidAppearancePda = 6227,
    InvalidAppearanceOwner = 6228,
    InvalidAppearanceData = 6229,
    InvalidCharacterModelKind = 6230,
    InvalidCharacterCode = 6231,
    InvalidTreasuryAuthority = 6232,
    InvalidAppearanceTitle = 6233,
    InvalidInviteIndexPda = 6234,
    InvalidInviteIndexData = 6235,
    InviteIndexPageFull = 6236,
    InviteFirstPageRequiresInviter = 6237,
    InvitePreviousPageRequired = 6238,
    InvitePreviousPageNotFull = 6239,
    InvalidInviteSelf = 6240,
    InvalidUsernameIndexPda = 6241,
    InvalidUsernameIndexData = 6242,
    UsernameAlreadyTaken = 6243,
    InvalidPlayerEquipmentPda = 6244,
    InvalidPlayerEquipmentOwner = 6245,
    InvalidPlayerEquipmentData = 6246,
    InvalidEquipmentModel = 6247,
    InvalidGameProgram = 6248,
    InvalidEquipmentTransferAuthority = 6249,
    EquipmentNotCustodied = 6250,
    InvalidDurabilityAmount = 6251,
    EquipmentBroken = 6252,
    PlayerSessionExpired = 6253,
    PlayerSessionActionNotAllowed = 6254,
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
