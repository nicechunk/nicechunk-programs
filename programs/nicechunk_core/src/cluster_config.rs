use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(all(
    feature = "devnet",
    feature = "mainnet",
    not(feature = "no-entrypoint")
))]
compile_error!("Only one cluster feature can be enabled");

#[cfg(all(
    feature = "devnet",
    feature = "testnet",
    not(feature = "no-entrypoint")
))]
compile_error!("Only one cluster feature can be enabled");

#[cfg(all(
    feature = "testnet",
    feature = "mainnet",
    not(feature = "no-entrypoint")
))]
compile_error!("Only one cluster feature can be enabled");

// Devnet/testnet values must be cluster-local mints with decimals = 6,
// genesis supply = 1,000,000,000 NCK base units, and no mint/freeze
// authorities. Do not use the mainnet NCK mint on devnet/testnet.
#[cfg(all(feature = "devnet", not(feature = "mainnet"), not(feature = "testnet")))]
pub const NCK_MINT: Pubkey = pubkey!("HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo");

#[cfg(all(feature = "testnet", not(feature = "mainnet")))]
pub const NCK_MINT: Pubkey = pubkey!("2ukPLJUs7N5ktZdFzPQTcJ2wVNN1Nb5WzRrARCVpE5kz");

#[cfg(feature = "mainnet")]
pub const NCK_MINT: Pubkey = pubkey!("DCoNyDmQC4kKmQeB7GnwjZuMEvAjjqFYzmnTjySPifEK");

pub const DEVELOPMENT_WALLET: Pubkey = pubkey!("CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF");
