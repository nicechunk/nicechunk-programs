use solana_program::{pubkey, pubkey::Pubkey};

pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");

#[cfg(feature = "unified-game")]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");
#[cfg(feature = "unified-game")]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("1PwPzFtdJ5gQqku5gBo4b6Wvo48Qe8NuXSogUP8TWpR");
#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");
