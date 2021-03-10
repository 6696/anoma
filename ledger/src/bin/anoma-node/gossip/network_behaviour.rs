use super::types::{self, NetworkEvent};
use libp2p::gossipsub::{
    self, Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic,
    MessageAuthenticity, MessageId, TopicHash, ValidationMode,
};
use libp2p::{
    identity::Keypair, swarm::NetworkBehaviourEventProcess, NetworkBehaviour,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver, Sender};

impl From<types::Topic> for IdentTopic {
    fn from(topic: types::Topic) -> Self {
        IdentTopic::new(topic.to_string())
    }
}
impl From<types::Topic> for TopicHash {
    fn from(topic: types::Topic) -> Self {
        IdentTopic::from(topic).hash()
    }
}
impl From<&TopicHash> for types::Topic {
    fn from(topic_hash: &TopicHash) -> Self {
        if topic_hash == &TopicHash::from(types::Topic::Dkg) {
            types::Topic::Dkg
        } else if topic_hash == &TopicHash::from(types::Topic::Orderbook) {
            types::Topic::Orderbook
        } else {
            panic!("topic_hash does not correspond to any topic of interest")
        }
    }
}

impl From<GossipsubMessage> for types::NetworkEvent {
    fn from(msg: GossipsubMessage) -> Self {
        Self::Message(types::InternMessage {
            peer: msg
                .source
                .expect("cannot convert message with anonymous message peer"),
            topic: types::Topic::from(&msg.topic),
            message_id: message_id(&msg),
            data: msg.data,
        })
    }
}

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub gossipsub: Gossipsub,
    #[behaviour(ignore)]
    event_chan: Sender<NetworkEvent>,
}
fn message_id(message: &GossipsubMessage) -> MessageId {
    let mut s = DefaultHasher::new();
    message.data.hash(&mut s);
    MessageId::from(s.finish().to_string())
}

impl Behaviour {
    pub fn new(key: Keypair) -> (Self, Receiver<NetworkEvent>) {
        // To content-address message, we can take the hash of message and use it as an ID.

        // Set a custom gossipsub
        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .protocol_id_prefix("orderbook")
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(message_id)
            .validate_messages()
            .build()
            .expect("Valid config");

        let gossipsub: Gossipsub =
            Gossipsub::new(MessageAuthenticity::Signed(key), gossipsub_config)
                .expect("Correct configuration");

        let (event_chan, rx) = channel::<NetworkEvent>(100);
        (
            Self {
                gossipsub,
                event_chan,
            },
            rx,
        )
    }
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for Behaviour {
    // Called when `gossipsub` produces an event.
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message {
            propagation_source,
            message_id,
            message,
        } = event
        {
            println!(
                "Got message of id: {} from peer: {:?}",
                message_id, propagation_source,
            );
            self.event_chan
                .try_send(NetworkEvent::from(message))
                .unwrap();
        }
    }
}
