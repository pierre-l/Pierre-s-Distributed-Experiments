extern crate futures;
extern crate tokio;

mod node;

use node::{MPSCNode, MPSCConnection};
use tokio::executor::current_thread;
use futures::future;
use futures::Stream;

#[derive(Clone)]
pub struct Message{}

fn main() {
    let mut node_a = MPSCNode::new(1);

    let mut node_b = MPSCNode::new(2);

    node_a.include_seed(node_b.address().clone());
    node_b.include_seed(node_a.address().clone());

    let thread = std::thread::spawn(move ||{
        node_a.run(connection_main);
    });

    node_b.run(connection_main);
    thread.join().unwrap_or(());
}

fn connection_main(connection: MPSCConnection<Message>){
    println!("Connection received.");
    let (sender, receiver) = connection.split();

    node::send_or_panic(&sender, Message{});

    let incoming = receiver.for_each(|_message|{
        println!("Message received.");
        future::ok(())
    });

    current_thread::spawn(incoming);
}