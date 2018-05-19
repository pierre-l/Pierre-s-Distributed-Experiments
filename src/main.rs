extern crate futures;
extern crate tokio;
extern crate rand;

mod node;

use node::{MPSCNode, MPSCAddress, MPSCConnection};
use futures::future;
use futures::Future;
use futures::Stream;
use std::time::Duration;
use rand::Rng;

#[derive(Clone)]
pub struct Message{}

fn main() {
    // TODO Use the tokio runtime instead of a thread per node.
    let nodes = new_network(20, 3);
    let mut handles = vec![];
    for node in nodes{
        println!("Starting a new node.");
        let handle = node.run(|connection: MPSCConnection<Message>|{
            println!("Connection received.");
            let (sender, receiver) = connection.split();

            node::send_or_panic(&sender, Message{});

            let reception = receiver
                .for_each(|_message|{
                    println!("Message received.");
                    future::ok(())
                })
                .map_err(|_|{
                    panic!()
                });

            reception
        });

        handles.push(handle);
    }

    std::thread::sleep(Duration::from_millis(1000));

    for handle in handles{
        drop(handle);
    }
}

fn new_network(size: usize, average_number_of_connections_per_node: usize) -> Vec<MPSCNode<Message>>{
    // TODO Avoid connecting the same nodes twice
    let mut nodes = vec![];
    let mut addresses = vec![];

    for i in 0..size {
        let node = MPSCNode::new(i);
        addresses.push(node.address().clone());
        nodes.push(node);
    }

    for node in &mut nodes{
        let mut addresses = addresses.clone();
        for _i in 0..average_number_of_connections_per_node/2 + 1 {
            let seed_index = node.random_different_address(&addresses);

            node.include_seed(addresses.remove(seed_index));
        }
    }

    nodes
}

impl MPSCNode<Message>{
    fn random_different_address(&self, pool: &Vec<MPSCAddress<Message>>) -> usize{
        let mut rng = rand::thread_rng();

        let mut candidate_index = rng.gen_range(0, pool.len());
        let mut candidate = pool.get(candidate_index).unwrap().clone();

        while candidate.eq(self.address()) {
            candidate_index = rng.gen_range(0, pool.len());
            candidate = pool.get(candidate_index).unwrap().clone();
        }

        candidate_index
    }
}