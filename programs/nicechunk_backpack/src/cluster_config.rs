use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(feature = "devnet")]
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");
#[cfg(feature = "devnet")]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("1PwPzFtdJ5gQqku5gBo4b6Wvo48Qe8NuXSogUP8TWpR");
#[cfg(feature = "devnet")]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");

#[cfg(feature = "testnet")]
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");
#[cfg(feature = "testnet")]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("1PwPzFtdJ5gQqku5gBo4b6Wvo48Qe8NuXSogUP8TWpR");
#[cfg(feature = "testnet")]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");

#[cfg(feature = "mainnet")]
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");
#[cfg(feature = "mainnet")]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("1PwPzFtdJ5gQqku5gBo4b6Wvo48Qe8NuXSogUP8TWpR");
#[cfg(feature = "mainnet")]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");
