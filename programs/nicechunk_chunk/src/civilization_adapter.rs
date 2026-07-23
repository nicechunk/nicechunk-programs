use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    hash::hash,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    pubkey::Pubkey,
    system_program,
};

use crate::errors::{require_key_eq, NicechunkChunkError};

pub const CIVILIZATION_RULE_BOOK_MAGIC: [u8; 8] = *b"NCKCVR01";
pub const CIVILIZATION_TALLY_MAGIC: [u8; 8] = *b"NCKCVT01";
pub const CIVILIZATION_EXECUTION_RECEIPT_MAGIC: [u8; 8] = *b"NCKCVE01";
pub const CIVILIZATION_VERSION: u16 = 1;
pub const CIVILIZATION_STATUS_FINALIZED: u8 = 2;
pub const CIVILIZATION_STATUS_EXECUTED: u8 = 3;
pub const CIVILIZATION_EXECUTE_RECEIPT_TAG: u8 = 3;
pub const CIVILIZATION_ADAPTER_AUTHORITY_SEED: &[u8] = b"civilization-adapter";

pub const RULE_BOOK_STATUS_OFFSET: usize = 11;
pub const RULE_BOOK_PATCH_HASH_OFFSET: usize = 108;
pub const RULE_BOOK_TARGET_PROGRAM_OFFSET: usize = 172;
pub const RULE_BOOK_TARGET_PDA_OFFSET: usize = 204;
pub const TALLY_THRESHOLD_MET_OFFSET: usize = 11;
pub const TALLY_RULE_BOOK_OFFSET: usize = 12;
pub const RECEIPT_EXECUTED_OFFSET: usize = 11;
pub const RECEIPT_RULE_BOOK_OFFSET: usize = 12;

pub fn validate_rule_book_for_chunk_patch(
    data: &[u8],
    civilization_program: &Pubkey,
    chunk_program: &Pubkey,
    target_pda: &Pubkey,
    patch_payload: &[u8],
    expected_status: u8,
) -> ProgramResult {
    validate_header(data, &CIVILIZATION_RULE_BOOK_MAGIC)?;
    if data.len() < RULE_BOOK_TARGET_PDA_OFFSET + 32 {
        return Err(NicechunkChunkError::InvalidCivilizationRule.into());
    }
    if data.get(RULE_BOOK_STATUS_OFFSET).copied() != Some(expected_status) {
        return Err(NicechunkChunkError::InvalidCivilizationRule.into());
    }
    if pubkey_at(data, RULE_BOOK_TARGET_PROGRAM_OFFSET)? != *chunk_program {
        return Err(NicechunkChunkError::CivilizationTargetMismatch.into());
    }
    if pubkey_at(data, RULE_BOOK_TARGET_PDA_OFFSET)? != *target_pda {
        return Err(NicechunkChunkError::CivilizationTargetMismatch.into());
    }
    let patch_hash = hash(patch_payload);
    if &data[RULE_BOOK_PATCH_HASH_OFFSET..RULE_BOOK_PATCH_HASH_OFFSET + 32] != patch_hash.as_ref() {
        return Err(NicechunkChunkError::CivilizationPatchHashMismatch.into());
    }
    if civilization_program == chunk_program {
        return Err(NicechunkChunkError::InvalidCivilizationProgram.into());
    }
    Ok(())
}

pub fn validate_tally_threshold(data: &[u8], rule_book: &Pubkey) -> ProgramResult {
    validate_header(data, &CIVILIZATION_TALLY_MAGIC)?;
    if pubkey_at(data, TALLY_RULE_BOOK_OFFSET)? != *rule_book {
        return Err(NicechunkChunkError::InvalidCivilizationTally.into());
    }
    if data.get(TALLY_THRESHOLD_MET_OFFSET).copied() != Some(1) {
        return Err(NicechunkChunkError::CivilizationThresholdNotMet.into());
    }
    Ok(())
}

pub fn validate_execution_receipt(data: &[u8], rule_book: &Pubkey) -> ProgramResult {
    validate_header(data, &CIVILIZATION_EXECUTION_RECEIPT_MAGIC)?;
    if data.get(RECEIPT_EXECUTED_OFFSET).copied() != Some(1) {
        return Err(NicechunkChunkError::InvalidCivilizationReceipt.into());
    }
    if pubkey_at(data, RECEIPT_RULE_BOOK_OFFSET)? != *rule_book {
        return Err(NicechunkChunkError::InvalidCivilizationReceipt.into());
    }
    Ok(())
}

pub fn invoke_civilization_execute_receipt<'a>(
    executor: &AccountInfo<'a>,
    rule_book: &AccountInfo<'a>,
    tally: &AccountInfo<'a>,
    receipt: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    civilization_program: &AccountInfo<'a>,
    adapter_authority: &AccountInfo<'a>,
    chunk_program: &Pubkey,
) -> ProgramResult {
    require_key_eq(
        system_program_account.key,
        &system_program::ID,
        NicechunkChunkError::InvalidSystemProgram,
    )?;
    let adapter_bump =
        validate_adapter_authority_pda(adapter_authority.key, chunk_program, rule_book.key)?;
    let ix = Instruction {
        program_id: *civilization_program.key,
        accounts: vec![
            AccountMeta::new(*executor.key, true),
            AccountMeta::new(*rule_book.key, false),
            AccountMeta::new_readonly(*tally.key, false),
            AccountMeta::new(*receipt.key, false),
            AccountMeta::new_readonly(*system_program_account.key, false),
            AccountMeta::new_readonly(*adapter_authority.key, true),
        ],
        data: vec![CIVILIZATION_EXECUTE_RECEIPT_TAG],
    };
    invoke_signed(
        &ix,
        &[
            executor.clone(),
            rule_book.clone(),
            tally.clone(),
            receipt.clone(),
            system_program_account.clone(),
            adapter_authority.clone(),
            civilization_program.clone(),
        ],
        &[&[
            CIVILIZATION_ADAPTER_AUTHORITY_SEED,
            rule_book.key.as_ref(),
            &[adapter_bump],
        ]],
    )
}

pub fn validate_adapter_authority_pda(
    adapter_authority: &Pubkey,
    chunk_program: &Pubkey,
    rule_book: &Pubkey,
) -> Result<u8, NicechunkChunkError> {
    let (expected, bump) = Pubkey::find_program_address(
        &[CIVILIZATION_ADAPTER_AUTHORITY_SEED, rule_book.as_ref()],
        chunk_program,
    );
    if &expected != adapter_authority {
        return Err(NicechunkChunkError::InvalidCivilizationProgram);
    }
    Ok(bump)
}

fn validate_header(data: &[u8], magic: &[u8; 8]) -> ProgramResult {
    if data.len() < 12 {
        return Err(NicechunkChunkError::InvalidCivilizationAccount.into());
    }
    if data[0..8] != *magic || read_u16(data, 8) != CIVILIZATION_VERSION {
        return Err(NicechunkChunkError::InvalidCivilizationAccount.into());
    }
    Ok(())
}

fn pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey, NicechunkChunkError> {
    if offset + 32 > data.len() {
        return Err(NicechunkChunkError::InvalidCivilizationAccount);
    }
    Ok(Pubkey::new_from_array(
        data[offset..offset + 32]
            .try_into()
            .map_err(|_| NicechunkChunkError::InvalidCivilizationAccount)?,
    ))
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_target_and_patch_hash() {
        let civilization_program = Pubkey::new_unique();
        let chunk_program = Pubkey::new_unique();
        let target_pda = Pubkey::new_unique();
        let patch = [42_u8; 16];
        let patch_hash = hash(&patch);
        let mut data = [0_u8; 320];
        data[0..8].copy_from_slice(&CIVILIZATION_RULE_BOOK_MAGIC);
        data[8..10].copy_from_slice(&CIVILIZATION_VERSION.to_le_bytes());
        data[RULE_BOOK_STATUS_OFFSET] = CIVILIZATION_STATUS_FINALIZED;
        data[RULE_BOOK_PATCH_HASH_OFFSET..RULE_BOOK_PATCH_HASH_OFFSET + 32]
            .copy_from_slice(patch_hash.as_ref());
        data[RULE_BOOK_TARGET_PROGRAM_OFFSET..RULE_BOOK_TARGET_PROGRAM_OFFSET + 32]
            .copy_from_slice(chunk_program.as_ref());
        data[RULE_BOOK_TARGET_PDA_OFFSET..RULE_BOOK_TARGET_PDA_OFFSET + 32]
            .copy_from_slice(target_pda.as_ref());

        validate_rule_book_for_chunk_patch(
            &data,
            &civilization_program,
            &chunk_program,
            &target_pda,
            &patch,
            CIVILIZATION_STATUS_FINALIZED,
        )
        .unwrap();

        let err = validate_rule_book_for_chunk_patch(
            &data,
            &civilization_program,
            &chunk_program,
            &target_pda,
            &[7_u8; 16],
            CIVILIZATION_STATUS_FINALIZED,
        )
        .unwrap_err();
        assert_eq!(
            err,
            solana_program::program_error::ProgramError::Custom(
                NicechunkChunkError::CivilizationPatchHashMismatch as u32,
            ),
        );
    }
}
