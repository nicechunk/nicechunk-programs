use solana_program::program_error::ProgramError;

#[derive(Debug)]
#[repr(u32)]
pub enum NicechunkGameError {
    InvalidInstruction = 6600,
}

impl From<NicechunkGameError> for ProgramError {
    fn from(error: NicechunkGameError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
