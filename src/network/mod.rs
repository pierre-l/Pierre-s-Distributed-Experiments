pub mod node;

pub use network::node::{send_or_panic, MPSCConnection};
use network::node::MPSCNode;
use network::node::MPSCAddress;
use rand::{self, Rng};
use futures::Future;
use std::time::Duration;
use std::thread;
use tokio;
use futures::Stream;
use futures::future;
use futures::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};

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
            A: Future<Item=(), Error=()> + Send + 'static,
            F: Fn(MPSCConnection<M>) -> A + Sync + Send + 'static,
            G: Fn() -> F + Sync + Send + 'static
    {
        let nodes = self.nodes;
        let handle = thread::spawn(move ||{
            let (sender, receiver) = stream_of(nodes);
            let nodes_future = receiver
                .for_each(move |node|{
                    println!("Starting a new node.");
                    node.run(connection_consumer_factory());
                    future::ok(())
                })
            ;

            tokio::run(nodes_future);

            drop(sender);
        });

        thread::sleep(Duration::from_millis(1000));

        drop(handle);
    }
}
fn stream_of<T>(vector: Vec<T>) -> (UnboundedSender<T>, UnboundedReceiver<T>) {
    let (sender, receiver,) = mpsc::unbounded::<T>();

    for item in vector{
        node::send_or_panic(&sender, item);
    }

    (sender, receiver,)
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

#[cfg(test)]
mod tests{
    use super::*;
    use std::time::Duration;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[derive(Clone, Debug)]
    pub struct Message{}

    #[test]
    fn can_create_a_network(){
        let network = Network::new(4, 1);

        let global_number_of_received_messages = Arc::new(AtomicUsize::new(0));
        let received_messages = global_number_of_received_messages.clone();
        network.run(move ||{
            let received_messages = received_messages.clone();
            move |connection|{
                let received_messages = received_messages.clone();
                let (sender, receiver) = connection.split();

                // Send one message per connection received for each node.
                send_or_panic(&sender, Message{});

                receiver
                    .for_each(move |_message|{
                        println!("Message received.");
                        received_messages.fetch_add(1, Ordering::Relaxed);
                        future::ok(())
                    })
                    .map_err(|_|{
                        panic!()
                    })
            }
        });

        thread::sleep(Duration::from_millis(10));
        assert_eq!(8, global_number_of_received_messages.load(Ordering::Relaxed))
    }
}