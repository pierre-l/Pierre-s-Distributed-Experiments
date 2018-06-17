mod pow;
mod miner;
mod node;

use std::u32::MAX as U32_MAX;
use std::sync::Arc;
use blockchain::pow::{Hash, Nonce};
use ring::digest::SHA256_OUTPUT_LEN;
pub use self::miner::{mining_stream, MiningStateUpdater};
pub use self::pow::Difficulty;
pub use self::node::PowNode;

pub struct Block{
    node_id: u32,
    nonce: Nonce,
    hash: Hash,
    previous_block_hash: Hash,
}

impl Block{
    pub fn new(node_id: u32, nonce: Nonce, previous_block_hash: Hash) -> Block {
        let hash = Hash::new(node_id, &nonce, previous_block_hash.bytes());
        Block{
            node_id,
            nonce,
            hash,
            previous_block_hash,
        }
    }

    /// The genesis block is the first block of the chain. It is the same for all nodes.
    pub fn genesis_block() -> Block {
        let nonce = Nonce::new();
        let genesis_node_id = U32_MAX;
        let hash = Hash::new(genesis_node_id, &nonce, &[0u8; SHA256_OUTPUT_LEN]);
        Block{
            node_id: genesis_node_id,
            nonce,
            previous_block_hash: hash.clone(),
            hash,
        }
    }

    pub fn is_valid(&self, difficulty: &Arc<Difficulty>) -> bool {
        if self.hash.less_than(difficulty) {
            let hash = Hash::new(self.node_id, &self.nonce, &self.previous_block_hash.bytes());

            hash.eq(&self.hash)
        } else {
            false
        }
    }

    pub fn hash(&self) -> &Hash{
        &self.hash
    }
}

pub struct Chain{
    head: Block,
    tail: Option<Arc<Chain>>,
    difficulty: Arc<Difficulty>,
    height: usize,
}

const CHAIN_ERROR_HASH_MISMATCH: &str = "Hash mismatch";
const CHAIN_ERROR_INVALID_GENESIS: &str = "Invalid genesis";
const CHAIN_ERROR_INVALID_HEAD: &str = "Invalid head";

impl Chain{
    pub fn init_new(difficulty: Difficulty) -> Chain{
        Chain{
            head: Block::genesis_block(),
            tail: None,
            difficulty: Arc::new(difficulty),
            height: 0,
        }
    }

    /// Creates a new chain by adding a block to an existing chain.
    /// Will fail if the block is invalid or the hashes do not match.
    pub fn expand(chain: &Arc<Chain>, block: Block) -> Result<Arc<Chain>, ()> {
        if Chain::hashes_match(&chain, &block)
            && block.is_valid(&chain.difficulty) {
            let new_chain = Chain {
                head: block,
                difficulty: chain.difficulty.clone(),
                height: chain.height + 1,
                tail: Some(chain.clone()),
            };

            Ok(Arc::new(new_chain))
        } else {
            Err(())
        }
    }

    /// The head of the chain is the block at the top of it.
    pub fn head(&self) -> &Block {
        &self.head
    }

    /// The height of the chain is the number of blocks composing the chain.
    /// It is the same that the heigh of the head block.
    pub fn height(&self) -> &usize {
        &self.height
    }

    fn hashes_match(chain: &Arc<Chain>, block: &Block) -> bool {
        chain.head.hash.eq(&block.previous_block_hash)
    }

    /// Checks that the chain is valid from head to tail and that it starts from the genesis block.
    /// The current implementation is not the most efficient but is efficient enough
    /// for this simulation.
    pub fn validate(&self) -> Result<(), &'static str>{
        if let Err(err) = self.validate_head(){
            return Err(err)
        }

        if let Some(ref tail) = self.tail{
            Chain::validate(tail)
        } else if self.head.hash().eq(Block::genesis_block().hash()) {
                Ok(())
        } else {
            Err(CHAIN_ERROR_INVALID_GENESIS)
        }
    }

    fn validate_head(&self) -> Result<(), &'static str>{
        if let Some(ref tail) = self.tail{
            if self.head.is_valid(&self.difficulty) {
                if Chain::hashes_match(tail, &self.head){
                    Ok(())
                } else {
                    Err(CHAIN_ERROR_HASH_MISMATCH)
                }
            } else {
                Err(CHAIN_ERROR_INVALID_HEAD)
            }
        } else {
            Ok(())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_and_expand_a_chain() {
        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();

        let chain = Chain::init_new(difficulty);
        let mut chain = Arc::new(chain);

        let node_id = 1;
        let mut nonce = Nonce::new();

        while {
            nonce.increment();
            let block = Block::new(node_id, nonce.clone(), chain.head().hash().clone());

            let new_chain = match Chain::expand(&chain, block){
                Ok(chain) => {
                    Some(chain)
                },
                Err(()) => {
                    None
                }
            };

            if let Some(new_chain) = new_chain {
                chain = new_chain;
            }

            chain.height < 5
        } {}

        assert!(chain.validate().is_ok())
    }
}