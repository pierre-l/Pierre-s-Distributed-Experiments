extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate ring;
extern crate tokio;
extern crate tokio_timer;

use futures::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};
use blockchain::{Chain, Difficulty, mining_stream, MiningStateUpdater};
use futures::{future, Future, Stream};
use network::{MPSCConnection, Network, Node};
use network::transport::send_or_panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

mod network;
mod blockchain;

#[derive(Clone)]
pub struct Peer{
    sender: UnboundedSender<Arc<Chain>>,
    known_chain_height: usize,
}

pub enum EitherPeerOrChain{
    Peer(Peer),
    MinedChain(Arc<Chain>),
    ChainRemoteUpdate(Arc<Chain>),
}

pub struct PowNode{
    node_id: u8,
    chain: Arc<Chain>,
}

impl PowNode{
    pub fn new(node_id: u8, initial_chain: Arc<Chain>,) -> PowNode{
        PowNode{
            node_id,
            chain: initial_chain,
        }
    }

    fn propagate(&mut self, chain: Arc<Chain>, peers: &mut Vec<Peer>, mining_state_updater: &MiningStateUpdater) {
        let chain_height = *chain.height();

        for mut peer in &mut peers.iter_mut(){
            if chain_height > peer.known_chain_height {
                network::transport::send_or_panic(&peer.sender, chain.clone());
                peer.known_chain_height = chain_height;
            }
        }

        if &chain_height > self.chain.height() {
            mining_state_updater.mine_new_chain(chain.clone());
            self.chain = chain;
            debug!("[#{}] New chain with height: {}", self.node_id, chain_height);
        }
    }
}

impl Node<Arc<Chain>> for PowNode{
    fn run<S>(mut self, connection_stream: S)
        where S: Stream<Item=MPSCConnection<Arc<Chain>>, Error=()> + Send + 'static {
        let (mining_stream, updater) = mining_stream(self.node_id, self.chain.clone());

        let (aggregation_sender, aggregation_receiver) = mpsc::unbounded();

        let aggregation_sender_clone = aggregation_sender.clone();
        let connection_future = connection_stream
            .for_each(move |connection|{
                info!("Connection received.");
                let (sender, receiver) = connection.split();

                let peer = Peer {
                    sender,
                    known_chain_height: 0,
                };
                send_or_panic(&aggregation_sender_clone, EitherPeerOrChain::Peer(peer));

                let aggregation_sender_clone = aggregation_sender_clone.clone();
                let reception = receiver
                    .for_each(move |chain|{
                        send_or_panic(&aggregation_sender_clone, EitherPeerOrChain::ChainRemoteUpdate(chain));
                        future::ok(())
                    })
                    .map_err(|_|{
                        panic!()
                    });
                tokio::spawn(reception)
            })
        ;

        let mut peers = vec![];
        let routing_future = aggregation_receiver
            .select(
                mining_stream
                    .map(move |chain|{
                        EitherPeerOrChain::MinedChain(chain)
                    })
            )
            .for_each(move |either_peer_or_chain|{
                match either_peer_or_chain{
                    EitherPeerOrChain::Peer(peer) => {
                        send_or_panic(&peer.sender, self.chain.clone());
                        peers.push(peer);
                        info!("[#{}] New peer. Total: {}", self.node_id, peers.len());
                    },
                    EitherPeerOrChain::MinedChain(chain) => {
                        info!("[#{}] Mined new chain with height {}: {:?}", self.node_id, chain.height(), chain.head().hash().bytes());
                        self.propagate(chain, &mut peers, &updater);
                    },
                    EitherPeerOrChain::ChainRemoteUpdate(chain) => {
                        self.propagate(chain, &mut peers, &updater);
                    }
                }

                future::ok(())
            });

        tokio::spawn(
            future::ok(())
                .join(connection_future)
                .join(routing_future)
                .map(|_|{()})
        );
    }
}

fn main() {
    env_logger::init();

    let mut difficulty = Difficulty::min_difficulty();
    for _i in 0..7{
        difficulty.increase();
    }

    let chain = Arc::new(Chain::init_new(difficulty));
    let node_id = AtomicUsize::new(0);

    let network = Network::new(8, 2);
    network.run(move ||{
        let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u8;
        PowNode::new(node_id, chain.clone())
    });
}