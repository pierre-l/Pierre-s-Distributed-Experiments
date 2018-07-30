use std::sync::Arc;
use Error;
use block::Block;
use crypto::Hash;
use transaction::UtxoStore;
use transaction::Address;
use block::COINBASE_AMOUNT;
use transaction::TxOut;
use block::Nonce;
use block::Body;
use block::Difficulty;
use block::Header;
use bincode;

pub struct Chain{
    head: Block,
    tail: Option<Arc<Chain>>,
}

impl Chain{
    pub fn mine_new_genesis(difficulty: Difficulty, coinbase_address: Address) -> Result<Chain, Error> {
        let coinbase_tx_out = TxOut::new(COINBASE_AMOUNT, coinbase_address);
        let body = Body::new(coinbase_tx_out, vec![]);
        let serialized_body = bincode::serialize(&body)?;

        let previous_block_hash = Hash::min();
        let mut header = Header::new(
            Nonce::new(),
            difficulty,
            previous_block_hash,
            0,
            &serialized_body
        )?;

        loop {
            match header.verify() {
                Ok(_) => {
                    let block = Block::new(header, body);

                    let chain = Chain {
                        head: block,
                        tail: None,
                    };

                    return chain.verify(
                        chain.head_hash(),
                        &EmptyUtxoStore
                    ).map(|_|{
                        chain
                    });
                },
                Err(Error::HashIsTooHigh) => {
                    header.increment_nonce()?;
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    pub fn head_hash(&self) -> &Hash {
        &self.head.header().hash()
    }

    // PERFORMANCE an iterative verification would be more efficient and would avoid stack overflow.
    pub fn verify<S>(&self, expected_genesis_hash: &Hash, utxo_store: &S)
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
        } else if self.head.header().hash() == expected_genesis_hash{
            Ok(())
        } else {
            Err(Error::InvalidGenesis)
        }
    }
}

struct EmptyUtxoStore;

impl UtxoStore for EmptyUtxoStore {
    fn find(&self, _transaction_hash: &Hash, _txo_index: &u8) -> Option<&TxOut> {
        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crypto::KeyPairGenerator;
    use transaction::Address;

    struct EmptyUtxoStore;

    impl UtxoStore for EmptyUtxoStore {
        fn find(&self, _transaction_hash: &Hash, _txo_index: &u8) -> Option<&TxOut> {
            None
        }
    }

    #[test]
    fn can_mine_new_chain() {
        mine_new_genesis().ok().unwrap()
    }

    fn mine_new_genesis() -> Result<(), Error>{
        let key_pair_generator = KeyPairGenerator::new();

        let wallet = key_pair_generator.random_keypair().ok().unwrap();
        let address = Address::from_pub_key(&wallet.pub_key());

        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();

        let chain = Chain::mine_new_genesis(difficulty, address)?;

        chain.verify(chain.head_hash(), &EmptyUtxoStore{})
    }
}