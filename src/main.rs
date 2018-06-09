extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate ring;
extern crate tokio;
extern crate tokio_timer;

use futures::sync::mpsc::{self, UnboundedSender};
use blockchain::{Chain, Difficulty, mining_stream};
use futures::{future, Future, Stream};
use network::{MPSCConnection, Network, Node};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

mod network;
mod blockchain;

#[derive(Clone)]
pub struct Message{}

pub struct PowNode{
    node_id: u8,
    initial_chain: Arc<Chain>,
}

impl PowNode{
    pub fn new(node_id: u8, initial_chain: Arc<Chain>,) -> PowNode{
        PowNode{
            node_id,
            initial_chain,
        }
    }
}

impl Node<Message> for PowNode{
    fn run<S>(self, connection_stream: S)
        where S: Stream<Item=MPSCConnection<Message>, Error=()> + Send + 'static {
        let (mining_stream, updater) = mining_stream(self.node_id, self.initial_chain.clone());

        let mining_future = mining_stream
            .for_each(move |chain|{
                updater.mine_new_chain(chain);
                future::ok(())
            });

        let connection_future = connection_stream
            .for_each(|connection|{
                info!("Connection received.");
                let (sender, receiver) = connection.split();

                network::transport::send_or_panic(&sender, Message{});

                let reception = receiver
                    .for_each(|_message|{
                        info!("Message received.");
                        future::ok(())
                    })
                    .map_err(|_|{
                        panic!()
                    });

                tokio::spawn(reception)
            });

        tokio::spawn(
            mining_future.join(connection_future)
                .map(|_|{()})
        );
    }
}

fn main() {
    env_logger::init();

    let mut difficulty = Difficulty::min_difficulty();
    for _i in 0..10{
        difficulty.increase();
    }

    let chain = Arc::new(Chain::init_new(difficulty));
    let node_id = AtomicUsize::new(0);

    let network = Network::new(4, 1);
    network.run(move ||{
        let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u8;
        PowNode::new(node_id, chain.clone())
    });
}