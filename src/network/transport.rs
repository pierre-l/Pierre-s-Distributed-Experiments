use futures::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use futures::Stream;

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

impl <M> MPSCAddress<M>{
    pub fn id(&self) -> &usize{
        &self.id
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

pub struct MPSCTransport<M> where M: Clone + Send{
    address: MPSCAddress<M>,
    transport_receiver: UnboundedReceiver<TransportMessage<M>>,
    seeds: Vec<MPSCAddress<M>>,
}

impl <M> MPSCTransport<M> where M: Clone + Send + 'static{
    pub fn new(address_id: usize) -> MPSCTransport<M>{
        let (channel_sender, channel_receiver) = mpsc::unbounded();

        let address = MPSCAddress{
            transport_sender: channel_sender,
            id: address_id,
        };

        MPSCTransport {
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

    pub fn run(self,) -> impl Stream<Item=MPSCConnection<M>, Error=()>{
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

        self.transport_receiver.map(move |transport_message|{
            match transport_message {
                TransportMessage::Init(remote_address, remote_connection_sender) => {
                    debug!("Initiating connection from {} to {}", &remote_address.id, &self_address_id);

                    let (
                        connection_sender,
                        connection_receiver,
                    ): (UnboundedSender<M>, UnboundedReceiver<M>) = mpsc::unbounded::<M>();

                    let connection = MPSCConnection{
                        sender: remote_connection_sender,
                        receiver: connection_receiver,
                    };

                    let ack_message = TransportMessage::Ack(self_address_id, connection_sender);
                    send_or_panic(&remote_address.transport_sender, ack_message);

                    connection
                },
                TransportMessage::Ack(address_id, sender) => {
                    debug!("Ack connection from {} to {}", &self_address_id, &address_id);
                    if let Some(receiver) = connections.remove(&address_id){
                        let connection = MPSCConnection{
                            sender,
                            receiver,
                        };

                        connection
                    } else {
                        panic!("Could not find the connection to acknowledge.")
                    }
                },
            }
        })
    }
}

pub fn send_or_panic<M>(sender: &UnboundedSender<M>, message: M){
    if let Err(_err) = sender.unbounded_send(message){
        panic!("{}", _err)
    }
}