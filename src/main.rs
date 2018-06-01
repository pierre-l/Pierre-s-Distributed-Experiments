#[macro_use] extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio;
extern crate rand;
extern crate ring;

mod network;
mod blockchain;

use network::{Network, MPSCConnection};
use futures::{future, Stream, Future};

#[derive(Clone)]
pub struct Message{}

fn main() {
    env_logger::init();

    let network = Network::new(50, 4);
    network.run(||{
        |connection: MPSCConnection<Message>|{
            debug!("Connection received.");
            let (sender, receiver) = connection.split();

            network::node::send_or_panic(&sender, Message{});

            let reception = receiver
                .for_each(|_message|{
                    debug!("Message received.");
                    future::ok(())
                })
                .map_err(|_|{
                    panic!()
                });

            reception
        }
    });
}