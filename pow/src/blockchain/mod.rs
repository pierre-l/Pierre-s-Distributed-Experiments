mod miner;
mod node;
mod pow;

pub use self::miner::{mining_stream, MiningStateUpdater};
pub use self::node::PowNode;
pub use self::pow::Difficulty;
use blockchain::pow::{Hash, Nonce};
use ring::digest::SHA256_OUTPUT_LEN;
use std::sync::Arc;
use std::u32::MAX as U32_MAX;

pub struct Block {
    /// in order to protect these fields to being tampered with, all of them
    /// are used as a the hash input.
    hash: Hash,
    /// Including the node id among these fields will make every node produce a
    /// different hash. If every node was to mine the exact same data, it would
    /// dramatically increase the probability of natural chain forks. In the
    /// Bitcoin network, this is already enforced by the coinbase transaction.
    node_id: u32,
    /// The nonce field enables a node to produce a different hash for every
    /// mining attempt.
    nonce: Nonce,
    /// For a block to be added to the chain and accepted by the other nodes,
    /// its hash must be inferior to the difficulty threshold.
    difficulty: Arc<Difficulty>,
    /// By including the hash of the previous block among the input of the hash
    /// of the current block, it links it to the previous one, making the chain
    /// more secure.
    previous_block_hash: Hash,
    /// Including the height in the hash input helps producing different hashes
    /// in the case where all the other fields have the same value for two
    /// different blocks. It has other benefits, like helping identifying a block
    /// or preventing us from having to count all the blocks one by one.
    height: u32,
}

const HEAD_ERROR_INVALID_HASH: &str = "Invalid hash";
const HEAD_ERROR_HASH_HIGHER_THAN_DIFFICULTY: &str = "Hash higher than difficulty";

impl Block {
    pub fn new(
        node_id: u32,
        nonce: Nonce,
        difficulty: &Arc<Difficulty>,
        previous_block_hash: Hash,
        height: u32,
    ) -> Block {
        let hash = Hash::new(node_id, &nonce, difficulty, height, previous_block_hash.bytes());
        Block {
            node_id,
            nonce,
            hash,
            difficulty: difficulty.clone(),
            height,
            previous_block_hash,
        }
    }

    /// The genesis block is the first block of the chain. It is the same for all nodes.
    pub fn genesis_block(difficulty: Arc<Difficulty>) -> Block {
        let nonce = Nonce::new();
        let genesis_node_id = U32_MAX;
        let height = 0;
        let hash = Hash::new(
            genesis_node_id,
            &nonce,
            &difficulty,
            height,
            &[0u8; SHA256_OUTPUT_LEN],
        );
        Block {
            node_id: genesis_node_id,
            nonce,
            difficulty,
            previous_block_hash: hash.clone(),
            height,
            hash,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.hash.less_than(&self.difficulty) {
            let hash = Hash::new(
                self.node_id,
                &self.nonce,
                &self.difficulty,
                self.height,
                &self.previous_block_hash.bytes(),
            );

            if hash.eq(&self.hash) {
                Ok(())
            } else {
                Err(HEAD_ERROR_INVALID_HASH)
            }
        } else {
            Err(HEAD_ERROR_HASH_HIGHER_THAN_DIFFICULTY)
        }
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }
}

pub struct Chain {
    head: Block,
    tail: Option<Arc<Chain>>,
}

const CHAIN_ERROR_HASH_MISMATCH: &str = "Hash mismatch";
const CHAIN_ERROR_HEIGHT_MISMATCH: &str = "Height mismatch";
const CHAIN_ERROR_INVALID_GENESIS: &str = "Invalid genesis";
const CHAIN_ERROR_INVALID_DIFFICULTY: &str = "Invalid difficulty";

impl Chain {
    pub fn init_new(difficulty: Difficulty) -> Chain {
        Chain {
            head: Block::genesis_block(Arc::new(difficulty)),
            tail: None,
        }
    }

    /// Creates a new chain by adding a block to an existing chain.
    /// Will fail if the block is invalid or the hashes do not match.
    pub fn expand(chain: &Arc<Chain>, block: Block) -> Result<Arc<Chain>, &'static str> {
        let new_chain = Chain::unvalidated_expand(chain, block);

        new_chain.validate_head()?;
        Ok(Arc::new(new_chain))
    }

    fn unvalidated_expand(chain: &Arc<Chain>, block: Block) -> Chain {
        Chain {
            head: block,
            tail: Some(chain.clone()),
        }
    }

    /// The head of the chain is the block at the top of it.
    pub fn head(&self) -> &Block {
        &self.head
    }

    /// The height of the chain is the number of blocks composing the chain.
    /// It is the same that the height of the head block.
    pub fn height(&self) -> u32 {
        self.head.height
    }

    fn hashes_match(chain: &Arc<Chain>, block: &Block) -> bool {
        chain.head.hash.eq(&block.previous_block_hash)
    }

    /// Checks that the chain is valid from head to tail and that it starts from the genesis block.
    /// The current implementation is not the most efficient but is efficient enough
    /// for this simulation.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Err(err) = self.validate_head() {
            return Err(err);
        }

        if let Some(ref tail) = self.tail {
            Chain::validate(tail)
        } else if self.head
            .hash()
            .eq(Block::genesis_block(self.head.difficulty.clone()).hash())
        {
            Ok(())
        } else {
            Err(CHAIN_ERROR_INVALID_GENESIS)
        }
    }

    fn validate_head(&self) -> Result<(), &'static str> {
        if let Some(ref tail) = self.tail {
            match self.head.validate() {
                Ok(()) => {
                    if self.height() == tail.height() + 1 {
                        if Chain::hashes_match(tail, &self.head) {
                            if tail.head.difficulty.eq(&self.head.difficulty) {
                                Ok(())
                            } else {
                                Err(CHAIN_ERROR_INVALID_DIFFICULTY)
                            }
                        } else {
                            Err(CHAIN_ERROR_HASH_MISMATCH)
                        }
                    } else {
                        Err(CHAIN_ERROR_HEIGHT_MISMATCH)
                    }
                }
                Err(err) => Err(err),
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decapitate(chain: Arc<Chain>) -> (Option<Arc<Chain>>, Block) {
        match Arc::try_unwrap(chain) {
            Ok(chain) => (chain.tail, chain.head),
            Err(_err) => panic!(),
        }
    }

    #[test]
    fn can_create_and_expand_a_chain() {
        let (mut chain, node_id, mut nonce) = init_chain();

        chain = mine_5_blocks(chain, node_id, &mut nonce);

        assert!(chain.validate().is_ok());
        assert_eq!(5, chain.height());
    }

    #[test]
    fn cannot_forge_difficulty() {
        let min_difficulty = Difficulty::min_difficulty();

        let (_nonce, mut block, chain) = init_decapitated_chain();
        block.difficulty = Arc::new(min_difficulty.clone());
        assert!(Chain::expand(&chain, block).is_err());

        let (_nonce, mut block, chain) = init_decapitated_chain();
        block.difficulty = Arc::new(min_difficulty);
        assert!(Chain::unvalidated_expand(&chain, block).validate().is_err());
    }

    #[test]
    fn cannot_forge_nonce() {
        let (mut nonce, mut block, chain) = init_decapitated_chain();
        nonce.increment();
        block.nonce = nonce;
        assert!(Chain::expand(&chain, block).is_err());

        let (mut nonce, mut block, chain) = init_decapitated_chain();
        nonce.increment();
        block.nonce = nonce;
        assert!(Chain::unvalidated_expand(&chain, block).validate().is_err());
    }

    fn init_decapitated_chain() -> (Nonce, Block, Arc<Chain>) {
        let (mut chain, node_id, mut nonce) = init_chain();
        chain = mine_5_blocks(chain, node_id, &mut nonce);
        let (tail, block) = decapitate(chain);
        let chain = tail.unwrap();
        (nonce, block, chain)
    }

    fn try_to_mine_next_block(chain: Arc<Chain>, node_id: u32, nonce: &mut Nonce) -> Arc<Chain> {
        nonce.increment();
        let block = Block::new(
            node_id,
            nonce.clone(),
            &chain.head().difficulty,
            chain.head().hash().clone(),
            chain.height() + 1,
        );

        match Chain::expand(&chain, block) {
            Ok(chain) => chain,
            Err(_err) => chain,
        }
    }

    fn mine_5_blocks(mut chain: Arc<Chain>, node_id: u32, nonce: &mut Nonce) -> Arc<Chain> {
        loop {
            chain = try_to_mine_next_block(chain, node_id, nonce);

            if chain.height() == 5 {
                return chain;
            }
        }
    }

    fn init_chain() -> (Arc<Chain>, u32, Nonce) {
        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();
        let chain = Chain::init_new(difficulty);
        let chain = Arc::new(chain);
        let node_id = 1;
        let nonce = Nonce::new();
        (chain, node_id, nonce)
    }
}
