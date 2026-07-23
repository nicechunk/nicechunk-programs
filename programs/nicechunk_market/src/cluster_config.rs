use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(all(feature = "devnet", feature = "mainnet"))]
compile_error!("Only one cluster feature can be enabled");
#[cfg(all(feature = "devnet", feature = "testnet"))]
compile_error!("Only one cluster feature can be enabled");
#[cfg(all(feature = "testnet", feature = "mainnet"))]
compile_error!("Only one cluster feature can be enabled");

#[cfg(feature = "devnet")]
pub const NCK_MINT: Pubkey = pubkey!("HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo");
#[cfg(feature = "testnet")]
pub const NCK_MINT: Pubkey = pubkey!("2ukPLJUs7N5ktZdFzPQTcJ2wVNN1Nb5WzRrARCVpE5kz");
#[cfg(feature = "mainnet")]
pub const NCK_MINT: Pubkey = pubkey!("DCoNyDmQC4kKmQeB7GnwjZuMEvAjjqFYzmnTjySPifEK");

pub const MARKET_TREASURY: Pubkey = pubkey!("CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF");
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB");

#[cfg(feature = "unified-game")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");
