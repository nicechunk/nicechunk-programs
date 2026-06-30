#![allow(unexpected_cfgs)]

use solana_program::{account_info::AccountInfo, declare_id, entrypoint::ProgramResult};

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

pub mod errors;

use errors::NicechunkGameError;

declare_id!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

const NS_BACKPACK: u8 = 1;
const NS_CHUNK: u8 = 2;
const NS_SMELTING: u8 = 3;
const NS_MARKET: u8 = 4;

pub fn process_instruction(
    program_id: &solana_program::pubkey::Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(NicechunkGameError::InvalidInstruction)?;

    match *tag {
        NS_BACKPACK => nicechunk_backpack::process_instruction(program_id, accounts, payload),
        NS_CHUNK => nicechunk_chunk::process_instruction(program_id, accounts, payload),
        NS_SMELTING => nicechunk_smelting::process_instruction(program_id, accounts, payload),
        NS_MARKET => nicechunk_market::process_instruction(program_id, accounts, payload),
        _ => Err(NicechunkGameError::InvalidInstruction.into()),
    }
}
