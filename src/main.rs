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

    pow_network_simulation()
}

pub fn pow_network_simulation(){
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