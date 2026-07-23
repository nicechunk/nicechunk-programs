use solana_program::{pubkey, pubkey::Pubkey};

#[cfg(all(feature = "devnet", feature = "mainnet"))]
compile_error!("Only one cluster feature can be enabled");
#[cfg(all(feature = "devnet", feature = "testnet"))]
compile_error!("Only one cluster feature can be enabled");
#[cfg(all(feature = "testnet", feature = "mainnet"))]
compile_error!("Only one cluster feature can be enabled");

pub const NICECHUNK_CORE_PROGRAM_ID: Pubkey =
    pubkey!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
pub const NICECHUNK_PLAYER_PROGRAM_ID: Pubkey =
    pubkey!("CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB");
pub const NICECHUNK_CIVILIZATION_PROGRAM_ID: Pubkey =
    pubkey!("3MRG4UjxTK1rMq7TGM4bX1GrD8C36bQtt1RdTmJD7Jah");
pub const NICECHUNK_BUILDING_PROGRAM_ID: Pubkey =
    pubkey!("39UMTUWXQkuomkFNbDPF5NGZnJmG6pDkJHVSkZyqVwWx");

#[cfg(feature = "unified-game")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");
