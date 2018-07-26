use std::sync::Arc;
use Error;
use block::Block;
use crypto::Hash;
use transaction::UtxoStore;

struct Chain{
    head: Block,
    tail: Option<Arc<Chain>>,
}

impl Chain{
    // TODO Performance: an iterative verification would be more efficient and would avoid stack overflow.
    pub fn verify<S>(&self, expected_genesis_hash: Hash, utxo_store: &S)
        -> Result<(), Error>
    where
        S: UtxoStore,
    {
        self.head.verify(utxo_store)?;

        if let &Some(ref tail) = &self.tail {
            let t_header = tail.head.header();
            let h_header = self.head.header();

            if t_header.previous_block_hash() != h_header.previous_block_hash() {
                return Err(Error::HeadAndTailHashMismatch);
            }

            if t_header.difficulty() != h_header.difficulty() {
                return Err(Error::InvalidDifficulty);
            }

            if t_header.height() + 1 != *h_header.height() {
                return Err(Error::InvalidHeight);
            }

            tail.verify(expected_genesis_hash, utxo_store)
        } else if self.head.header().hash() == &expected_genesis_hash{
            Ok(())
        } else {
            Err(Error::InvalidGenesis)
        }
    }
}