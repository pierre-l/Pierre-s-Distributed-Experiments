extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate ring;
extern crate tokio;
extern crate tokio_timer;

use futures::sync::mpsc::UnboundedSender;
use blockchain::{Chain, Difficulty, mining_stream, MiningStateUpdater};
use futures::{future, Future, Stream};
use network::{MPSCConnection, Network, Node};
use network::transport::send_or_panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

mod network;
mod blockchain;
mod flattenselect;

/// Contains a sink to the peer and information about the peer state.
#[derive(Clone)]
pub struct Peer{
    sender: UnboundedSender<Arc<Chain>>,
    known_chain_height: usize,
}

/// Represents the kind of events that can happen in a Proof of Work
/// blockchain node.
/// This enum helps us manipulate everything in the same stream, avoiding
/// concurrency issues, locking and lifetime management.
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

    /// Propagates the new chain to peers and to the mining stream.
    /// The propagation only happens if the update is a chain with a higher
    /// height than the known height of either the peer or the mining stream.
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
    fn run<S>(mut self, connection_stream: S) -> Box<Future<Item=(), Error=()> + Send>
        where S: Stream<Item=MPSCConnection<Arc<Chain>>, Error=()> + Send + 'static {
        let (mining_stream, updater) = mining_stream(self.node_id, self.chain.clone());

        let peer_stream = connection_stream
            .map(move |connection|{
                info!("Connection received.");
                let (sender, receiver) = connection.split();

                let reception = receiver
                    .map(|chain|{
                        EitherPeerOrChain::ChainRemoteUpdate(chain)
                    })
                    .map_err(|_|{
                        panic!()
                    });

                // Send a peer first, then every update received.
                futures::stream::once(Ok(EitherPeerOrChain::Peer(Peer {
                    sender,
                    known_chain_height: 0,
                })))
                    .chain(reception)
            })
        ;
        let peer_stream = flattenselect::new(peer_stream);

        // Joining all these streams helps us avoid concurrency issues, the use of locking and
        // complicated lifetime management.
        let mut peers = vec![];
        let routing_future = peer_stream
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
                        if chain.validate().is_ok(){
                            self.propagate(chain, &mut peers, &updater);
                        } else {
                            error!("Invalid chain.")
                        }
                    }
                }

                future::ok(())
            });

        Box::new(routing_future)
    }
}

fn main() {
    env_logger::init();

    // Set up a chain.
    let mut difficulty = Difficulty::min_difficulty();
    for _i in 0..7{
        difficulty.increase();
    }

    let chain = Arc::new(Chain::init_new(difficulty));
    let node_id = AtomicUsize::new(0);

    // Run the blockchain network.
    let network = Network::new(8, 2);
    network.run(move ||{
        let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u8;
        PowNode::new(node_id, chain.clone())
    }, Duration::from_secs(15));
}