extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate network_simulator as netsim;

use netsim::blockchain::{Chain, Difficulty, PowNode};
use netsim::network::Network;
use clap::{App, Arg};
use log::LevelFilter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    let matches = App::new("The Blockchain Network Simulation")
        .version("0.1")
        .author("Pierre L. <pierre.larger@gmail.com>")
        .about("Simulates a Proof-of-Work blockchain network")
        .arg(
            Arg::with_name("number_of_nodes")
                .short("n")
                .long("network_size")
                .value_name("NUMBER_OF_NODES")
                .help("Defines the size of the network.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("initiated_connections_per_node")
                .short("c")
                .long("connections")
                .value_name("INITIATED_CONNECTIONS_PER_NODE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("difficulty_factor")
                .short("d")
                .long("difficulty")
                .value_name("DIFFICULTY_FACTOR")
                .help("Number of times the minimum difficult is doubled")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("duration_in_seconds")
                .short("s")
                .long("duration_in_seconds")
                .value_name("DURATION_IN_SECONDS")
                .help("The duration of the simulation in seconds.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mining_delay")
                .short("m")
                .long("mining_delay")
                .value_name("MINING_DELAY_IN_MILLIS")
                .help("The delay between every attempt of a node to mine a new block.")
                .takes_value(true),
        )
        .get_matches();

    let number_of_nodes: u32 = matches
        .value_of("number_of_nodes")
        .unwrap_or("2048")
        .parse()
        .expect("Invalid number of nodes, expected [1-10000]");

    let initiated_connections_per_node: u8 = matches
        .value_of("initiated_connections_per_node")
        .unwrap_or("3")
        .parse()
        .expect("Invalid number of initiated connections per node, expected [1-255]");

    let difficulty_factor: u8 = matches
        .value_of("difficulty_factor")
        .unwrap_or("15")
        .parse()
        .expect("Invalid difficulty factor, expected [1-255]");

    let duration_in_seconds: u64 = matches
        .value_of("duration_in_seconds")
        .unwrap_or("30")
        .parse()
        .expect("Invalid duration in seconds, expected [1-18,446,744,073,709,551,615]");

    let mining_delay: u64 = matches
        .value_of("mining_delay")
        .unwrap_or("10")
        .parse()
        .expect("Invalid hash duration in milliseconds, expected [1-18,446,744,073,709,551,615]");

    pow_network_simulation(
        number_of_nodes,
        initiated_connections_per_node,
        difficulty_factor,
        Duration::from_secs(duration_in_seconds),
        Duration::from_millis(mining_delay),
    )
}

pub fn pow_network_simulation(
    number_of_nodes: u32,
    initiated_connections_per_node: u8,
    difficulty_factor: u8,
    duration: Duration,
    mining_attempt_delay: Duration,
) {
    // Set up a chain.
    let mut difficulty = Difficulty::min_difficulty();
    for _i in 0u8..difficulty_factor {
        difficulty.increase();
    }

    info!("Chain difficulty: {:?}", difficulty);

    let chain = Arc::new(Chain::init_new(difficulty));
    let node_id = AtomicUsize::new(0);

    // Run the blockchain network.
    let network = Network::new(number_of_nodes, initiated_connections_per_node);
    network.run(
        move || {
            let node_id = node_id.fetch_add(1, Ordering::Relaxed) as u32;
            PowNode::new(node_id, chain.clone(), mining_attempt_delay)
        },
        duration,
    );
}
