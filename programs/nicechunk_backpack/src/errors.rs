use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    PlayerBackpackAlreadyBound = 6419,
    InvalidResourceIndex = 6420,
    InvalidMarketAuthority = 6421,
    InvalidSmeltingAuthority = 6422,
    InvalidInventoryItem = 6423,
    InvalidForgingMaterial = 6424,
    InvalidChunkAuthority = 6425,
    InvalidGlobalConfig = 6426,
    InvalidForgeMaterialRequirements = 6427,
    InsufficientForgeMaterialParameters = 6428,
    UnverifiedForgeInstructionDisabled = 6429,
    InvalidBlueprintIssuer = 6430,
    InvalidBlueprintPda = 6431,
    BlueprintAlreadyIssued = 6432,
    InvalidBlueprintItem = 6433,
    InvalidPlayerEquipment = 6434,
    InvalidEquipmentTransferAuthority = 6435,
    InvalidEquipmentSlot = 6436,
    EquipmentSlotEmpty = 6437,
    InvalidMaterialPhysicsPda = 6438,
    InvalidMaterialPhysicsData = 6439,
    InvalidMaterialPhysicsRule = 6440,
    InvalidMaterialPhysicsAuthority = 6441,
    InvalidBackpackMassState = 6442,
    BackpackMassOverflow = 6443,
    InvalidMiningAction = 6444,
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
