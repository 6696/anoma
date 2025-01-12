use std::collections::{HashMap, VecDeque};

use anoma::types::address::Address;
use anoma::types::intent::{Auction, AuctionIntent, CreateAuction, Exchange, FungibleTokenIntent, MatchedExchanges, PlaceBid};
use anoma::types::matchmaker::{AddIntent, AddIntentResult};
use anoma::types::token;
use anoma_macros::Matchmaker;
use borsh::{BorshDeserialize, BorshSerialize};
use good_lp::{
    constraint, default_solver, variable, variables, Expression,
    ResolutionError, SolverModel, Variable, VariableDefinition,
};
use petgraph::graph::{node_index, DiGraph, NodeIndex};
use petgraph::visit::{depth_first_search, Control, DfsEvent, EdgeRef};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Sha256};
use sha2::Digest;

// use anoma::ledger::vp_env::get_block_height;
// use anoma_vp_prelude::*;

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

        println!("intent_id: {:?}", intent_id);

        //TODO: check if intent is defined for an existing auction, and resolve it, if
        // time is over
        for x in &auctions {
            println!("data: {:?}", x.data);
            println!("signature: {:?}", x.sig);

            // println!("auction_end: {:?}", x.data.auction_end);
            println!("create_auction: {:?}", x.data.create_auction);
            println!("place_bid: {:?}", x.data.place_bid);
            // println!("current height: {:?}", get_block_height());
            //TODO: get current height

            let result = try_resolve_auction(
                &mut self.auctions_map,
                intent_id.to_vec(),
                auction,
                intent.clone(),
            );

            if result.is_some() {
                return result.unwrap();
            }
        }

        //TODO: add new auctions if intent is AuctionIntent
        println!("trying to add create_auction intents");
        auctions.into_iter().for_each(|auction| {
            if auction.data.create_auction.is_some() {
                add_auction_entry(
                    &mut self.auctions_map,
                    intent_id.to_vec(),
                    auction,
                    intent.clone(),
                )
            }
        });

        //TODO: add new bid if intent is BidIntent
        println!("trying to add place_bid intents");
        auctions.into_iter().for_each(|auction| {
            if auction.data.place_bid.is_some() {
                add_bid_entry(
                    &mut self.auctions_map,
                    intent_id.to_vec(),
                    auction,
                    intent.clone(),
                )
            }
        });

        AddIntentResult {
            tx: None,
            matched_intents: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BidEntry {
    id: Vec<u8>,
    place_bid: PlaceBid,
    intent: anoma::proto::Signed<AuctionIntent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuctionEntry {
    id: Vec<u8>,
    create_auction: CreateAuction,
    intent: anoma::proto::Signed<AuctionIntent>,
    bids: Vec<BidEntry>,
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
    auction: anoma::proto::Signed<Auction>,
    intent: anoma::proto::Signed<AuctionIntent>,
) {
    let new_entry = AuctionEntry {
        id,
        create_auction: auction.data.create_auction.unwrap().clone(),
        intent,
        bids: vec![],
    };

    // create a Sha256 object
    let mut hasher = Sha256::new();
    // write input message
    hasher.update(new_entry.create_auction);
    // read hash digest and consume hasher
    let key = hasher.finalize();

    if auctions_map.contains_key(&*key[..].encode_hex::<String>()) {
        println!("Hashmap already contains entry with key: {:?}.", key[..]);
        return;
    }

    auctions_map.insert(key[..].encode_hex::<String>(), new_entry.clone());
}

/// Add a new node to the graph for the intent
fn add_bid_entry(
    auctions_map: &mut HashMap<String, AuctionEntry>,
    id: Vec<u8>,
    auction: anoma::proto::Signed<Auction>,
    intent: anoma::proto::Signed<AuctionIntent>,
) {
    let new_entry = BidEntry {
        id,
        place_bid: auction.data.place_bid.unwrap().clone(),
        intent,
    };

    if auctions_map.contains_key(&new_entry.place_bid.auction_id) {
        // println!("Hashmap already contains entry with key: {:?}.", key[..]);
        // TODO:
        return;
    } else {
        println!("No such auction exist with id: {:?}.", new_entry.place_bid.auction_id);
    }
}

/// Add a new node to the graph for the intent
fn try_resolve_auction(
    auctions_map: &mut HashMap<String, AuctionEntry>,
    id: Vec<u8>,
    auction: anoma::proto::Signed<Auction>,
    intent: anoma::proto::Signed<AuctionIntent>,
) -> Option<AddIntentResult> {
    let new_entry = BidEntry {
        id,
        place_bid: auction.data.place_bid.unwrap().clone(),
        intent,
    };

    return if auctions_map.contains_key(&new_entry.place_bid.auction_id) {
        // TODO:
        Some(AddIntentResult {
            tx: None,
            matched_intents: None,
        })
    } else {
        None
    }
}

// /// Find the nodes that are matching the intent on sell side and buy side.
// fn find_nodes_to_update(
//     graph: &DiGraph<ExchangeNode, Address>,
//     new_node: &ExchangeNode,
// ) -> (Vec<NodeIndex>, Vec<NodeIndex>) {
//     let start = node_index(0);
//     let mut connect_sell = Vec::new();
//     let mut connect_buy = Vec::new();
//     depth_first_search(graph, Some(start), |event| {
//         if let DfsEvent::Discover(index, _time) = event {
//             let current_node = &graph[index];
//             if new_node.exchange.data.token_sell
//                 == current_node.exchange.data.token_buy
//             // && new_node.exchange.data.max_sell
//             //     >= current_node.exchange.data.min_buy
//             {
//                 connect_sell.push(index);
//             }
//             if new_node.exchange.data.token_buy
//                 == current_node.exchange.data.token_sell
//             // && current_node.exchange.data.max_sell
//             //     >= new_node.exchange.data.min_buy
//             {
//                 connect_buy.push(index);
//             }
//         }
//         Control::<()>::Continue
//     });
//     (connect_sell, connect_buy)
// }
//
// // The cycle returned by tarjan_scc only contains the node_index in an arbitrary
// // order without edges. we must reorder them to craft the transfer
// fn sort_intents(
//     graph: &DiGraph<ExchangeNode, Address>,
//     matched_intents_indices: &[NodeIndex],
// ) -> Vec<NodeIndex> {
//     let mut cycle_ordered = Vec::new();
//     let mut cycle_intents = VecDeque::from(matched_intents_indices.to_vec());
//     let mut to_connect_node = cycle_intents.pop_front().unwrap();
//     cycle_ordered.push(to_connect_node);
//     while !cycle_intents.is_empty() {
//         let pop_node = cycle_intents.pop_front().unwrap();
//         if graph.contains_edge(to_connect_node, pop_node) {
//             cycle_ordered.push(pop_node);
//             to_connect_node = pop_node;
//         } else {
//             cycle_intents.push_back(pop_node);
//         }
//     }
//     cycle_ordered.reverse();
//     cycle_ordered
// }
//
// /// Try to find matching intents in the graph. If found, returns the tx bytes
// /// and a hash set of the matched intent IDs.
// fn try_match(
//     graph: &mut DiGraph<ExchangeNode, Address>,
// ) -> Option<(Vec<u8>, HashSet<Vec<u8>>)> {
//     // We only use the first found cycle, because an intent cannot be matched
//     // into more than one tx
//     if let Some(mut matchned_intents_indices) =
//     petgraph::algo::tarjan_scc(&*graph).into_iter().next()
//     {
//         // a node is a cycle with itself
//         if matchned_intents_indices.len() > 1 {
//             println!("found a match: {:?}", matchned_intents_indices);
//             // Must be sorted in reverse order because it removes the node by
//             // index otherwise it would not remove the correct node
//             matchned_intents_indices.sort_by(|a, b| b.cmp(a));
//             if let Some(tx_data) =
//             prepare_tx_data(graph, &matchned_intents_indices)
//             {
//                 let removed_intent_ids = matchned_intents_indices
//                     .into_iter()
//                     .filter_map(|i| {
//                         if let Some(removed) = graph.remove_node(i) {
//                             Some(removed.id)
//                         } else {
//                             None
//                         }
//                     })
//                     .collect();
//                 return Some((tx_data, removed_intent_ids));
//             }
//         }
//     }
//     None
// }

// /// Prepare the transaction's data from the matched intents
// fn prepare_tx_data(
//     graph: &DiGraph<ExchangeNode, Address>,
//     matched_intent_indices: &[NodeIndex],
// ) -> Option<Vec<u8>> {
//     println!(
//         "found match; creating tx with {:?} nodes",
//         matched_intent_indices.len()
//     );
//     let matched_intents = sort_intents(graph, matched_intent_indices);
//     let amounts = compute_amounts(graph, &matched_intents);
//
//     match amounts {
//         Ok(res) => {
//             println!(
//                 "amounts: {}",
//                 res.values()
//                     .map(|x| x.to_string())
//                     .collect::<Vec<String>>()
//                     .join(", ")
//             );
//             let mut matched_intents = matched_intents.into_iter();
//             let first_node = matched_intents.next().map(|i| &graph[i]).unwrap();
//             let mut tx_data = MatchedExchanges::empty();
//
//             let last_node =
//                 matched_intents.fold(first_node, |prev_node, intent_index| {
//                     let node = &graph[intent_index];
//                     let exchanged_amount =
//                         *res.get(&node.exchange.data).unwrap();
//                     println!(
//                         "crafting transfer: {}, {}, {}",
//                         node.exchange.data.addr.clone(),
//                         prev_node.exchange.data.addr.clone(),
//                         exchanged_amount
//                     );
//                     tx_data.transfers.insert(create_transfer(
//                         node,
//                         prev_node,
//                         exchanged_amount, /* safe as we have as many amounts
//                                            * as intents */
//                     ));
//                     tx_data.exchanges.insert(
//                         node.exchange.data.addr.clone(),
//                         node.exchange.clone(),
//                     );
//                     tx_data.intents.insert(
//                         node.exchange.data.addr.clone(),
//                         node.intent.clone(),
//                     );
//                     node
//                 });
//             let last_amount = *res.get(&first_node.exchange.data).unwrap();
//             println!(
//                 "crafting transfer: {}, {}, {}",
//                 first_node.exchange.data.addr.clone(),
//                 last_node.exchange.data.addr.clone(),
//                 last_amount
//             );
//             tx_data.transfers.insert(create_transfer(
//                 first_node,
//                 last_node,
//                 last_amount,
//             ));
//             tx_data.exchanges.insert(
//                 first_node.exchange.data.addr.clone(),
//                 first_node.exchange.clone(),
//             );
//             tx_data.intents.insert(
//                 first_node.exchange.data.addr.clone(),
//                 first_node.intent.clone(),
//             );
//             println!("tx data: {:?}", tx_data.transfers);
//             Some(tx_data.try_to_vec().unwrap())
//         }
//         Err(err) => {
//             println!("Invalid exchange: {}.", err);
//             None
//         }
//     }
// }

// fn compute_amounts(
//     graph: &DiGraph<ExchangeNode, Address>,
//     cycle_intents: &[NodeIndex],
// ) -> Result<HashMap<Exchange, token::Amount>, ResolutionError> {
//     let nodes = graph
//         .raw_nodes()
//         .iter()
//         .map(|x| x.weight.exchange.data.clone())
//         .collect::<Vec<Exchange>>();
//     let mut vars = variables!();
//
//     let mut var_set: HashMap<NodeIndex, VariableDefinition> = HashMap::new();
//
//     let mut intent_graph = graph.filter_map(
//         |node_index, node| {
//             if cycle_intents.contains(&node_index) {
//                 let edges = graph.neighbors(node_index);
//
//                 *edges
//                     .map(|target_node_index| {
//                         let target = graph[target_node_index].clone();
//
//                         let variable_definition = variable();
//                         var_set.insert(node_index, variable_definition.clone());
//
//                         let var_def = variable_definition
//                             .min(target.exchange.data.min_buy)
//                             .max(node.exchange.data.max_sell);
//
//                         let var = vars.add(var_def);
//
//                         Some((var, node))
//                     })
//                     .collect::<Vec<Option<(Variable, &ExchangeNode)>>>()
//                     .get(0)
//                     .unwrap()
//             } else {
//                 None
//             }
//         },
//         |_edge_index, edge| Some(edge),
//     );
//
//     let variables_iter = vars.iter_variables_with_def().map(|(var, _)| var);
//     let obj_function: Expression = variables_iter.sum();
//     let mut model = vars.maximise(obj_function).using(default_solver);
//
//     let mut constrains = Vec::new();
//
//     // we need to invert the graph otherwise we are not able to build the
//     // constrains
//     intent_graph.reverse();
//
//     let start = node_index(0);
//     depth_first_search(&intent_graph, Some(start), |event| {
//         if let DfsEvent::Discover(index, _time) = event {
//             let edges = graph.edges(index);
//
//             edges.for_each(|edge| {
//                 let source = intent_graph[edge.source()];
//                 let target = intent_graph[edge.target()];
//
//                 constrains.push((
//                     source.0,
//                     target.0,
//                     target.1.exchange.data.rate_min.0.to_f64().unwrap(),
//                 ));
//             });
//         }
//         Control::<()>::Continue
//     });
//
//     for constrain in constrains.iter() {
//         let constrain = constraint!(constrain.0 >= constrain.1 * constrain.2);
//         model = model.with(constrain);
//     }
//
//     match model.solve() {
//         Ok(solution) => {
//             let mut amount_map = HashMap::new();
//             let amounts = solution
//                 .into_inner()
//                 .iter()
//                 .map(|(_, amount)| token::Amount::from(*amount))
//                 .collect::<Vec<_>>();
//             nodes.iter().enumerate().for_each(|(index, exchange)| {
//                 amount_map.insert(exchange.clone(), amounts[index]);
//             });
//             Ok(amount_map)
//         }
//         Err(error) => Err(error),
//     }
// }
//
// fn create_transfer(
//     from_node: &ExchangeNode,
//     to_node: &ExchangeNode,
//     amount: token::Amount,
// ) -> token::Transfer {
//     token::Transfer {
//         source: from_node.exchange.data.addr.clone(),
//         target: to_node.exchange.data.addr.clone(),
//         token: to_node.exchange.data.token_buy.clone(),
//         amount,
//     }
// }

fn decode_intent_data(
    bytes: &[u8],
) -> anoma::proto::Signed<AuctionIntent> {
    anoma::proto::Signed::<AuctionIntent>::try_from_slice(bytes).unwrap()
}
