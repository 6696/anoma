use std::collections::HashSet;
use std::io::Write;

use anoma::types::intent::{Auction, AuctionIntent, Exchange, FungibleTokenIntent};
use anoma::types::key::ed25519::Signed;
use borsh::BorshSerialize;
#[cfg(not(feature = "ABCI"))]
use tendermint_config::net::Address as TendermintAddress;
#[cfg(feature = "ABCI")]
use tendermint_config_abci::net::Address as TendermintAddress;

use super::signing;
use crate::cli::{self, args, Context};
use crate::proto::services::rpc_service_client::RpcServiceClient;
use crate::proto::{services, RpcMessage};
use crate::wallet::Wallet;
use sha2::{Digest, Sha256};

/// Create an intent, sign it and submit it to the gossip node (unless
/// `to_stdout` is `true`).
pub async fn gossip_intent(
    mut ctx: Context,
    args::Intent {
        node_addr,
        topic,
        source,
        signing_key,
        exchanges,
        ledger_address,
        to_stdout,
    }: args::Intent,
) {
    let mut signed_exchanges: HashSet<Signed<Exchange>> =
        HashSet::with_capacity(exchanges.len());
    for exchange in exchanges {
        let signed =
            sign_exchange(&mut ctx.wallet, exchange, ledger_address.clone())
                .await;
        signed_exchanges.insert(signed);
    }

    let source_keypair = match ctx.get_opt_cached(&signing_key) {
        Some(key) => key,
        None => {
            let source = ctx.get_opt(&source).unwrap_or_else(|| {
                eprintln!("A source or a signing key is required.");
                cli::safe_exit(1)
            });
            signing::find_keypair(
                &mut ctx.wallet,
                &source,
                ledger_address.clone(),
            )
            .await
        }
    };
    let signed_ft: Signed<FungibleTokenIntent> = Signed::new(
        &source_keypair,
        FungibleTokenIntent {
            exchange: signed_exchanges,
        },
    );
    let data_bytes = signed_ft.try_to_vec().unwrap();

    if to_stdout {
        let mut out = std::io::stdout();
        out.write_all(&data_bytes).unwrap();
        out.flush().unwrap();
    } else {
        let node_addr = node_addr.expect(
            "Gossip node address must be defined to submit the intent to it.",
        );
        let topic = topic.expect(
            "The topic must be defined to submit the intent to a gossip node.",
        );

        match RpcServiceClient::connect(node_addr.clone()).await {
            Ok(mut client) => {
                let intent = anoma::proto::Intent::new(data_bytes);
                let message: services::RpcMessage =
                    RpcMessage::new_intent(intent, topic).into();
                let response = client.send_message(message).await.expect(
                    "Failed to send message and/or receive rpc response",
                );
                println!("{:#?}", response);
            }
            Err(e) => {
                eprintln!(
                    "Error connecting RPC client to {}: {}",
                    node_addr, e
                );
            }
        };
    }
}
/// Create an intent, sign it and submit it to the gossip node (unless
/// `to_stdout` is `true`).
pub async fn gossip_auction_intent(
    mut ctx: Context,
    args::AuctionIntent {
        node_addr,
        topic,
        signing_key,
        auctions,
        ledger_address,
        to_stdout,
    }: args::AuctionIntent,
) {
    let mut signed_auctions: HashSet<Signed<Auction>> =
        HashSet::with_capacity(auctions.len());
    for auction in auctions {
        let signed =
            sign_auction(&mut ctx.wallet, auction, ledger_address.clone())
                .await;

        let mut hasher = Sha256::new();
        // write input message
        hasher.update(signed.try_to_vec().unwrap());
        // read hash digest and consume hasher
        let key = hasher.finalize();
        let key_string = format!("{:x?}", key).replace(&['[', ']', ',', ' '][..], "");
        println!("auction id: {}\n", key_string);

        signed_auctions.insert(signed);
    }

    let source_keypair = match ctx.get_opt_cached(&signing_key) {
        Some(key) => key,
        None => {
            eprintln!("A source or a signing key is required.");
            cli::safe_exit(1)
        }
    };
    let signed_ac: Signed<AuctionIntent> = Signed::new(
        &source_keypair,
        AuctionIntent {
            auctions: signed_auctions,
        },
    );
    let data_bytes = signed_ac.try_to_vec().unwrap();

    if to_stdout {
        let mut out = std::io::stdout();
        out.write_all(&data_bytes).unwrap();
        out.flush().unwrap();
    } else {
        let node_addr = node_addr.expect(
            "Gossip node address must be defined to submit the intent to it.",
        );
        let topic = topic.expect(
            "The topic must be defined to submit the intent to a gossip node.",
        );

        match RpcServiceClient::connect(node_addr.clone()).await {
            Ok(mut client) => {
                let intent = anoma::proto::Intent::new(data_bytes);
                let message: services::RpcMessage =
                    RpcMessage::new_intent(intent, topic).into();
                let response = client.send_message(message).await.expect(
                    "Failed to send message and/or receive rpc response",
                );
                println!("{:#?}", response);
            }
            Err(e) => {
                eprintln!(
                    "Error connecting RPC client to {}: {}",
                    node_addr, e
                );
            }
        };
    }
}


/// Request an intent gossip node with a  matchmaker to subscribe to a given
/// topic.
pub async fn subscribe_topic(
    _ctx: Context,
    args::SubscribeTopic { node_addr, topic }: args::SubscribeTopic,
) {
    let mut client = RpcServiceClient::connect(node_addr).await.unwrap();
    let message: services::RpcMessage = RpcMessage::new_topic(topic).into();
    let response = client
        .send_message(message)
        .await
        .expect("failed to send message and/or receive rpc response");
    println!("{:#?}", response);
}

async fn sign_exchange(
    wallet: &mut Wallet,
    exchange: Exchange,
    ledger_address: TendermintAddress,
) -> Signed<Exchange> {
    let source_keypair =
        signing::find_keypair(wallet, &exchange.addr, ledger_address).await;
    Signed::new(&source_keypair, exchange.clone())
}

async fn sign_auction(
    wallet: &mut Wallet,
    auction: Auction,
    ledger_address: TendermintAddress,
) -> Signed<Auction> {
    let source_keypair =
        signing::find_keypair(wallet, &auction.addr, ledger_address).await;
    Signed::new(&source_keypair, auction.clone())
}