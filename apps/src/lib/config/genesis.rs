//! The parameters used for the chain's genesis

use std::collections::HashMap;

use anoma::ledger::parameters::Parameters;
use anoma::ledger::pos::{GenesisValidator, PosParams};
use anoma::types::address::Address;
#[cfg(feature = "dev")]
use anoma::types::key::ed25519::Keypair;
use anoma::types::key::ed25519::PublicKey;
use anoma::types::{storage, token};

#[cfg(not(feature = "dev"))]
/// Genesis configuration file format
mod genesis_config {
    use std::collections::HashMap;

    use anoma::ledger::parameters::{EpochDuration, Parameters};
    use anoma::ledger::pos::{GenesisValidator, PosParams};
    use anoma::ledger::pos::types::BasisPoints;
    use anoma::types::address::Address;
    use anoma::types::key::ed25519::PublicKey;
    use anoma::types::{storage, token};
    use hex;
    use serde::Deserialize;

    use super::{EstablishedAccount, Genesis, ImplicitAccount, TokenAccount, Validator};

    #[derive(Debug,Deserialize)]
    struct HexString(String);

    impl HexString {
        pub fn to_bytes(&self) -> Result<Vec<u8>, HexKeyError> {
            let bytes = hex::decode(self.0.to_owned())?;
            Ok(bytes)
        }

        pub fn to_public_key(&self) -> Result<PublicKey, HexKeyError> {
            let bytes = self.to_bytes()?;
            let key = PublicKey::from_bytes(&bytes)?;
            Ok(key)
        }
    }

    #[derive(Debug)]
    enum HexKeyError {
        InvalidHexString,
        InvalidPublicKey,
    }

    impl From<hex::FromHexError> for HexKeyError {
        fn from(_err: hex::FromHexError) -> Self {
            Self::InvalidHexString
        }
    }

    impl From<ed25519_dalek::ed25519::Error> for HexKeyError {
        fn from(_err: ed25519_dalek::ed25519::Error) -> Self {
            Self::InvalidPublicKey
        }
    }

    #[derive(Debug,Deserialize)]
    struct GenesisConfig {
        // Initial validator set
        pub validator: Vec<ValidatorConfig>,
        // Token accounts present at genesis
        pub token: Option<Vec<TokenAccountConfig>>,
        // Established accounts present at genesis
        pub established: Option<Vec<EstablishedAccountConfig>>,
        // Implicit accounts present at genesis
        pub implicit: Option<Vec<ImplicitAccountConfig>>,
        // Protocol parameters
        pub parameters: ParametersConfig,
        // PoS parameters
        pub pos_params: PosParamsConfig,
    }

    #[derive(Debug,Deserialize)]
    struct ValidatorConfig {
        // Public key for consensus. (default: generate)
        consensus_public_key: Option<HexString>,
        // Public key for validator account. (default: generate)
        account_public_key: Option<HexString>,
        // Public key for staking reward account. (default: generate)
        staking_reward_public_key: Option<HexString>,
        // Validator address.
        address: String,
        // Staking reward account address.
        staking_reward_address: String,
        // Total number of tokens held at genesis.
        tokens: u64,
        // Unstaked balance at genesis.
        non_staked_balance: u64,
        // Filename of validator VP. (default: default validator VP)
        validator_vp: Option<String>,
        // Filename of staking reward account VP. (default: user VP)
        staking_reward_vp: Option<String>,
    }

    #[derive(Debug,Deserialize)]
    struct TokenAccountConfig {
        // Address of token account.
        address: String,
        // Filename of token account VP. (default: token VP)
        vp: Option<String>,
        // Initial balances held by addresses.
        balances: Option<HashMap<String, u64>>,
    }

    #[derive(Debug,Deserialize)]
    struct EstablishedAccountConfig {
        // Address of established account.
        address: String,
        // Filename of established account VP. (default: user VP)
        vp: Option<String>,
        // Public key of established account. (default: generate)
        public_key: Option<HexString>,
        // Initial storage key values.
        storage: Option<HashMap<String, HexString>>,
    }

    #[derive(Debug,Deserialize)]
    struct ImplicitAccountConfig {
        // Public key of implicit account.
        public_key: HexString,
    }

    #[derive(Debug,Deserialize)]
    struct ParametersConfig {
        // Minimum number of blocks per epoch.
        min_num_of_blocks: u64,
        // Minimum duration of an epoch (in seconds).
        min_duration: i64,
    }

    #[derive(Debug,Deserialize)]
    struct PosParamsConfig {
        // Maximum number of active validators.
        max_validator_slots: u64,
        // Pipeline length (in epochs).
        pipeline_len: u64,
        // Unbonding length (in epochs).
        unbonding_len: u64,
        // Votes per token (in basis points).
        votes_per_token: u64,
        // Reward for proposing a block.
        block_proposer_reward: u64,
        // Reward for voting on a block.
        block_vote_reward: u64,
        // Portion of a validator's stake that should be slashed on a
        // duplicate vote (in basis points).
        duplicate_vote_slash_rate: u64,
        // Portion of a validator's stake that should be slashed on a
        // light client attack (in basis points).
        light_client_attack_slash_rate: u64,
    }

    fn load_validator(config: &ValidatorConfig) -> Validator {
        Validator {
            pos_data: GenesisValidator {
                address: Address::decode(&config.address).unwrap(),
                staking_reward_address: Address::decode(&config.staking_reward_address).unwrap(),
                tokens: token::Amount::whole(config.tokens),
                consensus_key: config.consensus_public_key.as_ref().unwrap().to_public_key().unwrap(),
                staking_reward_key: config.staking_reward_public_key.as_ref().unwrap().to_public_key().unwrap(),
            },
            account_key: config.account_public_key.as_ref().unwrap().to_public_key().unwrap(),
            non_staked_balance: token::Amount::whole(config.non_staked_balance),
            vp_code_path: config.validator_vp.as_ref().unwrap().to_string(),
        }
    }

    fn load_token(config: &TokenAccountConfig) -> TokenAccount {
        TokenAccount {
            address: Address::decode(&config.address).unwrap(),
            vp_code_path: config.vp.as_ref().unwrap().to_string(),
            balances: config.balances.as_ref().unwrap_or(&HashMap::default())
                .iter().map(|(address, amount)| {
                    (Address::decode(&address).unwrap(),
                     token::Amount::whole(*amount))
                }).collect(),
        }
    }

    fn load_established(config: &EstablishedAccountConfig) -> EstablishedAccount {
        EstablishedAccount {
            address: Address::decode(&config.address).unwrap(),
            vp_code_path: config.vp.as_ref().unwrap().to_string(),
            public_key: match &config.public_key {
                Some(hex) => Some(hex.to_public_key().unwrap()),
                None => None,
            },
            storage: config.storage.as_ref().unwrap_or(&HashMap::default())
                .iter().map(|(address, hex)| {
                    (storage::Key::parse(&address).unwrap(),
                     hex.to_bytes().unwrap())
                }).collect(),
        }
    }

    fn load_implicit(config: &ImplicitAccountConfig) -> ImplicitAccount {
        ImplicitAccount {
            public_key: config.public_key.to_public_key().unwrap(),
        }
    }

    fn load_genesis_config(config: GenesisConfig) -> Genesis {
        let validators = config.validator.iter().map(load_validator).collect();
        let tokens = config.token.unwrap_or(vec![])
            .iter().map(load_token).collect();
        let established = config.established.unwrap_or(vec![])
            .iter().map(load_established).collect();
        let implicit = config.implicit.unwrap_or(vec![])
            .iter().map(load_implicit).collect();

        let parameters = Parameters {
            epoch_duration: EpochDuration {
                min_num_of_blocks: config.parameters.min_num_of_blocks,
                min_duration: anoma::types::time::Duration::seconds(config.parameters.min_duration).into(),
            },
        };

        let pos_params = PosParams {
            max_validator_slots: config.pos_params.max_validator_slots,
            pipeline_len: config.pos_params.pipeline_len,
            unbonding_len: config.pos_params.unbonding_len,
            votes_per_token: BasisPoints::new(config.pos_params.votes_per_token),
            block_proposer_reward: config.pos_params.block_proposer_reward,
            block_vote_reward: config.pos_params.block_vote_reward,
            duplicate_vote_slash_rate: BasisPoints::new(config.pos_params.duplicate_vote_slash_rate),
            light_client_attack_slash_rate: BasisPoints::new(config.pos_params.light_client_attack_slash_rate),
        };

        Genesis {
            validators: validators,
            token_accounts: tokens,
            established_accounts: established,
            implicit_accounts: implicit,
            parameters: parameters,
            pos_params: pos_params,
        }
    }

    pub fn read_genesis_config(path: &str) -> Genesis {
        let config_file = std::fs::read_to_string(path).unwrap();
        load_genesis_config(toml::from_str(&config_file).unwrap())
    }
}

#[derive(Debug)]
pub struct Genesis {
    pub validators: Vec<Validator>,
    pub token_accounts: Vec<TokenAccount>,
    pub established_accounts: Vec<EstablishedAccount>,
    pub implicit_accounts: Vec<ImplicitAccount>,
    pub parameters: Parameters,
    pub pos_params: PosParams,
}

#[derive(Clone, Debug)]
/// Genesis validator definition
pub struct Validator {
    /// Data that is used for PoS system initialization
    pub pos_data: GenesisValidator,
    /// Public key associated with the validator account. The default validator
    /// VP will check authorization of transactions from this account against
    /// this key on a transaction signature.
    /// Note that this is distinct from consensus key used in the PoS system.
    pub account_key: PublicKey,
    /// These tokens are no staked and hence do not contribute to the
    /// validator's voting power
    pub non_staked_balance: token::Amount,
    /// Validity predicate code WASM
    pub vp_code_path: String,
}

#[derive(Clone, Debug)]
pub struct EstablishedAccount {
    /// Address
    pub address: Address,
    /// Validity predicate code WASM
    pub vp_code_path: String,
    /// A public key to be stored in the account's storage, if any
    pub public_key: Option<PublicKey>,
    /// Account's sub-space storage. The values must be borsh encoded bytes.
    pub storage: HashMap<storage::Key, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct TokenAccount {
    /// Address
    pub address: Address,
    /// Validity predicate code WASM
    pub vp_code_path: String,
    /// Accounts' balances of this token
    pub balances: HashMap<Address, token::Amount>,
}

#[derive(Clone, Debug)]
pub struct ImplicitAccount {
    /// A public key from which the implicit account is derived. This will be
    /// stored on chain for the account.
    pub public_key: PublicKey,
}

#[cfg(feature = "dev")]
pub fn genesis() -> Genesis {
    use std::iter::FromIterator;

    use anoma::ledger::parameters::EpochDuration;
    use anoma::types::address;

    use crate::wallet;

    let vp_token_path = "vp_token.wasm";
    let vp_user_path = "vp_user.wasm";

    // NOTE When the validator's key changes, tendermint must be reset with
    // `anoma reset` command. To generate a new validator, use the
    // `tests::gen_genesis_validator` below.
    let consensus_keypair = wallet::defaults::validator_keypair();
    let account_keypair = wallet::defaults::validator_keypair();
    let staking_reward_keypair = Keypair::from_bytes(&[
        61, 198, 87, 204, 44, 94, 234, 228, 217, 72, 245, 27, 40, 2, 151, 174,
        24, 247, 69, 6, 9, 30, 44, 16, 88, 238, 77, 162, 243, 125, 240, 206,
        111, 92, 66, 23, 105, 211, 33, 236, 5, 208, 17, 88, 177, 112, 100, 154,
        1, 132, 143, 67, 162, 121, 136, 247, 20, 67, 4, 27, 226, 63, 47, 57,
    ])
    .unwrap();
    let address = wallet::defaults::validator_address();
    let staking_reward_address = Address::decode("a1qq5qqqqqxaz5vven8yu5gdpng9zrys6ygvurwv3sgsmrvd6xgdzrys6yg4pnwd6z89rrqv2xvjcy9t").unwrap();
    let validator = Validator {
        pos_data: GenesisValidator {
            address,
            staking_reward_address,
            tokens: token::Amount::whole(200_000),
            consensus_key: consensus_keypair.public,
            staking_reward_key: staking_reward_keypair.public,
        },
        account_key: account_keypair.public,
        non_staked_balance: token::Amount::whole(100_000),
        // TODO replace with https://github.com/anoma/anoma/issues/25)
        vp_code_path: vp_user_path.into(),
    };
    let parameters = Parameters {
        epoch_duration: EpochDuration {
            min_num_of_blocks: 10,
            min_duration: anoma::types::time::Duration::minutes(1).into(),
        },
    };
    let albert = EstablishedAccount {
        address: wallet::defaults::albert_address(),
        vp_code_path: vp_user_path.into(),
        public_key: Some(wallet::defaults::albert_keypair().public),
        storage: HashMap::default(),
    };
    let bertha = EstablishedAccount {
        address: wallet::defaults::bertha_address(),
        vp_code_path: vp_user_path.into(),
        public_key: Some(wallet::defaults::bertha_keypair().public),
        storage: HashMap::default(),
    };
    let christel = EstablishedAccount {
        address: wallet::defaults::christel_address(),
        vp_code_path: vp_user_path.into(),
        public_key: Some(wallet::defaults::christel_keypair().public),
        storage: HashMap::default(),
    };
    let matchmaker = EstablishedAccount {
        address: wallet::defaults::matchmaker_address(),
        vp_code_path: vp_user_path.into(),
        public_key: Some(wallet::defaults::matchmaker_keypair().public),
        storage: HashMap::default(),
    };
    let implicit_accounts = vec![ImplicitAccount {
        public_key: wallet::defaults::daewon_keypair().public,
    }];
    let default_user_tokens = token::Amount::whole(1_000_000);
    let balances: HashMap<Address, token::Amount> = HashMap::from_iter([
        (wallet::defaults::albert_address(), default_user_tokens),
        (wallet::defaults::bertha_address(), default_user_tokens),
        (wallet::defaults::christel_address(), default_user_tokens),
        (wallet::defaults::daewon_address(), default_user_tokens),
    ]);
    let token_accounts = address::tokens()
        .into_iter()
        .map(|(address, _)| TokenAccount {
            address,
            vp_code_path: vp_token_path.into(),
            balances: balances.clone(),
        })
        .collect();
    Genesis {
        validators: vec![validator],
        established_accounts: vec![albert, bertha, christel, matchmaker],
        implicit_accounts,
        token_accounts,
        parameters,
        pos_params: PosParams::default(),
    }
}
#[cfg(not(feature = "dev"))]
pub fn genesis() -> Genesis {
    genesis_config::read_genesis_config("genesis/genesis.toml")
}

#[cfg(test)]
pub mod tests {
    use anoma::types::address::testing::gen_established_address;
    use anoma::types::key::ed25519::Keypair;
    use rand::prelude::ThreadRng;
    use rand::thread_rng;

    /// Run `cargo test gen_genesis_validator -- --nocapture` to generate a
    /// new genesis validator address, staking reward address and keypair.
    #[test]
    fn gen_genesis_validator() {
        let address = gen_established_address();
        let staking_reward_address = gen_established_address();
        let mut rng: ThreadRng = thread_rng();
        let keypair = Keypair::generate(&mut rng);
        let staking_reward_keypair = Keypair::generate(&mut rng);
        println!("address: {}", address);
        println!("staking_reward_address: {}", staking_reward_address);
        println!("keypair: {:?}", keypair.to_bytes());
        println!(
            "staking_reward_keypair: {:?}",
            staking_reward_keypair.to_bytes()
        );
    }
}
