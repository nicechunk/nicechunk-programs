use solana_program::{entrypoint::ProgramResult, pubkey::Pubkey};

use crate::errors::NicechunkMarketError;

pub const MARKET_USER_MAGIC: [u8; 8] = *b"NCKMUS01";
pub const MARKET_USER_VERSION: u16 = 1;
pub const MARKET_USER_SEED: &[u8] = b"market-user-v1";
pub const MARKET_USER_LEN: usize = 64;
pub const MAX_ACTIVE_LISTINGS: u8 = 50;
pub const CONTRACT_TYPE_BLANK_LAND: u8 = 1;
pub const MAX_CONTRACT_PURCHASE_QUANTITY: u32 = 4_096;

pub struct MarketUserState;

impl MarketUserState {
    pub const LEN: usize = MARKET_USER_LEN;
    pub const ACTIVE_COUNT_OFFSET: usize = 11;
    pub const OWNER_OFFSET: usize = 12;
    pub const UPDATED_SLOT_OFFSET: usize = 44;
    pub const BLANK_LAND_CONTRACTS_OFFSET: usize = 52;
    pub const RESERVED_BLANK_LAND_CONTRACTS_OFFSET: usize = 56;

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
            || data[60..64].iter().any(|byte| *byte != 0)
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

    pub fn blank_land_contracts(data: &[u8]) -> Result<u32, NicechunkMarketError> {
        read_u32_checked(data, Self::BLANK_LAND_CONTRACTS_OFFSET)
    }

    pub fn reserved_blank_land_contracts(data: &[u8]) -> Result<u32, NicechunkMarketError> {
        read_u32_checked(data, Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET)
    }

    pub fn credit_blank_land_contracts(
        data: &mut [u8],
        quantity: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        if quantity == 0 || quantity > MAX_CONTRACT_PURCHASE_QUANTITY {
            return Err(NicechunkMarketError::InvalidContractQuantity.into());
        }
        let current_balance = Self::blank_land_contracts(data)?;
        let reserved = Self::reserved_blank_land_contracts(data)?;
        current_balance
            .checked_add(reserved)
            .and_then(|total| total.checked_add(quantity))
            .ok_or(NicechunkMarketError::ContractBalanceOverflow)?;
        let balance = current_balance
            .checked_add(quantity)
            .ok_or(NicechunkMarketError::ContractBalanceOverflow)?;
        data[Self::BLANK_LAND_CONTRACTS_OFFSET..Self::BLANK_LAND_CONTRACTS_OFFSET + 4]
            .copy_from_slice(&balance.to_le_bytes());
        Self::write_updated_slot(data, updated_slot)
    }

    pub fn reserve_blank_land_contracts(
        data: &mut [u8],
        quantity: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        if quantity == 0 {
            return Err(NicechunkMarketError::InvalidContractQuantity.into());
        }
        let remaining = Self::blank_land_contracts(data)?
            .checked_sub(quantity)
            .ok_or(NicechunkMarketError::InsufficientLandContracts)?;
        let reserved = Self::reserved_blank_land_contracts(data)?
            .checked_add(quantity)
            .ok_or(NicechunkMarketError::ContractBalanceOverflow)?;
        data[Self::BLANK_LAND_CONTRACTS_OFFSET..Self::BLANK_LAND_CONTRACTS_OFFSET + 4]
            .copy_from_slice(&remaining.to_le_bytes());
        data[Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET
            ..Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET + 4]
            .copy_from_slice(&reserved.to_le_bytes());
        Self::write_updated_slot(data, updated_slot)
    }

    pub fn consume_reserved_blank_land_contracts(
        data: &mut [u8],
        quantity: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        if quantity == 0 {
            return Err(NicechunkMarketError::InvalidContractQuantity.into());
        }
        let reserved = Self::reserved_blank_land_contracts(data)?
            .checked_sub(quantity)
            .ok_or(NicechunkMarketError::InsufficientReservedLandContracts)?;
        data[Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET
            ..Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET + 4]
            .copy_from_slice(&reserved.to_le_bytes());
        Self::write_updated_slot(data, updated_slot)
    }

    pub fn release_reserved_blank_land_contracts(
        data: &mut [u8],
        quantity: u32,
        updated_slot: u64,
    ) -> ProgramResult {
        if quantity == 0 {
            return Err(NicechunkMarketError::InvalidContractQuantity.into());
        }
        let reserved = Self::reserved_blank_land_contracts(data)?
            .checked_sub(quantity)
            .ok_or(NicechunkMarketError::InsufficientReservedLandContracts)?;
        let balance = Self::blank_land_contracts(data)?
            .checked_add(quantity)
            .ok_or(NicechunkMarketError::ContractBalanceOverflow)?;
        data[Self::BLANK_LAND_CONTRACTS_OFFSET..Self::BLANK_LAND_CONTRACTS_OFFSET + 4]
            .copy_from_slice(&balance.to_le_bytes());
        data[Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET
            ..Self::RESERVED_BLANK_LAND_CONTRACTS_OFFSET + 4]
            .copy_from_slice(&reserved.to_le_bytes());
        Self::write_updated_slot(data, updated_slot)
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

fn read_u32_checked(data: &[u8], offset: usize) -> Result<u32, NicechunkMarketError> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(NicechunkMarketError::InvalidMarketUser)?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| NicechunkMarketError::InvalidMarketUser)?,
    ))
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

    #[test]
    fn land_contract_reservations_are_consumed_or_released_exactly_once() {
        let owner = Pubkey::new_unique();
        let mut data = [0_u8; MarketUserState::LEN];
        MarketUserState::pack(&mut data, 7, &owner, 10).unwrap();

        MarketUserState::credit_blank_land_contracts(&mut data, 6, 11).unwrap();
        MarketUserState::reserve_blank_land_contracts(&mut data, 4, 12).unwrap();

        MarketUserState::validate(&data, &owner).unwrap();
        assert_eq!(MarketUserState::blank_land_contracts(&data).unwrap(), 2);
        assert_eq!(
            MarketUserState::reserved_blank_land_contracts(&data).unwrap(),
            4
        );
        assert_eq!(
            MarketUserState::reserve_blank_land_contracts(&mut data, 3, 13).unwrap_err(),
            NicechunkMarketError::InsufficientLandContracts.into()
        );

        MarketUserState::release_reserved_blank_land_contracts(&mut data, 3, 14).unwrap();
        assert_eq!(MarketUserState::blank_land_contracts(&data).unwrap(), 5);
        assert_eq!(
            MarketUserState::reserved_blank_land_contracts(&data).unwrap(),
            1
        );
        MarketUserState::consume_reserved_blank_land_contracts(&mut data, 1, 15).unwrap();
        assert_eq!(
            MarketUserState::reserved_blank_land_contracts(&data).unwrap(),
            0
        );
        assert_eq!(
            MarketUserState::consume_reserved_blank_land_contracts(&mut data, 1, 16).unwrap_err(),
            NicechunkMarketError::InsufficientReservedLandContracts.into()
        );
    }
}
