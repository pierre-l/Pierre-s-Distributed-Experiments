use futures::Stream;
use blockchain::{Chain, Block, pow::Nonce};
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::ops::Add;
use tokio_timer::Interval;

pub struct MinerState {
    chain: Arc<Chain>,
    nonce: Nonce,
    node_id: u8,
}

impl MinerState {
    pub fn new(node_id: u8, chain: Arc<Chain>) -> MinerState{
        MinerState{
            chain,
            nonce: Nonce::new(),
            node_id,
        }
    }
}

pub fn mine(node_id: u8, chain: Arc<Chain>) -> impl Stream<Item=Arc<Chain>, Error=()>{
    let interval_duration = Duration::from_millis(10);
    let start_instant = Instant::now().add(interval_duration.clone());
    let interval = Interval::new(start_instant, interval_duration);

    let mut state = MinerState::new(node_id, chain);

    interval
        .map(move |_instant|{
            state.nonce.increment();

            let head_hash = state.chain.head().hash().clone();
            let block = Block::new(state.node_id, state.nonce.clone(), head_hash);

            match Chain::expand(&state.chain, block){
                Ok(chain) => {
                    info!("[N#{}] Mined new block with height: {}", state.node_id, chain.height);
                    state.nonce = Nonce::new();
                    state.chain = chain.clone();
                    Some(chain)
                },
                Err(()) => {
                    debug!("[N#{}] Failed to mine a new block", state.node_id);
                    None
                }
            }
        })
        .filter_map(|chain_option|{
            chain_option
        })
        .map_err(|timer_err|{
            panic!("Timer error: {}", timer_err)
        })
}
