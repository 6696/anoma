//! The CLI commands that are re-used between the executables `anoma`,
//! `anoma-node` and `anoma-client`.
//!
//! The `anoma` executable groups together the most commonly used commands
//! inlined from the node and the client. The other commands for the node or the
//! client can be dispatched via `anoma node ...` or `anoma client ...`,
//! respectively.

use clap::{Clap, FromArgMatches, IntoApp};

const AUTHOR: &str = "Heliax <TODO@heliax.dev>";

// Examples of how to use Clap v3: https://github.com/clap-rs/clap/tree/v3.0.0-beta.2/clap_derive
// Use `cargo expand --lib cli` to see the expanded macros

/// The Anoma CLI
#[derive(Clap)]
#[clap(version = "1.0", author = AUTHOR)]
pub enum AnomaOpts {
    /// Anoma node commands
    Node(LazyOpt),
    /// Anoma client commands
    Client(LazyOpt),
    InlinedNode(NodeOpts),
    #[clap(flatten)]
    InlinedClient(ClientOpts),
}

/// The Anoma Client CLI
#[derive(Clap)]
#[clap(version = "1.0", author = AUTHOR)]
pub enum ClientOpts {
    #[clap(flatten)]
    Inlined(InlinedClientOpts),
}

// `anomac` commands inlined in `anoma`
#[derive(Clap)]
pub enum InlinedClientOpts {
    /// Transfer
    Transfer(Transfer),
    Gossip(Gossip),
}

// `anomac` subcommand for controlling transfers
#[derive(Clap)]
pub struct Transfer {
    /// The transfer source
    #[clap(short)]
    pub src: String,
    /// An example command
    #[clap(short)]
    pub dest: String,
    /// How much to transfer
    #[clap(short)]
    pub amount: u64,
}
// `anomac` subcommand for controlling intent
#[derive(Clap)]
pub struct Gossip {
    /// An example command
    #[clap(short, long)]
    pub orderbook: String,
    #[clap(short, long)]
    pub msg: String,
}

/// The Anoma Node CLI
#[derive(Clap)]
#[clap(version = "1.0", author = AUTHOR)]
pub struct NodeOpts {
    #[clap(short, long)]
    pub base_dir: Option<String>,
    #[clap(short, long)]
    pub rpc: bool,
    #[clap(flatten)]
    pub ops: InlinedNodeOpts,
}

// `anomad` commands inlined in `anoma`
#[derive(Clap)]
pub enum InlinedNodeOpts {
    /// Run the Anoma gossip node daemon
    RunOrderbook(Orderbook),
    /// Run the Anoma node daemon
    RunAnoma,
    /// Reset any store state
    ResetAnoma,
}

#[derive(Clap)]
pub struct Orderbook {
    #[clap(short, long)]
    pub local_address: Option<String>,
    #[clap(short, long)]
    pub peers: Option<Vec<String>>,
    #[clap(short, long)]
    pub topics: Option<Vec<String>>,
}

/// The lazy opt is used for node and client sub-commands, it doesn't actually
/// parse any commands as the commands are dispatched to `anoma-node` and
/// `anoma-client`, respectively.
pub struct LazyOpt;
impl IntoApp for LazyOpt {
    fn into_app<'help>() -> clap::App<'help> {
        clap::App::default()
    }

    fn augment_clap(app: clap::App<'_>) -> clap::App<'_> {
        app
    }
}
impl FromArgMatches for LazyOpt {
    fn from_arg_matches(_matches: &clap::ArgMatches) -> Self {
        Self
    }
}
