use futures::sync::mpsc::{self, UnboundedSender};
use futures::Stream;
use blockchain::{Chain, Block, pow::Nonce};
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::ops::Add;
use tokio_timer::Interval;

pub struct MiningState {
    chain: Arc<Chain>,
    nonce: Nonce,
    node_id: u32,
}

impl MiningState {
    pub fn new(node_id: u32, chain: Arc<Chain>) -> MiningState {
        MiningState {
            chain,
            nonce: Nonce::new(),
            node_id,
        }
    }
}

#[derive(Clone)]
pub struct MiningStateUpdater {
    sender: UnboundedSender<Arc<Chain>>,
}

impl MiningStateUpdater {
    pub fn new(sender: UnboundedSender<Arc<Chain>>) -> MiningStateUpdater {
        MiningStateUpdater {
            sender,
        }
    }

    pub fn mine_new_chain(&self, new_chain: Arc<Chain>){
        if let Err(_err) = self.sender.unbounded_send(new_chain){
            panic!("Could not notify of new chain: {}", _err)
        }
    }
}

pub fn mining_stream(node_id: u32, chain: Arc<Chain>, attempt_delay: Duration)
    -> (impl Stream<Item=Arc<Chain>, Error=()>, MiningStateUpdater){
    let (updater_sender, updater_receiver) = mpsc::unbounded();

    let mut state = MiningState::new(node_id, chain);

    let mining_state_updater = MiningStateUpdater::new(updater_sender);

    let mining_stream = updater_receiver
        // Merging both streams avoids the need of locking on the state by doing everything sequentially.
        .map(|chain_update|{Some(chain_update)})
        .select(interval_stream(attempt_delay).map(|_instant|{None}))
        // Now we can mine or update the state.
        .map(move |chain_update_option|{
            if let Some(chain_update) = chain_update_option{
                if state.chain.height() < chain_update.height() {
                    state.chain = chain_update.clone();
                    state.nonce = Nonce::new();
                }

                None

            } else {
                match mine(&mut state){
                    MiningResult::Success(mined_new_chain) => {
                        Some(mined_new_chain)
                    }
                    MiningResult::Failure => {
                        None
                    }
                }
            }
        })
        // Filter it so only the mined blocks are returned.
        .filter_map(|chain_option|{ chain_option })
    ;

    (mining_stream, mining_state_updater)
}

/// Returns a stream that yields an item every time the `interval_duration` passes.
///
/// # Arguments
///
/// `interval_duration`: the duration of the interval between two yielded items.
fn interval_stream(interval_duration: Duration) -> impl Stream<Item=Instant, Error=()> {
    let start_instant = Instant::now().add(interval_duration);
    Interval::new(start_instant, interval_duration)
        .map_err(|timer_err|{
            panic!("Timer error: {}", timer_err)
        })
}

enum MiningResult{
    Success(Arc<Chain>),
    Failure,
}

fn mine(state: &mut MiningState) -> MiningResult{
    state.nonce.increment();

    let head_hash = state.chain.head().hash().clone();
    let difficulty = &state.chain.head().difficulty;
    let block = Block::new(state.node_id, state.nonce.clone(), difficulty, head_hash);

    match Chain::expand(&state.chain, block){
        Ok(mined_chain) => {
            debug!("[N#{}] Mined new block with height: {}", state.node_id, mined_chain.height);
            MiningResult::Success(mined_chain)
        },
        Err(err) => {
            debug!("[N#{}] Failed to mine a new block for height {}. Cause: {}", state.node_id, state.chain.height() + 1, err);
            MiningResult::Failure
        }
    }
}