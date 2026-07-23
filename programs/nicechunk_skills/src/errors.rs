use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NicechunkSkillsError {
    InvalidInstruction = 7600,
    InvalidAccountCount = 7601,
    InvalidPayer = 7602,
    InvalidWritableAccount = 7603,
    InvalidSystemProgram = 7604,
    InvalidSystemAccount = 7605,
    InvalidGlobalConfig = 7606,
    InvalidGlobalConfigOwner = 7607,
    UnauthorizedAuthority = 7608,
    InvalidRuleTablePda = 7609,
    RuleTableAlreadyInitialized = 7610,
    InvalidRuleTableOwner = 7611,
    InvalidRuleTableData = 7612,
    InvalidRuleIndex = 7613,
    InvalidRule = 7614,
    RuleIdentityImmutable = 7615,
    InvalidThresholds = 7616,
    InvalidSkillIndex = 7617,
    InvalidPlayerSkillsPda = 7618,
    InvalidPlayerSkillsOwner = 7619,
    InvalidPlayerSkillsData = 7620,
    InvalidSourceAccount = 7621,
    InvalidSourceOwner = 7622,
    InvalidSourcePda = 7623,
    InvalidSourceData = 7624,
    SourceCounterRegressed = 7625,
    ArithmeticOverflow = 7626,
    PackSizeMismatch = 7627,
    InvalidInstructionsSysvar = 7628,
    InvalidMiningProof = 7629,
    InvalidMiningTravelRule = 7630,
    InvalidBurdenMiningRule = 7631,
    InvalidBackpackSource = 7632,
    BackpackMassMigrationRequired = 7633,
}

impl From<NicechunkSkillsError> for ProgramError {
    fn from(error: NicechunkSkillsError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn require_key_eq(
    left: &Pubkey,
    right: &Pubkey,
    error: NicechunkSkillsError,
) -> Result<(), ProgramError> {
    if left != right {
        return Err(error.into());
    }
    Ok(())
}
