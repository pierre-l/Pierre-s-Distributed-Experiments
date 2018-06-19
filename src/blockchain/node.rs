use futures::sync::mpsc::UnboundedSender;
use blockchain::{Chain, mining_stream, MiningStateUpdater};
use futures::{self, future, Future, Stream};
use network::{MPSCConnection, Node};
use std::sync::Arc;
use std::time::Duration;
use flatten_select;

/// Contains a sink to the peer and information about the peer state.
#[derive(Clone)]
pub struct Peer{
    sender: UnboundedSender<Arc<Chain>>,
    known_chain_height: usize,
    is_closed: bool,
}

/// Represents the events that can happen in a Proof of Work
/// blockchain node.
/// This enum helps us manipulate everything in the same stream, avoiding
/// concurrency issues, locking and lifetime management.
pub enum NodeEvent {
    Peer(Peer),
    MinedChain(Arc<Chain>),
    ChainRemoteUpdate(Arc<Chain>),
}

pub struct PowNode{
    node_id: u32,
    mining_attempt_delay: Duration,
    chain: Arc<Chain>,
}

impl PowNode{
    pub fn new(
        node_id: u32,
        initial_chain: Arc<Chain>,
        mining_attempt_delay: Duration,
    ) -> PowNode{
        PowNode{
            node_id,
            chain: initial_chain,
            mining_attempt_delay,
        }
    }

    /// Propagates the new chain to peers and to the mining stream.
    /// The propagation only happens if the update is a chain with a higher
    /// height than the known height of either the peer or the mining stream.
    fn propagate(&mut self, chain: Arc<Chain>, peers: &mut Vec<Peer>, mining_state_updater: &MiningStateUpdater) {
        let chain_height = *chain.height();

        peers
            .iter_mut()
            .for_each(|peer|{
                if chain_height > peer.known_chain_height {
                    match &peer.sender.unbounded_send(chain.clone()){
                        Ok(()) => {
                            peer.known_chain_height = chain_height;
                        }
                        Err(err) => {
                            info!("Lost connection: {}", err);
                            peer.is_closed = true;
                        }
                    }
                }
            });

        peers
            .retain(|peer|{
                !peer.is_closed
        });

        if chain_height > *self.chain.height() {
            mining_state_updater.mine_new_chain(chain.clone());
            self.chain = chain;
            debug!("[#{}] New chain with height: {}", self.node_id, chain_height);
        }
    }
}

impl Node<Arc<Chain>> for PowNode{
    fn run<S>(mut self, connection_stream: S) -> Box<Future<Item=(), Error=()> + Send>
        where S: Stream<Item=MPSCConnection<Arc<Chain>>, Error=()> + Send + 'static {
        // Start a mining stream.
        let (
            mining_stream, // This stream will yield valid blocks.
            updater// This provides a way to warn the miner that it should mine a new chain
        ) = mining_stream(self.node_id, self.chain.clone(), self.mining_attempt_delay);

        let peer_stream = connection_stream
            .map(move |connection|{
                info!("Connection received.");
                let (sender, receiver) = connection.split();

                let reception = receiver
                    .map(|chain|{
                        NodeEvent::ChainRemoteUpdate(chain)
                    })
                    .map_err(|_|{
                        panic!()
                    });

                // Send a peer first, then every update received.
                futures::stream::once(Ok(NodeEvent::Peer(Peer {
                    sender,
                    known_chain_height: 0,
                    is_closed: false,
                })))
                    .chain(reception)
            })
        ;
        // Flatten this stream so all incoming traffic is considered a single stream.
        let peer_stream = flatten_select::new(peer_stream);

        // Joining all these streams helps us avoid concurrency issues, the use of locking and
        // complicated lifetime management.
        let mut peers = vec![];
        let routing_future = peer_stream
            .select( // This merges the events coming from peers with the events of new mined nodes.
                mining_stream
                    .map(move |chain|{
                        NodeEvent::MinedChain(chain)
                    })
            )
            .for_each(move |node_event|{
                match node_event{
                    NodeEvent::Peer(peer) => {
                        match &peer.sender.unbounded_send(self.chain.clone()) {
                            Ok(()) => {
                                peers.push(peer);
                                info!("[#{}] New peer. Total: {}", self.node_id, peers.len());
                            },
                            Err(err) => {
                                info!("[#{}] Peer lost: {}", self.node_id, err);
                            }
                        }
                    },
                    NodeEvent::MinedChain(chain) => {
                        info!("[#{}] Mined new chain {:?}, height {}", self.node_id, chain.head().hash(), chain.height());
                        self.propagate(chain, &mut peers, &updater);
                    },
                    NodeEvent::ChainRemoteUpdate(chain) => {
                        match chain.validate(){
                            Ok(()) => {
                                self.propagate(chain, &mut peers, &updater);
                            },
                            Err(err) => {
                                error!("Invalid chain: {}", err)
                            },
                        }
                    }
                }

                future::ok(())
            });

        Box::new(routing_future)
    }
}