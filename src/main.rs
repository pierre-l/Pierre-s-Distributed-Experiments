extern crate futures;
extern crate tokio;
extern crate rand;

mod network;

use network::MPSCConnection;
use futures::future;
use futures::Future;
use futures::Stream;
use std::time::Duration;

#[derive(Clone)]
pub struct Message{}

fn main() {
    // TODO Use the tokio runtime instead of a thread per node.
    let nodes = network::new_network(20, 3);
    let mut handles = vec![];
    for node in nodes{
        println!("Starting a new node.");
        let handle = node.run(|connection: MPSCConnection<Message>|{
            println!("Connection received.");
            let (sender, receiver) = connection.split();

            network::node::send_or_panic(&sender, Message{});

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