extern crate clap;
extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate rand;
extern crate ring;
extern crate tokio;
extern crate tokio_timer;

use blockchain::{Chain, Difficulty, PowNode};
use clap::{App, Arg};
use network::Network;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

mod network;
mod blockchain;
mod flattenselect;

fn main() {
    env_logger::init();

    let matches = App::new("The Blockchain Network Simulation")
        .version("0.1")
        .author("Pierre L. <pierre.larger@gmail.com>")
        .about("Simulates a Proof-of-Work blockchain network")
        .arg(Arg::with_name("number_of_nodes")
            .short("n")
            .long("network_size")
            .value_name("NUMBER_OF_NODES")
            .help("Defines the size of the network.")
            .takes_value(true))
        .arg(Arg::with_name("initiated_connections_per_node")
            .short("c")
            .long("connections")
            .value_name("INITIATED_CONNECTIONS_PER_NODE")
            .takes_value(true))
        .arg(Arg::with_name("difficulty_factor")
            .short("d")
            .long("difficulty")
            .value_name("DIFFICULTY_FACTOR")
            .help("Number of times the minimum difficult is doubled")
            .takes_value(true))
        .arg(Arg::with_name("duration_in_seconds")
            .short("s")
            .long("duration_in_seconds")
            .value_name("DURATION_IN_SECONDS")
            .help("The duration of the simulation in seconds.")
            .takes_value(true))
        .get_matches();

    let number_of_nodes: u32 = matches
        .value_of("number_of_nodes")
        .unwrap_or("8")
        .parse().expect("Invalid number of nodes, expected [1-4,294,967,295]");

    let initiated_connections_per_node: u8 = matches
        .value_of("initiated_connections_per_node")
        .unwrap_or("2")
        .parse().expect("Invalid number of initiated connections per node, expected [1-255]");

    let difficulty_factor: u8 = matches
        .value_of("difficulty_factor")
        .unwrap_or("7")
        .parse().expect("Invalid difficulty factor, expected [1-255]");

    let duration_in_seconds: u64 = matches
        .value_of("duration_in_seconds")
        .unwrap_or("30")
        .parse().expect("Invalid duration in seconds, expected [1-18,446,744,073,709,551,615]");

    pow_network_simulation(
        number_of_nodes,
        initiated_connections_per_node,
        difficulty_factor,
        Duration::from_secs(duration_in_seconds),
    )
}

pub fn pow_network_simulation(
    number_of_nodes: u32,
    initiated_connections_per_node: u8,
    difficulty_factor: u8,
    duration: Duration,
){
    // Set up a chain.
    let mut difficulty = Difficulty::min_difficulty();
    for _i in 0u8..difficulty_factor{
        difficulty.increase();
    }

    let chain = Arc::new(Chain::init_new(difficulty));
    let node_id = AtomicUsize::new(0);

    // Run the blockchain network.
    let network = Network::new(number_of_nodes, initiated_connections_per_node);
    network.run(move ||{
        let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u32;
        PowNode::new(node_id, chain.clone())
    }, duration);
}