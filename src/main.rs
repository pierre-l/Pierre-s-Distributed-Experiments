extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate ring;
extern crate tokio;
extern crate tokio_timer;

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
    fn on_new_connection(&self, connection: MPSCConnection<Message>) {
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

        tokio::spawn(reception);
    }

    fn on_start(&mut self) {
        let (mining_stream, updater) = mining_stream(self.node_id, self.initial_chain.clone());

        tokio::spawn(mining_stream
            .for_each(|_chain|{
                future::ok(())
            })
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

    let network = Network::new(10, 2);
    network.run(move ||{
        let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u8;
        PowNode::new(node_id, chain.clone())
    });

    thread::sleep(Duration::from_millis(1000));
}