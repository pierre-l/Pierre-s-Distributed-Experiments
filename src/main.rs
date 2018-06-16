extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate ring;
extern crate tokio;
extern crate tokio_timer;

mod network;
mod blockchain;
mod flattenselect;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use blockchain::{PowNode, Chain, Difficulty};
use network::Network;

fn main() {
    env_logger::init();

    pow_network_simulation(
        8,
        2,
        7,
        Duration::from_secs(15),
    )
}

pub fn pow_network_simulation(
    number_of_nodes: u8,
    initiated_connections_per_node: usize,
    difficulty_factor: usize,
    duration: Duration,
){
    // Set up a chain.
    let mut difficulty = Difficulty::min_difficulty();
    for _i in 0..difficulty_factor{
        difficulty.increase();
    }

    let chain = Arc::new(Chain::init_new(difficulty));
    let node_id = AtomicUsize::new(0);

    // Run the blockchain network.
    let network = Network::new(number_of_nodes, initiated_connections_per_node);
    network.run(move ||{
        let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u8;
        PowNode::new(node_id, chain.clone())
    }, duration);
}