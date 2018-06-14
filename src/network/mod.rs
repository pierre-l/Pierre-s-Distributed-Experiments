use futures::{Future, stream, Stream};
pub use network::transport::{MPSCConnection, send_or_panic};
use network::transport::MPSCAddress;
use network::transport::MPSCTransport;
use rand::{self, Rng};
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Add;
use std::time::{Duration, Instant};
use tokio;
use tokio_timer::Delay;

pub trait Node<M>{
    fn run<S>(self, connection_stream: S) -> Box<Future<Item=(), Error=()> + Send>
        where S: Stream<Item=MPSCConnection<M>, Error=()> + Send + 'static;
}

pub mod transport;

pub struct Network<M> where M: Clone + Send + 'static{
    transports: Vec<MPSCTransport<M>>,
}

impl <M> Network<M> where M: Clone + Send + 'static{
    pub fn new(size: usize, initiated_connections_per_node: usize)
        -> Network<M> where M: Clone + Send + 'static
    {
        let mut transports = vec![];
        let mut addresses = vec![];
        let mut defined_connections = BiSet::new();

        for i in 0..size {
            let node = MPSCTransport::new(i);
            addresses.push(node.address().clone());
            transports.push(node);
        }

        for transports in &mut transports{
            let mut candidate_addresses = vec![];

            let node_address_id = *transports.address().id();
            for candidate in &addresses{
                let candidate_address_id = *candidate.id();
                if node_address_id != candidate_address_id
                    && !defined_connections.contains(node_address_id, candidate_address_id)
                {
                    candidate_addresses.push(candidate.clone());
                }
            }

            for _i in 0..initiated_connections_per_node {
                let pool_not_empty = !candidate_addresses.is_empty();
                if pool_not_empty {
                    let seed_index = transports.random_different_address(&candidate_addresses);

                    let seed_address = candidate_addresses.remove(seed_index);
                    defined_connections.insert(*seed_address.id(), node_address_id);
                    transports.include_seed(seed_address);
                } else {
                    debug!("Empty pool.");
                }
            }
        }

        Network{
            transports
        }
    }

    pub fn run<N, F>(self, node_factory: F, for_duration: Duration)
        where
            N: Node<M> + Sync + Send + 'static,
            F: Fn() -> N + Send + 'static
    {
        let nodes = self.transports;
        let nodes_future = stream::iter_ok(nodes)
            .for_each(move |transport|{
                info!("Starting a new node.");

                let node_future = node_factory().run(transport.run());
                tokio::spawn(with_timeout(node_future, for_duration))
            });

        tokio::run(
            nodes_future
        );
    }
}

fn with_timeout<F>(future: F, timeout: Duration) -> impl Future<Item=(), Error=()> where F: Future<Item=(), Error=()>{
    let delay_future = Delay::new(Instant::now().add(timeout))
        .map_err(|err|{
            panic!("Timer error: {}", err)
        })
    ;

    future
        .select(delay_future)
        .map(|_|{})
        .map_err(|_|{})
}

impl <M> MPSCTransport<M> where M: Clone + Send + 'static{
    fn random_different_address(&self, pool: &[MPSCAddress<M>]) -> usize{
        let mut rng = rand::thread_rng();
        rng.gen_range(0, pool.len())
    }
}

/// A very naive HashSet for tuples.
/// May not be the most efficient because 'contains' method instantiate a new tuple, requiring
/// owned items.
struct BiSet<T> where T: Hash + Ord{
    inner: HashSet<(T, T)>
}

impl <T> BiSet<T> where T: Hash + Ord{
    pub fn new() -> BiSet<T>{
        BiSet{
            inner: HashSet::new()
        }
    }

    pub fn insert(&mut self, one: T, other: T) {
        if one < other {
            self.inner.insert((one, other));
        } else {
            self.inner.insert((other, one));
        }
    }

    pub fn contains(&self, one: T, other: T) -> bool{
        if one < other {
            self.inner.contains(&(one, other))
        } else {
            self.inner.contains(&(other, one))
        }
    }
}


#[cfg(test)]
mod tests{
    use futures::{future, Future};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
    use super::*;

    #[derive(Clone, Debug)]
    pub struct Message{}

    pub struct TestNode{
        received_messages: Arc<AtomicUsize>,
        connections_established: Arc<AtomicUsize>,
        notified_of_start: Arc<AtomicBool>,
    }

    impl Node<Message> for TestNode{
        fn run<S>(self, connection_stream: S) -> Box<Future<Item=(), Error=()> + Send>
            where S: Stream<Item=MPSCConnection<Message>, Error=()> + Send + 'static {
            self.notified_of_start.store(true, Ordering::Relaxed);

            let connection_future = connection_stream.for_each(move |connection|{
                self.connections_established.fetch_add(1, Ordering::Relaxed);
                let received_messages = self.received_messages.clone();
                let (sender, receiver) = connection.split();

                // Send one message per connection received for each node.
                send_or_panic(&sender, Message{});

                let reception = receiver
                    .for_each(move |_message|{
                        received_messages.fetch_add(1, Ordering::Relaxed);
                        future::ok(())
                    })
                    .map_err(|_|{
                        panic!()
                    });
                tokio::spawn(reception)
            });

            Box::new(connection_future)
        }
    }

    #[test]
    fn can_create_a_network(){
        new_network_test(4, 1);
        new_network_test(8, 2);
        new_network_test(8, 1);
    }

    fn new_network_test(network_size: usize, initiated_connections: usize) {
        let network = Network::new(network_size, initiated_connections);

        let global_number_of_received_messages = Arc::new(AtomicUsize::new(0));
        let notified_of_start = Arc::new(AtomicBool::new(false));
        let connections_established = Arc::new(AtomicUsize::new(0));

        let received_messages_clone = global_number_of_received_messages.clone();
        let notified_of_start_clone = notified_of_start.clone();
        let connections_established_clone = connections_established.clone();

        network.run(move || {
            TestNode {
                received_messages: received_messages_clone.clone(),
                notified_of_start: notified_of_start_clone.clone(),
                connections_established: connections_established_clone.clone(),
            }
        }, Duration::from_secs(5));

        assert_eq!(network_size * 2 * initiated_connections, connections_established.load(Ordering::Relaxed));
        assert_eq!(network_size * 2 * initiated_connections, global_number_of_received_messages.load(Ordering::Relaxed));
        assert!(notified_of_start.load(Ordering::Relaxed));
    }
}