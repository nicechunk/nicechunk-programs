use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkMarketError;

pub const MARKET_USER_MAGIC: [u8; 8] = *b"NCKMUS01";
pub const MARKET_USER_VERSION: u16 = 1;
pub const MARKET_USER_SEED: &[u8] = b"market-user-v1";
pub const MARKET_USER_LEN: usize = 64;
pub const MAX_ACTIVE_LISTINGS: u8 = 50;

pub struct MarketUserState;

impl MarketUserState {
    pub const LEN: usize = MARKET_USER_LEN;
    pub const ACTIVE_COUNT_OFFSET: usize = 11;
    pub const OWNER_OFFSET: usize = 12;
    pub const UPDATED_SLOT_OFFSET: usize = 44;

    pub fn pack(data: &mut [u8], bump: u8, owner: &Pubkey, updated_slot: u64) -> ProgramResult {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidMarketUser.into());
        }
        data.fill(0);
        data[0..8].copy_from_slice(&MARKET_USER_MAGIC);
        data[8..10].copy_from_slice(&MARKET_USER_VERSION.to_le_bytes());
        data[10] = bump;
        data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32].copy_from_slice(owner.as_ref());
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }

    pub fn validate(data: &[u8], owner: &Pubkey) -> ProgramResult {
        if data.len() != Self::LEN
            || data[0..8] != MARKET_USER_MAGIC
            || read_u16(data, 8) != MARKET_USER_VERSION
            || &data[Self::OWNER_OFFSET..Self::OWNER_OFFSET + 32] != owner.as_ref()
            || data[Self::ACTIVE_COUNT_OFFSET] > MAX_ACTIVE_LISTINGS
            || data[52..64] != [0_u8; 12]
        {
            return Err(NicechunkMarketError::InvalidMarketUser.into());
        }
        Ok(())
    }

    pub fn active_count(data: &[u8]) -> Result<u8, NicechunkMarketError> {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidMarketUser);
        }
        Ok(data[Self::ACTIVE_COUNT_OFFSET])
    }

    pub fn increment_active(data: &mut [u8], updated_slot: u64) -> ProgramResult {
        let active_count = Self::active_count(data)?;
        if active_count >= MAX_ACTIVE_LISTINGS {
            return Err(NicechunkMarketError::ActiveListingLimitReached.into());
        }
        data[Self::ACTIVE_COUNT_OFFSET] = active_count + 1;
        Self::write_updated_slot(data, updated_slot)
    }

    pub fn decrement_active(data: &mut [u8], updated_slot: u64) -> ProgramResult {
        let active_count = Self::active_count(data)?;
        if active_count == 0 {
            return Err(NicechunkMarketError::InvalidActiveListingCount.into());
        }
        data[Self::ACTIVE_COUNT_OFFSET] = active_count - 1;
        Self::write_updated_slot(data, updated_slot)
    }

    fn write_updated_slot(data: &mut [u8], updated_slot: u64) -> ProgramResult {
        if data.len() != Self::LEN {
            return Err(NicechunkMarketError::InvalidMarketUser.into());
        }
        data[Self::UPDATED_SLOT_OFFSET..Self::UPDATED_SLOT_OFFSET + 8]
            .copy_from_slice(&updated_slot.to_le_bytes());
        Ok(())
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_user_enforces_the_active_listing_limit() {
        let owner = Pubkey::new_unique();
        let mut data = [0_u8; MarketUserState::LEN];
        MarketUserState::pack(&mut data, 3, &owner, 10).unwrap();
        MarketUserState::validate(&data, &owner).unwrap();

        for slot in 0..MAX_ACTIVE_LISTINGS {
            MarketUserState::increment_active(&mut data, slot as u64).unwrap();
        }
        assert_eq!(
            MarketUserState::increment_active(&mut data, 99).unwrap_err(),
            NicechunkMarketError::ActiveListingLimitReached.into()
        );
        MarketUserState::decrement_active(&mut data, 100).unwrap();
        assert_eq!(
            MarketUserState::active_count(&data).unwrap(),
            MAX_ACTIVE_LISTINGS - 1
        );
    }
}
