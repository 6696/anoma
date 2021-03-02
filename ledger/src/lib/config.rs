//! Node and client configuration settings

use std::path::PathBuf;

// TODO use https://crates.io/crates/config

pub struct Config {
    pub home_dir: PathBuf,
    // TODO add anoma and tendermint address
}

impl Default for Config {
    fn default() -> Self {
        Self {
            home_dir: PathBuf::from(".anoma"),
        }
    }
}

impl Config {
    pub fn tendermint_home_dir(&self) -> PathBuf {
        self.home_dir.join("tendermint")
    }
    pub fn orderbook_home_dir(&self) -> PathBuf {
        self.home_dir.join("orderbook")
    }
}
