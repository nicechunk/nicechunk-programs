use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(feature = "devnet")]
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey = pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");

#[cfg(feature = "testnet")]
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey = pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");

#[cfg(feature = "mainnet")]
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey = pubkey!("oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe");
