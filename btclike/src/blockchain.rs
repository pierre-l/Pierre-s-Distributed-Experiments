use bincode;
use crypto::Hash;
use crypto::hash;
use Error;
use ring::digest::SHA256_OUTPUT_LEN;
use serde::ser::SerializeTuple;
use serde::Serialize;
use serde::Serializer;
use std::sync::Arc;
use std::u8::MAX as U8_MAX;
use transaction::Address;
use transaction::SignedTx;
use transaction::TxOut;
use transaction::UtxoStore;

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

pub struct Block {
    header: Header,
    body: Body,
}

impl Block{
    pub fn new(header: Header, body: Body) -> Block{
        Block{
            header,
            body,
        }
    }

    pub fn verify<S>(&self, utxo_store: &S) -> Result<(), Error>
        where
            S: UtxoStore,
    {
        self.header.verify()?;
        self.body.verify(utxo_store)?;

        if self.body.hash()? == self.header.hashed_content.body_hash {
            Ok(())
        } else {
            Err(Error::HeaderAndBodyHashMismatch)
        }
    }

    pub fn header(&self) -> &Header{
        &self.header
    }
}

pub struct Header {
    hash: Hash,
    hashed_content: HeaderHashedContent,
}

impl Header {
    pub fn new(
        nonce: Nonce,
        difficulty: Difficulty,
        previous_block_hash: Hash,
        height: u32,
        serialized_body: &[u8],
    ) -> Result<Header, Error>{
        let body_hash = hash(&serialized_body);

        let hashed_content = HeaderHashedContent {
            nonce,
            difficulty,
            previous_block_hash,
            height,
            body_hash,
        };

        Ok(Header{
            hash: hashed_content.hash()?,
            hashed_content,
        })
    }

    pub fn increment_nonce(&mut self) -> Result<(), Error>{
        self.hashed_content.nonce.increment();
        self.hash = self.hashed_content.hash()?;
        Ok(())
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    pub fn previous_block_hash(&self) -> &Hash {
        &self.hashed_content.previous_block_hash
    }

    pub fn difficulty(&self) -> &Difficulty {
        &self.hashed_content.difficulty
    }

    pub fn height(&self) -> &u32 {
        &self.hashed_content.height
    }

    pub fn verify(&self) -> Result<(), Error>{
        let computed_hash = self.hashed_content.hash()?;

        if computed_hash != self.hash {
            Err(Error::InvalidHeaderHash)
        } else if self.difficulty().is_lower_than(computed_hash) {
            Err(Error::HashIsTooHigh)
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize)]
struct HeaderHashedContent {
    nonce: Nonce,
    difficulty: Difficulty,
    previous_block_hash: Hash,
    height: u32,
    body_hash: Hash,
}

impl HeaderHashedContent{
    fn hash(&self) -> Result<Hash, Error> {
        let serialized = bincode::serialize(&self)?;
        Ok(hash(&serialized))
    }
}

pub const COINBASE_AMOUNT:u32 = 1000;

#[derive(Serialize, Clone)]
pub struct Body {
    coinbase_tx_out: TxOut,
    transactions: Vec<SignedTx>,
}

impl Body{
    pub fn new(
        coinbase_tx_out: TxOut,
        transactions: Vec<SignedTx>
    ) -> Body {
        Body{
            coinbase_tx_out,
            transactions,
        }
    }

    pub fn hash(&self) -> Result<Hash, Error> {
        let serialized = bincode::serialize(&self)?;
        Ok(hash(&serialized))
    }

    fn verify<S>(&self, utxo_store: &S) -> Result<(), Error>
        where
            S: UtxoStore
    {
        self.verify_coinbase_tx()?;

        for transaction in &self.transactions {
            transaction.verify(utxo_store)?;
        }

        Ok(())
    }

    fn verify_coinbase_tx(&self) -> Result<(), Error> {
        if self.coinbase_tx_out.amount() != &COINBASE_AMOUNT {
            Err(Error::InvalidCoinbaseAmount)
        } else {
            Ok(())
        }
    }
}

const DIFFICULTY_BYTES_LEN: usize = SHA256_OUTPUT_LEN;
#[derive(Clone, PartialEq, Eq)]
pub struct Difficulty {
    threshold: [u8; SHA256_OUTPUT_LEN],
}

impl Difficulty {
    pub fn min_difficulty() -> Difficulty {
        let array = [U8_MAX as u8; SHA256_OUTPUT_LEN];
        Difficulty { threshold: array }
    }

    pub fn increase(&mut self) {
        self.divide_threshold_by_two()
    }

    fn divide_threshold_by_two(&mut self) {
        let mut index_to_split = 0;

        let max_index = self.threshold.len();
        while self.threshold[index_to_split] == 0 {
            index_to_split += 1;

            if index_to_split >= max_index {
                panic!("Exceeded the maximum difficulty.")
            }
        }

        self.threshold[index_to_split] /= 2;

        if self.threshold[index_to_split] == 0 {
            let next_index = index_to_split + 1;

            if next_index >= max_index {
                panic!("Exceeded the maximum difficulty.")
            }

            self.threshold[next_index] = U8_MAX / 2;
        }
    }

    pub fn is_lower_than(&self, hash: Hash) -> bool {
        &self.threshold < hash.as_ref()
    }
}

impl Serialize for Difficulty
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut seq = serializer.serialize_tuple(DIFFICULTY_BYTES_LEN)?;
        for e in self.threshold.iter() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

#[derive(Serialize, Clone)]
pub struct Nonce(u64);

impl Nonce {
    pub fn new() -> Nonce {
        Nonce(0u64)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

#[cfg(test)]
mod tests {
    use crypto::KeyPairGenerator;
    use super::*;
    use transaction::Address;

    #[test]
    fn can_verify_an_empty_block() {
        let key_pair_generator = KeyPairGenerator::new();

        let account = key_pair_generator.random_keypair().ok().unwrap();
        let address = Address::from_pub_key(&account.pub_key());

        let coinbase_tx_out = TxOut::new(COINBASE_AMOUNT, address);

        let nonce = Nonce::new();
        let difficulty = Difficulty::min_difficulty();
        let body = Body::new(coinbase_tx_out, vec![]);
        let serialized_body = bincode::serialize(&body).ok().unwrap();
        let previous_block_hash = Hash::min();
        let header = Header::new(nonce, difficulty, previous_block_hash, 0, &serialized_body).ok().unwrap();

        let block = Block::new(header, body);

        block.verify(&EmptyUtxoStore{}).ok().unwrap()
    }

    #[test]
    fn can_mine_new_chain() {
        let chain = mine_new_genesis().ok().unwrap();
        verify_genesis_chain(&chain).ok().unwrap();
    }

    #[test]
    fn hash_ensures_integrity() {
        let mut chain = mine_new_genesis().ok().unwrap();
        chain.head.header.hashed_content.height = 1;
        assert_eq!(Error::InvalidHeaderHash, verify_genesis_chain(&chain).err().unwrap());

        let mut chain = mine_new_genesis().ok().unwrap();
        chain.head.header.hashed_content.nonce = Nonce::new();
        assert_eq!(Error::InvalidHeaderHash, verify_genesis_chain(&chain).err().unwrap());

        let mut chain = mine_new_genesis().ok().unwrap();
        chain.head.header.hashed_content.body_hash = hash(b"Garneray");
        assert_eq!(Error::InvalidHeaderHash, verify_genesis_chain(&chain).err().unwrap());

        let mut chain = mine_new_genesis().ok().unwrap();
        chain.head.header.hashed_content.previous_block_hash = hash(b"Garneray");
        assert_eq!(Error::InvalidHeaderHash, verify_genesis_chain(&chain).err().unwrap());

        let mut chain = mine_new_genesis().ok().unwrap();
        chain.head.header.hashed_content.difficulty = Difficulty::min_difficulty();
        assert_eq!(Error::InvalidHeaderHash, verify_genesis_chain(&chain).err().unwrap());

        let mut chain = mine_new_genesis().ok().unwrap();
        chain.head.header.hash = hash(b"Garneray");
        assert_eq!(Error::InvalidHeaderHash, verify_genesis_chain(&chain).err().unwrap());

        let mut chain = mine_new_genesis().ok().unwrap();
        let key_pair_generator = KeyPairGenerator::new();
        let account = key_pair_generator.random_keypair().ok().unwrap();
        let address = Address::from_pub_key(&account.pub_key());
        let coinbase_tx_out = TxOut::new(COINBASE_AMOUNT, address);
        let body = Body::new(coinbase_tx_out, vec![]);
        chain.head.body = body;
        assert_eq!(Error::HeaderAndBodyHashMismatch, verify_genesis_chain(&chain).err().unwrap());
    }

    fn mine_new_genesis() -> Result<Chain, Error>{
        let key_pair_generator = KeyPairGenerator::new();

        let wallet = key_pair_generator.random_keypair().ok().unwrap();
        let address = Address::from_pub_key(&wallet.pub_key());

        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();

        let chain = Chain::mine_new_genesis(difficulty, address)?;

        verify_genesis_chain(&chain)?;

        Ok(chain)
    }

    fn verify_genesis_chain(chain: &Chain) -> Result<(), Error>{
        chain.verify(chain.head_hash(), &EmptyUtxoStore{})
    }
}