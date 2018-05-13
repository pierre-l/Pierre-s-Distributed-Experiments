extern crate futures;
extern crate tokio;

mod node;

use node::MPSCNode;
use futures::future;
use futures::Future;
use futures::Stream;
use std::time::Duration;

#[derive(Clone)]
pub struct Message{}

fn main() {
    let mut nodes = vec![];
    let mut addresses = vec![];
    for i in 0..2 {
        let node = MPSCNode::new(i);
        addresses.push(node.address().clone());
        nodes.push(node);
    }

    for node in &mut nodes{
        for seed in &addresses{
            if node.address().ne(seed){
                node.include_seed(seed.clone());
            }
        }
    }

    for node in nodes{
        println!("Starting a new node.");
        node.run(|connection|{
            println!("Connection received.");
            let (sender, receiver) = connection.split();

            node::send_or_panic(&sender, Message{});

            receiver
                .for_each(|_message|{
                    println!("Message received.");
                    future::ok(())
                })
                .map_err(|_|{
                    panic!()
                })
        });
    }

    std::thread::sleep(Duration::from_millis(10));
}