//! Proof of Stake system integration with functions for transactions

use anoma::ledger::pos::{
    anoma_proof_of_stake, bond_key, params_key, total_voting_power_key,
    unbond_key, validator_consensus_key_key, validator_set_key,
    validator_staking_reward_address_key, validator_state_key,
    validator_total_deltas_key, validator_voting_power_key, BondId, Bonds,
    PosParams, TotalVotingPowers, Unbonds, ValidatorConsensusKeys,
    ValidatorSets, ValidatorStates, ValidatorTotalDeltas,
    ValidatorVotingPowers,
};
use anoma::types::address::{self, Address, InternalAddress};
use anoma::types::{key, token};
pub use anoma_proof_of_stake::{PoS as PosWrite, PoSReadOnly as PosRead};

use crate::imports::tx;

/// Proof of Stake system
pub struct PoS;

impl anoma_proof_of_stake::PoSReadOnly for PoS {
    type Address = Address;
    type PublicKey = key::ed25519::PublicKey;
    type TokenAmount = token::Amount;
    type TokenChange = token::Change;

    const POS_ADDRESS: Self::Address = Address::Internal(InternalAddress::PoS);

    fn staking_token_address() -> Self::Address {
        address::xan()
    }

    fn read_params(&self) -> PosParams {
        tx::read(params_key().to_string()).unwrap()
    }

    fn read_validator_staking_reward_address(
        &self,
        key: &Self::Address,
    ) -> Option<Self::Address> {
        tx::read(validator_staking_reward_address_key(key).to_string())
    }

    fn read_validator_consensus_key(
        &self,
        key: &Self::Address,
    ) -> Option<ValidatorConsensusKeys> {
        tx::read(validator_consensus_key_key(key).to_string())
    }

    fn read_validator_state(
        &self,
        key: &Self::Address,
    ) -> Option<ValidatorStates> {
        tx::read(validator_state_key(key).to_string())
    }

    fn read_validator_total_deltas(
        &self,
        key: &Self::Address,
    ) -> Option<ValidatorTotalDeltas> {
        tx::read(validator_total_deltas_key(key).to_string())
    }

    fn read_validator_voting_power(
        &self,
        key: &Self::Address,
    ) -> Option<ValidatorVotingPowers> {
        tx::read(validator_voting_power_key(key).to_string())
    }

    fn read_bond(&self, key: &BondId) -> Option<Bonds> {
        tx::read(bond_key(key).to_string())
    }

    fn read_unbond(&self, key: &BondId) -> Option<Unbonds> {
        tx::read(unbond_key(key).to_string())
    }

    fn read_validator_set(&self) -> ValidatorSets {
        tx::read(validator_set_key().to_string()).unwrap()
    }

    fn read_total_voting_power(&self) -> TotalVotingPowers {
        tx::read(total_voting_power_key().to_string()).unwrap()
    }
}

impl anoma_proof_of_stake::PoS for PoS {
    fn write_params(&mut self, params: &PosParams) {
        tx::write(params_key().to_string(), params)
    }

    fn write_validator_staking_reward_address(
        &mut self,
        key: &Self::Address,
        value: Self::Address,
    ) {
        tx::write(
            validator_staking_reward_address_key(key).to_string(),
            &value,
        )
    }

    fn write_validator_consensus_key(
        &mut self,
        key: &Self::Address,
        value: ValidatorConsensusKeys,
    ) {
        tx::write(validator_consensus_key_key(key).to_string(), &value)
    }

    fn write_validator_state(
        &mut self,
        key: &Self::Address,
        value: ValidatorStates,
    ) {
        tx::write(validator_state_key(key).to_string(), &value)
    }

    fn write_validator_total_deltas(
        &mut self,
        key: &Self::Address,
        value: ValidatorTotalDeltas,
    ) {
        tx::write(validator_total_deltas_key(key).to_string(), &value)
    }

    fn write_validator_voting_power(
        &mut self,
        key: &Self::Address,
        value: ValidatorVotingPowers,
    ) {
        tx::write(validator_voting_power_key(key).to_string(), &value)
    }

    fn write_bond(&mut self, key: &BondId, value: Bonds) {
        tx::write(bond_key(key).to_string(), &value)
    }

    fn write_unbond(&mut self, key: &BondId, value: Unbonds) {
        tx::write(unbond_key(key).to_string(), &value)
    }

    fn write_validator_set(&mut self, value: ValidatorSets) {
        tx::write(validator_set_key().to_string(), &value)
    }

    fn write_total_voting_power(&mut self, value: TotalVotingPowers) {
        tx::write(total_voting_power_key().to_string(), &value)
    }

    fn delete_bond(&mut self, key: &BondId) {
        tx::delete(bond_key(key).to_string())
    }

    fn delete_unbond(&mut self, key: &BondId) {
        tx::delete(unbond_key(key).to_string())
    }

    fn transfer(
        &mut self,
        token: &Self::Address,
        amount: Self::TokenAmount,
        src: &Self::Address,
        dest: &Self::Address,
    ) {
        crate::token::tx::transfer(src, dest, token, amount)
    }
}