#[macro_use] extern crate log;
extern crate env_logger;
extern crate futures;
extern crate tokio;
extern crate rand;
extern crate ring;

mod network;
mod blockchain;

use network::{Network, Node, MPSCConnection};
use futures::{future, Stream, Future};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct Message{}

pub struct PowNode{

}

impl PowNode{
    pub fn new() -> PowNode{
        PowNode{}
    }
}

impl Node<Message> for PowNode{
    fn on_new_connection(&self, connection: MPSCConnection<Message>) {
        info!("Connection received.");
        let (sender, receiver) = connection.split();

        network::transport::send_or_panic(&sender, Message{});

        let reception = receiver
            .for_each(|_message|{
                info!("Message received.");
                future::ok(())
            })
            .map_err(|_|{
                panic!()
            });

        tokio::spawn(reception);
    }

    fn on_start(&self) {}
}

fn main() {
    env_logger::init();

    let network = Network::new(10, 1);
    network.run(PowNode::new);

    thread::sleep(Duration::from_millis(1000));
}