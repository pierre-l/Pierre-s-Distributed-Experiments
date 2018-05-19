use futures::future;
use futures::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};
use tokio::executor::current_thread;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::thread;
use futures::Stream;
use futures::Future;
use std::thread::JoinHandle;

#[derive(Debug)]
enum TransportMessage<M> {
    Init(MPSCAddress<M>, UnboundedSender<M>),
    Ack(usize, UnboundedSender<M>),
}

#[derive(Clone, Debug)]
pub struct MPSCAddress<M>{
    transport_sender: UnboundedSender<TransportMessage<M>>,
    id: usize, // Necessary for PartialEq
}

impl <M> Eq for MPSCAddress<M>{

}

impl <M> PartialEq for MPSCAddress<M>{
    fn eq(&self, other: &MPSCAddress<M>) -> bool {
        self.id == other.id
    }
}

impl <M> Hash for MPSCAddress<M>{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct MPSCConnection<M>{
    sender: UnboundedSender<M>,
    receiver: UnboundedReceiver<M>,
}

impl <M> MPSCConnection<M>{
    pub fn split(self) -> (UnboundedSender<M>, UnboundedReceiver<M>) {
        (self.sender, self.receiver)
    }
}

pub struct MPSCNode<M> where M: Clone + Send{
    address: MPSCAddress<M>,
    transport_receiver: UnboundedReceiver<TransportMessage<M>>,
    seeds: Vec<MPSCAddress<M>>,
}

impl <M> MPSCNode<M> where M: Clone + Send + 'static{
    pub fn new(address_id: usize) -> MPSCNode<M>{
        let (channel_sender, channel_receiver) = mpsc::unbounded();

        let address = MPSCAddress{
            transport_sender: channel_sender,
            id: address_id,
        };

        MPSCNode{
            address,
            transport_receiver: channel_receiver,
            seeds: vec![],
        }
    }

    pub fn address(&self) -> &MPSCAddress<M>{
        &self.address
    }

    pub fn include_seed(&mut self, address: MPSCAddress<M>){
        self.seeds.push(address);
    }

    pub fn run<A, F>(self, connection_consumer: F) -> JoinHandle<()>
        where
    A: Future<Item=(), Error=()> + 'static,
    F: Fn(MPSCConnection<M>) -> A + Sync + Send + 'static{
        let self_address = self.address;
        let self_address_id = self_address.id;
        let mut connections = HashMap::new();

        for remote_address in &self.seeds {
            let (
                connection_sender,
                connection_receiver,
            ): (UnboundedSender<M>, UnboundedReceiver<M>) = mpsc::unbounded::<M>();
            connections.insert(remote_address.id, connection_receiver);

            let init_message = TransportMessage::Init(self_address.clone(), connection_sender);

            send_or_panic(&remote_address.transport_sender, init_message);
        }

        let node_future = self.transport_receiver.for_each(move |transport_message|{
            match transport_message {
                TransportMessage::Init(remote_address, remote_connection_sender) => {
                    println!("Initiating connection from {} to {}", &remote_address.id, &self_address_id);

                    let connection_sender = MPSCNode::init_new_virtual_connection(remote_connection_sender, &connection_consumer);

                    let ack_message = TransportMessage::Ack(self_address_id, connection_sender);
                    send_or_panic(&remote_address.transport_sender, ack_message);
                },
                TransportMessage::Ack(address_id, sender) => {
                    println!("Ack connection from {} to {}", &self_address_id, &address_id);
                    if let Some(receiver) = connections.remove(&address_id){
                        let connection = MPSCConnection{
                            sender,
                            receiver,
                        };

                        current_thread::spawn(connection_consumer(connection));
                    } else {
                        panic!("Could not find the connection to acknowledge.")
                    }
                },
            };

            future::ok(())
        })
            .then(|_|{
                future::ok(())
            })
            .map_err(|()|{})
        ;


        thread::spawn(move || {
            current_thread::block_on_all(node_future).unwrap_or(());
        })
    }

    fn init_new_virtual_connection<A, F>(remote_connection_sender: UnboundedSender<M>, connection_consumer: &F)
        -> UnboundedSender<M>
        where
            A: Future<Item=(), Error=()> + 'static,
            F: Fn(MPSCConnection<M>) -> A + Sync + Send + 'static
    {
        let (
            connection_sender,
            connection_receiver,
        ): (UnboundedSender<M>, UnboundedReceiver<M>) = mpsc::unbounded::<M>();

        let connection = MPSCConnection{
            sender: remote_connection_sender,
            receiver: connection_receiver,
        };

        current_thread::spawn(connection_consumer(connection));

        connection_sender
    }
}

pub fn send_or_panic<M>(sender: &UnboundedSender<M>, message: M){
    if let Err(_err) = sender.unbounded_send(message){
        panic!("{}", _err)
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use std::time::Duration;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    pub struct Message{}

    #[test]
    fn can_connect_2_nodes_together(){
        let mut nodes = vec![];

        for _i in 0..2{
            let mut node: MPSCNode<Message> = MPSCNode::new(1);
            nodes.push(node);
        }

        let node_b_address = nodes.get(1).unwrap().address.clone();

        {
            let node_a = nodes.get_mut(0).unwrap();

            node_a.include_seed(node_b_address);
        }


        let mut handles = vec![];
        let global_number_of_received_messages = Arc::new(AtomicUsize::new(0));
        for node in nodes{
            let received_messages = global_number_of_received_messages.clone();
            let handle = node.run(move |connection|{
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
            });
            handles.push(handle);
        }

        thread::sleep(Duration::from_millis(100));
        assert_eq!(2, global_number_of_received_messages.load(Ordering::Relaxed))
    }
}