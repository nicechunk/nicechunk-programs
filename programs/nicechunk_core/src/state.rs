use solana_program::{entrypoint::ProgramResult, pubkey, pubkey::Pubkey};

use crate::{
    cluster_config::{DEVELOPMENT_WALLET, NCK_MINT},
    errors::NicechunkError,
};

pub const CONFIG_MAGIC: [u8; 8] = *b"NCKCFG01";
pub const CONFIG_VERSION: u16 = 1;
pub const NCK_DECIMALS: u8 = 6;
pub const NCK_GENESIS_SUPPLY: u64 = 1_000_000_000_000_000;

pub const STARTER_PACK_PRICE_LAMPORTS: u64 = 100_000_000;
pub const GENESIS_PASS_PRICE_LAMPORTS: u64 = 1_000_000_000;
pub const GUARDIAN_STAKE_AMOUNT: u64 = 100_000_000_000;

pub const GLOBAL_CONFIG_SEED: &[u8] = b"global-config";

pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const WORLD_ID: u16 = 1;
pub const WORLD_SEED: [u8; 32] = [
    186, 26, 157, 68, 97, 87, 197, 55, 175, 88, 252, 95, 245, 58, 40, 66, 44, 220, 106, 179, 221,
    136, 218, 162, 64, 149, 219, 59, 217, 192, 240, 65,
];
pub const TERRAIN_CONFIG_HASH: [u8; 32] = [
    102, 240, 59, 92, 247, 50, 206, 249, 91, 222, 69, 201, 190, 239, 114, 112, 248, 133, 161, 145,
    71, 20, 137, 22, 235, 49, 191, 32, 169, 2, 33, 8,
];
pub const RESOURCE_RULE_HASH: [u8; 32] = [
    114, 32, 67, 125, 241, 70, 69, 189, 12, 27, 75, 84, 43, 146, 26, 69, 123, 216, 81, 6, 205, 35,
    117, 239, 235, 11, 204, 133, 127, 202, 138, 58,
];
pub const CLIENT_WORLD_CONFIG_HASH: [u8; 32] = [
    21, 60, 4, 234, 74, 63, 221, 241, 141, 253, 96, 238, 7, 2, 131, 11, 204, 59, 93, 146, 197, 179,
    203, 18, 147, 45, 225, 110, 211, 254, 159, 135,
];

pub const STARTER_PACK_MAX_PER_WALLET: u8 = 1;
pub const GENESIS_PASS_MAX_PER_WALLET: u8 = 1;
pub const GENESIS_PASS_MAX_SUPPLY: u32 = 10_000;

pub const GUARDIAN_TAX_BPS: u16 = 10;
pub const PROTOCOL_FEE_BPS: u16 = 50;
pub const MARKET_FEE_BPS: u16 = 100;
pub const SLASH_BPS: u16 = 3000;

pub const SOL_TO_LIQUIDITY_BPS: u16 = 5000;
pub const SOL_TO_REWARD_BPS: u16 = 3000;
pub const SOL_TO_DEVELOPMENT_BPS: u16 = 2000;

pub const CHUNK_SIZE: u16 = 16;
pub const SECTION_HEIGHT: u16 = 16;
pub const MIN_BUILD_Y: i16 = -32;
pub const MAX_BUILD_Y: i16 = 256;
pub const MAX_TERRAIN_HEIGHT: i16 = 160;
pub const SEA_LEVEL: i16 = 2;

pub const GUARDIAN_REGION_SIZE_CHUNKS: u16 = 64;
pub const GUARDIAN_REALTIME_RADIUS_CHUNKS: u16 = 16;
pub const MINE_COOLDOWN_SLOTS: u16 = 2;

/// Minimal immutable genesis account.
///
/// There is no admin key, no mutable config authority, no pause flag, and no
/// update instruction. The account stores only fixed public rules and hashes.
/// The native program writes these values from constants compiled into the
/// program, so the initializer cannot choose or alter genesis parameters.
pub struct GlobalConfig;

impl GlobalConfig {
    pub const LEN: usize = 293;

    pub fn pack(
        dst: &mut [u8],
        global_config_bump: u8,
        genesis_slot: u64,
        created_at: i64,
    ) -> ProgramResult {
        if dst.len() != Self::LEN {
            return Err(NicechunkError::InvalidGlobalConfigData.into());
        }

        let mut writer = ByteWriter { dst, offset: 0 };

        writer.bytes(&CONFIG_MAGIC)?;
        writer.u16(CONFIG_VERSION)?;
        writer.u8(global_config_bump)?;
        writer.u8(1)?;
        writer.pubkey(&NCK_MINT)?;
        writer.u8(NCK_DECIMALS)?;
        writer.u64(NCK_GENESIS_SUPPLY)?;
        writer.pubkey(&DEVELOPMENT_WALLET)?;
        writer.u16(WORLD_ID)?;
        writer.bytes(&WORLD_SEED)?;
        writer.bytes(&TERRAIN_CONFIG_HASH)?;
        writer.bytes(&RESOURCE_RULE_HASH)?;
        writer.bytes(&CLIENT_WORLD_CONFIG_HASH)?;
        writer.u64(STARTER_PACK_PRICE_LAMPORTS)?;
        writer.u64(GENESIS_PASS_PRICE_LAMPORTS)?;
        writer.u8(STARTER_PACK_MAX_PER_WALLET)?;
        writer.u8(GENESIS_PASS_MAX_PER_WALLET)?;
        writer.u32(GENESIS_PASS_MAX_SUPPLY)?;
        writer.u64(GUARDIAN_STAKE_AMOUNT)?;
        writer.u16(GUARDIAN_TAX_BPS)?;
        writer.u16(PROTOCOL_FEE_BPS)?;
        writer.u16(MARKET_FEE_BPS)?;
        writer.u16(SLASH_BPS)?;
        writer.u16(SOL_TO_LIQUIDITY_BPS)?;
        writer.u16(SOL_TO_REWARD_BPS)?;
        writer.u16(SOL_TO_DEVELOPMENT_BPS)?;
        writer.u16(CHUNK_SIZE)?;
        writer.u16(SECTION_HEIGHT)?;
        writer.i16(MIN_BUILD_Y)?;
        writer.i16(MAX_BUILD_Y)?;
        writer.i16(MAX_TERRAIN_HEIGHT)?;
        writer.i16(SEA_LEVEL)?;
        writer.u16(GUARDIAN_REGION_SIZE_CHUNKS)?;
        writer.u16(GUARDIAN_REALTIME_RADIUS_CHUNKS)?;
        writer.u16(MINE_COOLDOWN_SLOTS)?;
        writer.u64(genesis_slot)?;
        writer.i64(created_at)?;

        if writer.offset != Self::LEN {
            return Err(NicechunkError::PackSizeMismatch.into());
        }

        Ok(())
    }
}

struct ByteWriter<'a> {
    dst: &'a mut [u8],
    offset: usize,
}

impl ByteWriter<'_> {
    fn bytes(&mut self, bytes: &[u8]) -> ProgramResult {
        let end = self.offset + bytes.len();
        if end > self.dst.len() {
            return Err(NicechunkError::PackSizeMismatch.into());
        }
        self.dst[self.offset..end].copy_from_slice(bytes);
        self.offset = end;
        Ok(())
    }

    fn pubkey(&mut self, key: &Pubkey) -> ProgramResult {
        self.bytes(key.as_ref())
    }

    fn u8(&mut self, value: u8) -> ProgramResult {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn i16(&mut self, value: i16) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> ProgramResult {
        self.bytes(&value.to_le_bytes())
    }
}
