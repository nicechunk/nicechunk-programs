use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(all(feature = "devnet", feature = "mainnet"))]
compile_error!("Only one cluster feature can be enabled");
#[cfg(all(feature = "devnet", feature = "testnet"))]
compile_error!("Only one cluster feature can be enabled");
#[cfg(all(feature = "testnet", feature = "mainnet"))]
compile_error!("Only one cluster feature can be enabled");

#[cfg(feature = "devnet")]
pub const NICECHUNK_CORE_PROGRAM_ID: Pubkey =
    pubkey!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
#[cfg(feature = "devnet")]
pub const NCK_MINT: Pubkey = pubkey!("HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo");

#[cfg(feature = "testnet")]
pub const NICECHUNK_CORE_PROGRAM_ID: Pubkey =
    pubkey!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
#[cfg(feature = "testnet")]
pub const NCK_MINT: Pubkey = pubkey!("2ukPLJUs7N5ktZdFzPQTcJ2wVNN1Nb5WzRrARCVpE5kz");

#[cfg(feature = "mainnet")]
pub const NICECHUNK_CORE_PROGRAM_ID: Pubkey =
    pubkey!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
#[cfg(feature = "mainnet")]
pub const NCK_MINT: Pubkey = pubkey!("DCoNyDmQC4kKmQeB7GnwjZuMEvAjjqFYzmnTjySPifEK");

pub const DEVELOPMENT_WALLET: Pubkey = pubkey!("CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF");
