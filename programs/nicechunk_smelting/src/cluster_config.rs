use solana_program::{pubkey, pubkey::Pubkey};

pub const NICECHUNK_CIVILIZATION_PROGRAM_ID: Pubkey =
    pubkey!("3MRG4UjxTK1rMq7TGM4bX1GrD8C36bQtt1RdTmJD7Jah");

#[cfg(feature = "unified-game")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");
