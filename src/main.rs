extern crate futures;
extern crate tokio;
extern crate rand;

mod network;

use network::{Network, MPSCConnection};
use futures::{future, Stream, Future};

#[derive(Clone)]
pub struct Message{}

fn main() {
    let network = Network::new(10, 3);
    network.run(||{
        |connection: MPSCConnection<Message>|{
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
        }
    });
    // TODO
}