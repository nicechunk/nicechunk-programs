use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug)]
#[repr(u32)]
pub enum NicechunkChunkError {
    InvalidInstruction = 6300,
    InvalidAccountCount = 6301,
    InvalidPayer = 6302,
    InvalidWritableAccount = 6303,
    InvalidSystemProgram = 6304,
    InvalidSystemAccount = 6305,
    InvalidChunkPda = 6306,
    ChunkAlreadyInitialized = 6307,
    InvalidChunkOwner = 6308,
    InvalidChunkData = 6309,
    InvalidGlobalConfig = 6310,
    InvalidGlobalConfigOwner = 6311,
    InvalidGlobalConfigData = 6312,
    InvalidPlayerProgram = 6313,
    InvalidPlayerProfile = 6314,
    InvalidPlayerAuthority = 6315,
    InvalidBlockCoordinate = 6316,
    InvalidBlockChange = 6317,
    PackSizeMismatch = 6318,
    InvalidDelegationProgram = 6319,
    InvalidOwnerProgram = 6320,
    InvalidDelegationAccount = 6321,
    InvalidPlayerSession = 6322,
    InvalidSessionAuthority = 6323,
    PlayerSessionExpired = 6324,
    SessionActionNotAllowed = 6325,
    GeneratedBlockMismatch = 6326,
    InvalidChunkBrokenPda = 6327,
    InvalidChunkBrokenData = 6328,
    BlockAlreadyMined = 6329,
    UnmineableBlock = 6330,
    ChunkBrokenCapacityExceeded = 6331,
    InvalidPackedCoordinate = 6332,
}

impl From<NicechunkChunkError> for ProgramError {
    fn from(error: NicechunkChunkError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkChunkError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
