pub mod node;

use network::node::MPSCNode;
use network::node::MPSCAddress;
pub use network::node::{send_or_panic, MPSCConnection};
use rand::{self, Rng};

pub fn new_network<M>(size: usize, average_number_of_connections_per_node: usize) -> Vec<MPSCNode<M>> where M: Clone + Send + 'static{
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

impl <M> MPSCNode<M> where M: Clone + Send + 'static{
    fn random_different_address(&self, pool: &Vec<MPSCAddress<M>>) -> usize{
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