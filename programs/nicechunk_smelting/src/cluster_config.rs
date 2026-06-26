use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(feature = "devnet")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");

#[cfg(feature = "testnet")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");

#[cfg(feature = "mainnet")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");
