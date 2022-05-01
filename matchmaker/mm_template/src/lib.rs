use std::{process::Command, io::Write, io, collections::HashMap, time::SystemTime};
use std::collections::HashSet;
use std::fs::OpenOptions;
// use anoma::types::address::Address;
use anoma::types::intent::{Auction, AuctionIntent, CreateAuction,
                           Exchange, FungibleTokenIntent, MatchedExchanges,
                           PlaceBid};
use anoma::types::matchmaker::{AddIntent, AddIntentResult};
use anoma::types::token;
use anoma_macros::Matchmaker;
use anoma::types::key::ed25519::Signed;
use borsh::{BorshDeserialize, BorshSerialize};

// use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use base64;

// use anoma::ledger::vp_env::get_block_height;
// use anoma_vp_prelude::*;

use std::convert::AsMut;
use anoma::types::token::Amount;

fn clone_into_array<A, T>(slice: &[T]) -> A
    where
        A: Default + AsMut<[T]>,
        T: Clone,
{
    let mut a = A::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

fn get_bid_id(str: [u8; 6]) -> u8{
    let mut res: u8 = 0;

    if str[0] == 49 {
        res += 2;
    }
    if str[1] == 49 {
        res += 1;
    }

    return res;
}

fn get_bid_value(str: [u8; 6]) -> u8{
    let mut res: u8 = 0;

    if str[2] == 49 {
        res += 8;
    }
    if str[3] == 49 {
        res += 4;
    }
    if str[4] == 49 {
        res += 2;
    }
    if str[5] == 49 {
        res += 1;
    }

    return res;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BidEntry {
    id: Vec<u8>,
    place_bid: PlaceBid,
    intent: Signed<AuctionIntent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuctionEntry {
    id: Vec<u8>,
    create_auction: CreateAuction,
    intent: Signed<AuctionIntent>,
    bids: Vec<BidEntry>,
    result_calculated: bool,
}

#[derive(Default, Matchmaker)]
struct AuctionMaker {
    auctions_map: HashMap<String, AuctionEntry>,
}

impl AddIntent for AuctionMaker {
    fn add_intent(
        &mut self,
        intent_id: &Vec<u8>,
        intent_data: &Vec<u8>,
    ) -> AddIntentResult {
        let intent = decode_intent_data(&intent_data[..]);
        let auctions = intent.data.auctions.clone();
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        // println!("Trying to resolve auctions");
        // for pair in &self.auctions_map {
        //     let result = try_resolve_auction(pair);
        //     if result.is_some() {
        //         return result.unwrap();
        //     }
        //
        //
        // }

        // println!("intent_id: {:?}", intent_id);

        // for x in &auctions {
        //     println!("data: {:?}", x.data);
        //     println!("signature: {:?}", x.sig);
        // }

        let mut result = None;

        //add new auctions if intent is AuctionIntent
        // println!("trying to add create_auction intents");
        auctions.into_iter().for_each(|auction| {
            if auction.data.create_auction.is_some() {
                add_auction_entry(
                    &mut self.auctions_map,
                    intent_id.to_vec(),
                    auction,
                    intent.clone(),
                    now,
                )
            } else if auction.data.place_bid.is_some() {
                 let res = add_bid_entry(
                    &mut self.auctions_map,
                    intent_id.to_vec(),
                    auction,
                    intent.clone(),
                    now,
                );

                if res.matched_intents.is_some(){
                    result = Some(res);
                }
            }
        });

        return if result.is_none() {
            AddIntentResult {
                tx: None,
                matched_intents: None,
            }
        } else {
            result.unwrap()
        }


    }
}


// ???
// impl PartialEq for ExchangeNode {
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }

/// Add a new node to the graph for the intent
fn add_auction_entry(
    auctions_map: &mut HashMap<String, AuctionEntry>,
    id: Vec<u8>,
    auction: Signed<Auction>,
    intent: Signed<AuctionIntent>,
    now: u64,
) {
    let auc = auction.data.create_auction.unwrap();
    let start = auc.auction_start;
    let end = auc.auction_end;
    let clearance = auc.auction_clearance;

    if now < start && start < end && end < clearance {
        ()
    } else { return; }

    let new_entry = AuctionEntry {
        id,
        create_auction: auc,
        intent,
        bids: vec![],
        result_calculated: false,
    };

    // create a Sha256 object
    let mut hasher = Sha256::new();
    // write input message
    hasher.update(new_entry.intent.try_to_vec().unwrap());
    // read hash digest and consume hasher
    let key = hasher.finalize();
    let key_string = format!("{:x?}", key).replace(&['[', ']', ',', ' '][..], "");

    match auctions_map.get(&key_string) {
        Some(_a) => {
            println!("Hashmap already contains entry with key: {:?}", key_string);
            return;
        }
        None => ()
    }

    auctions_map.insert(key_string.clone(), new_entry);

    println!("auction with id {} was successfully added\n", key_string);
}

/// Add a new node to the graph for the intent
fn add_bid_entry(
    auctions_map: &mut HashMap<String, AuctionEntry>,
    id: Vec<u8>,
    auction: Signed<Auction>,
    intent: Signed<AuctionIntent>,
    now: u64,
) -> AddIntentResult {
    let new_entry = BidEntry {
        id,
        place_bid: auction.data.place_bid.unwrap(),
        intent,
    };

    println!("auction id: {:x?}\n", new_entry.place_bid.auction_id);

    // key.encode_hex::<String>().as_ref()
    match auctions_map.get_mut(&new_entry.place_bid.auction_id) {
        Some(a) => {
            if a.create_auction.auction_start < now && now < a.create_auction.auction_end {
                // push bids at the beginning
                a.bids.push(new_entry.clone());
                println!("bid with auction id {} was successfully added\n", new_entry.place_bid.auction_id);
                AddIntentResult {
                    tx: None,
                    matched_intents: None,
                }
            } else if !a.result_calculated &&
                a.create_auction.auction_end < now &&
                now < a.create_auction.auction_clearance {
                // calculate result in the middle
                // base64 -> binary
                for b in &a.bids {
                    let id = b.place_bid.bid_id;
                    let bytes = base64::decode(&b.place_bid.bid).unwrap();
                    let path = format!("/home/daniil/IdeaProjects/\
                        mk-tfhe-decoupled/build/test/server/client{}/sampleSeq{}.binary", id, id);
                    let mut file = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .create_new(true)
                        .open(path)
                        .unwrap();
                    file.write_all(&bytes).unwrap();
                }
                // run calculation
                let output = Command::new("./mk_tfhe_server-spqlios-fma")
                    .arg("c")
                    .arg("./server")
                    .current_dir("/home/daniil/IdeaProjects/mk-tfhe-decoupled/build/test")
                    .output()
                    .expect("Calculation failed");

                println!("status: {}", output.status);
                io::stdout().write_all(&output.stdout).unwrap();

                a.result_calculated = true;
                println!("auction with id {} was successfully calculated\n", new_entry.place_bid.auction_id);
                AddIntentResult {
                    tx: None,
                    matched_intents: None,
                }
            } else if a.result_calculated &&
                a.create_auction.auction_clearance < now {
                // resolve in the end

                // run Finalization
                let output = Command::new("./mk_tfhe_client-spqlios-fma")
                    .arg("f")
                    .arg("./sampleResult.binary")
                    .current_dir("/home/daniil/IdeaProjects/mk-tfhe-decoupled/build/test")
                    .output()
                    .expect("Finalization failed");

                println!("status: {}", output.status);
                io::stdout().write_all(&output.stdout).unwrap();
                // for (0..(output.stdout.len()-1)).rev(){
                //
                // }
                // let mut str: Vec<u8> = vec![];
                let mut str: [u8; 6] = [0, 0, 0, 0, 0, 0];

                // println!("Result str is {:?}", str);
                str = clone_into_array(&output.stdout[80..86]);

                let bid_id = get_bid_id(str) + 1;
                let bid_value = (get_bid_value(str) as u64) * 1000000;

                println!("Result str is {:?}", str);
                println!("Bid id: {}, Bid value: {}", bid_id, bid_value);

                ////////////////////////////

                let mut tx_data = MatchedExchanges::empty();
                // println!(
                //     "crafting transfer: {}, {}, {}",
                //     first_node.exchange.data.addr.clone(),
                //     last_node.exchange.data.addr.clone(),
                //     last_amount
                // );

                let winner = get_winning_bid(bid_id, &a.bids);

                // tx bid from winner to seller
                tx_data.transfers.insert(token::Transfer {
                    source: winner.unwrap().intent.data.auctions.iter().last().unwrap().data.addr.clone(),
                    target: a.intent.data.auctions.iter().last().unwrap().data.addr.clone(),
                    token: a.create_auction.token_buy.clone(),
                    amount: Amount::from(bid_value),
                });

                // tx auction item from seller to winner
                tx_data.transfers.insert(token::Transfer {
                    source: a.intent.data.auctions.iter().last().unwrap().data.addr.clone(),
                    target: winner.unwrap().intent.data.auctions.iter().last().unwrap().data.addr.clone(),
                    token: a.create_auction.token_sell.clone(),
                    amount: a.create_auction.amount,
                });

                // tx_data.exchanges.insert(
                //     first_node.exchange.data.addr.clone(),
                //     first_node.exchange.clone(),
                // );
                // tx_data.intents.insert(
                //     first_node.exchange.data.addr.clone(),
                //     first_node.intent.clone(),
                // );

                let mut intent_ids_to_remove = HashSet::new();

                for b in &a.bids {
                    intent_ids_to_remove.insert(b.id.clone());
                }

                println!("tx data: {:?}", tx_data.transfers);

                ////////////////////////////

                //TODO: remove auction entry, create and return tx

                println!("auction with id {} was successfully cleared\n", new_entry.place_bid.auction_id);
                AddIntentResult {
                    tx: Some(tx_data.try_to_vec().unwrap()),
                    matched_intents: Some(intent_ids_to_remove),
                }
            } else {
                println!("too late or too early for the bid");
                AddIntentResult {
                    tx: None,
                    matched_intents: None,
                }
            }
        }
        None => {
            println!("Hashmap does not contain entry with key: {:?}", new_entry.place_bid.auction_id);
            AddIntentResult {
                tx: None,
                matched_intents: None,
            }
        }
    }
}

fn get_winning_bid(bid_id: u8, bids: &Vec<BidEntry>) -> Option<&BidEntry> {
    for b in bids {
        if b.place_bid.bid_id == bid_id {
            return Some(b);
        }
    }
    None
}

fn decode_intent_data(
    bytes: &[u8],
) -> Signed<AuctionIntent> {
    Signed::<AuctionIntent>::try_from_slice(bytes).unwrap()
}
