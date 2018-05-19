pub mod node;

pub use network::node::{send_or_panic, MPSCConnection};
use network::node::MPSCNode;
use network::node::MPSCAddress;
use rand::{self, Rng};
use futures::Future;
use std::time::Duration;
use std::thread;

pub struct Network<M> where M: Clone + Send + 'static{
    nodes: Vec<MPSCNode<M>>,
}

impl <M> Network<M> where M: Clone + Send + 'static{
    pub fn new(size: usize, average_number_of_connections_per_node: usize)
        -> Network<M> where M: Clone + Send + 'static
    {
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

        Network{
            nodes
        }
    }

    pub fn run<G, F, A>(self, connection_consumer_factory: G)
        where
            A: Future<Item=(), Error=()> + 'static,
            F: Fn(MPSCConnection<M>) -> A + Sync + Send + 'static,
            G: Fn() -> F + 'static
    {
        // TODO Use the tokio runtime instead of a thread per node.
        let nodes = self.nodes;
        let mut handles = vec![];
        for node in nodes{
            println!("Starting a new node.");
            let handle = node.run(connection_consumer_factory());

            handles.push(handle);
        }

        thread::sleep(Duration::from_millis(1000));

        for handle in handles{
            drop(handle);
        }
    }
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