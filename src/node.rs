use futures::future;
use futures::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};
use tokio::executor::current_thread;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use futures::Stream;
use futures::Future;

pub struct Message{}

enum TransportMessage {
    Init(MPSCAddress),
    Ack(usize, UnboundedSender<Message>),
}

#[derive(Clone)]
pub struct MPSCAddress{
    transport_sender: UnboundedSender<TransportMessage>,
    id: usize, // Necessary for PartialEq
}

impl Eq for MPSCAddress{

}

impl PartialEq for MPSCAddress{
    fn eq(&self, other: &MPSCAddress) -> bool {
        self.id == other.id
    }
}

impl Hash for MPSCAddress{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct MPSCConnection{
    sender: UnboundedSender<Message>,
    receiver: UnboundedReceiver<Message>,
}

impl MPSCConnection{
    pub fn split(self) -> (UnboundedSender<Message>, UnboundedReceiver<Message>) {
        (self.sender, self.receiver)
    }
}

pub struct MPSCNode{
    address: MPSCAddress,
    transport_receiver: UnboundedReceiver<TransportMessage>,
    seeds: Vec<MPSCAddress>,
}

impl MPSCNode{
    pub fn new(address_id: usize) -> MPSCNode{
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

    pub fn address(&self) -> &MPSCAddress{
        &self.address
    }

    pub fn include_seed(&mut self, address: MPSCAddress){
        self.seeds.push(address);
    }

    pub fn run<F>(self, connection_consumer: F)
        where F: Fn(MPSCConnection) -> () + 'static{
        for address in &self.seeds {
            let init_message = TransportMessage::Init(self.address.clone());

            send_or_panic(&address.transport_sender, init_message);
        }

        let self_address_id = self.address.id;
        let mut connections = HashMap::new();
        let consumer_ref = &connection_consumer;
        let node_future = self.transport_receiver.for_each(|transport_message|{
            match transport_message {
                TransportMessage::Init(remote_address) => {
                    let (
                        connection_sender,
                        connection_receiver,
                    ): (UnboundedSender<Message>, UnboundedReceiver<Message>) = mpsc::unbounded::<Message>();

                    let ack_message = TransportMessage::Ack(self_address_id, connection_sender);
                    connections.insert(remote_address.id, connection_receiver);

                    send_or_panic(&remote_address.transport_sender, ack_message);
                },
                TransportMessage::Ack(address_id, sender) => {
                    if let Some(receiver) = connections.remove(&address_id){
                        let connection = MPSCConnection{
                            sender,
                            receiver
                        };

                        consumer_ref(connection);
                    } else {
                        panic!()
                    }
                }
            }

            future::ok(())
        })
            .then(|_|{
                future::ok(())
            })
            .map_err(|()|{})
        ;

        current_thread::block_on_all(node_future).unwrap_or(());
    }
}

pub fn send_or_panic<M>(sender: &UnboundedSender<M>, message: M){
    if let Err(_err) = sender.unbounded_send(message){
        panic!()
    }
}
