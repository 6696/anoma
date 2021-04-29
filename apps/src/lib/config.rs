//! Node and client configuration
use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;

use libp2p::multiaddr::Multiaddr;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::gossiper::Gossiper;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while reading config: {0}")]
    ReadError(config::ConfigError),
    #[error("Error while deserializing config: {0}")]
    DeserializationError(config::ConfigError),
    #[error("Error while serializing to toml: {0}")]
    TomlError(toml::ser::Error),
    #[error("Error while writing config: {0}")]
    WriteError(std::io::Error),
    #[error("Error while creating config file: {0}")]
    FileError(std::io::Error),
    #[error("A config file already exists in {0}")]
    AlreadyExistingConfig(PathBuf),
}

pub const BASEDIR: &str = ".anoma";
pub const FILENAME: &str = "config.toml";
pub const TENDERMINT_DIR: &str = "tendermint";
pub const DB_DIR: &str = "db";

pub type Result<T> = std::result::Result<T, Error>;
const VALUE_AFTER_TABLE_ERROR_MSG: &str = r#"
Error while serializing to toml. It means that some nested structure is followed
 by simple fields.
This fails:
    struct Nested{
       i:int
    }

    struct Broken{
       nested:Nested,
       simple:int
    }
And this is correct
    struct Nested{
       i:int
    }

    struct Correct{
       simple:int
       nested:Nested,
    }
"#;

#[derive(Debug, Serialize, Deserialize)]
pub struct Ledger {
    pub tendermint: PathBuf,
    pub db_type: String,
    pub db_path: PathBuf,
    pub address: SocketAddr,
    pub network: String,
}

impl Default for Ledger {
    fn default() -> Self {
        Self {
            // this two value are override when generating a default config in
            // config::generate(base_dir). There must be a better way ?
            tendermint: PathBuf::from(BASEDIR).join(TENDERMINT_DIR),
            db_type: "rocksdb".to_owned(),
            db_path: PathBuf::from(BASEDIR).join(DB_DIR),
            address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                26658,
            ),
            network: String::from("mainnet"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Matchmaker {
    pub matchmaker: PathBuf,
    pub tx_template: PathBuf,
    pub ledger_address: SocketAddr,
    pub filter: Option<PathBuf>,
}

// TODO maybe add also maxCount for a maximum number of subscription for a
// filter.

// TODO toml failed to serialize without "untagged" because does not support
// enum with nested data, unless with the untagged flag. This might be a source
// of confusion in the future... Another approach would be to have multiple
// field for each filter possibility but it's less nice.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SubscriptionFilter {
    RegexFilter(#[serde(with = "serde_regex")] Regex),
    WhitelistFilter(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntentBroadcaster {
    pub address: Multiaddr,
    pub rpc: bool,
    pub peers: HashSet<Multiaddr>,
    pub topics: HashSet<String>,
    pub subscription_filter: SubscriptionFilter,
    pub gossiper: Gossiper,
    pub matchmaker: Option<Matchmaker>,
}

impl Default for IntentBroadcaster {
    fn default() -> Self {
        Self {
            // TODO there must be a better option here
            address: Multiaddr::from_str("/ip4/127.0.0.1/tcp/20201").unwrap(),
            rpc: false,
            subscription_filter: SubscriptionFilter::RegexFilter(
                Regex::new("asset_v\\d{1,2}").unwrap(),
            ),
            peers: HashSet::new(),
            topics: vec!["asset_v0"].into_iter().map(String::from).collect(),
            gossiper: Gossiper::new(),
            matchmaker: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub ledger: Option<Ledger>,
    pub intent_broadcaster: Option<IntentBroadcaster>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ledger: Some(Ledger::default()),
            // TODO Should it be None by default
            intent_broadcaster: Some(IntentBroadcaster::default()),
        }
    }
}

impl Config {
    // TODO try to check from any "config.*" file instead of only .yaml
    pub fn read(base_dir_path: &str) -> Result<Self> {
        let file_path = PathBuf::from(base_dir_path).join(FILENAME);
        let mut config = config::Config::new();
        config
            .merge(config::File::with_name(
                file_path.to_str().expect("uncorrect file"),
            ))
            .map_err(Error::ReadError)?;
        config.try_into().map_err(Error::DeserializationError)
    }

    pub fn generate(base_dir_path: &str, replace: bool) -> Result<Self> {
        let base_dir = PathBuf::from(base_dir_path);
        let mut config = Config::default();
        let mut ledger_cfg = config
            .ledger
            .as_mut()
            .expect("safe because default has ledger");
        ledger_cfg.db_path = base_dir.join(DB_DIR);
        ledger_cfg.tendermint = base_dir.join(TENDERMINT_DIR);
        config.write(base_dir, replace)?;
        Ok(config)
    }

    // TODO add format in config instead and serialize it to that format
    fn write(&self, base_dir: PathBuf, replace: bool) -> Result<()> {
        create_dir_all(&base_dir).map_err(Error::FileError)?;
        let file_path = base_dir.join(FILENAME);
        if file_path.exists() && !replace {
            Err(Error::AlreadyExistingConfig(file_path))
        } else {
            let mut file = File::create(file_path).map_err(Error::FileError)?;
            let toml = toml::ser::to_string(&self).map_err(|err| {
                if let toml::ser::Error::ValueAfterTable = err {
                    log::error!("{}", VALUE_AFTER_TABLE_ERROR_MSG);
                }
                Error::TomlError(err)
            })?;
            file.write_all(toml.as_bytes()).map_err(Error::WriteError)
        }
    }
}
