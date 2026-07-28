use solana_program::{pubkey, pubkey::Pubkey};

pub const NICECHUNK_CORE_PROGRAM_ID: Pubkey =
    pubkey!("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
pub const NICECHUNK_CIVILIZATION_PROGRAM_ID: Pubkey =
    pubkey!("3MRG4UjxTK1rMq7TGM4bX1GrD8C36bQtt1RdTmJD7Jah");
pub const NICECHUNK_SMELTING_RECIPE_AUTHORITY: Pubkey =
    pubkey!("9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud");
pub const NICECHUNK_SKILLS_PROGRAM_ID: Pubkey =
    pubkey!("5gkdfmRJogdSdPrT8rvnEkPdn2N2fLBnQ6YDdegUcu3P");

#[cfg(feature = "unified-game")]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7");

#[cfg(not(feature = "unified-game"))]
pub const NICECHUNK_BACKPACK_PROGRAM_ID: Pubkey =
    pubkey!("FwTrMDGyRg653L9svvt5aoGii9ZjX1WekSFWcwByjxqt");
