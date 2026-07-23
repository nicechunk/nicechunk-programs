use solana_program::{pubkey, pubkey::Pubkey};

pub const NICECHUNK_CORE_PROGRAM_ID: Pubkey =
    pubkey!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB");
pub const NICECHUNK_BLUEPRINT_ISSUER: Pubkey =
    pubkey!("9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");
pub const NICECHUNK_BOOTSTRAP_AUTHORITY: Pubkey =
    pubkey!("9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");

#[cfg(feature = "unified-game")]
pub const NICECHUNK_CHUNK_PROGRAM_ID: Pubkey =
    pubkey!("GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj");
#[cfg(feature = "unified-game")]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");
#[cfg(feature = "unified-game")]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_CHUNK_PROGRAM_ID: Pubkey =
    pubkey!("GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj");
#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_MARKET_PROGRAM_ID: Pubkey =
    pubkey!("1PwPzFtdJ5gQqku5gBo4b6Wvo48Qe8NuXSogUP8TWpR");
#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_SMELTING_PROGRAM_ID: Pubkey =
    pubkey!("7imEiNtpiN487HRwrftdLrMFs8TcAnjLE94vKsDgU6L7");
