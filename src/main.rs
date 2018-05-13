extern crate futures;
extern crate tokio;

mod node;

use node::MPSCNode;
use futures::future;
use futures::Future;
use futures::Stream;

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

    let mut threads = vec![];
    for node in nodes{
        let thread = std::thread::spawn(move ||{
            println!("Starting a new node.");
            node.run(|connection|{
                println!("Connection received.");
                let (sender, receiver) = connection.split();

                node::send_or_panic(&sender, Message{});

                receiver
                    .into_future()
                    .and_then(|(_first, _rest)|{
                        println!("Message received.");
                        future::ok(())
                    })
                    .map_err(|_|{
                        panic!()
                    })
            });
        });
        threads.push(thread);
    }

    for thread in threads{
        thread.join().unwrap_or(());
    }
}