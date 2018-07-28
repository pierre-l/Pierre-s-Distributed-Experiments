use bincode;
use crypto::Hash;
use crypto::hash;
use Error;
use ring::digest::SHA256_OUTPUT_LEN;
use serde::ser::SerializeTuple;
use serde::Serialize;
use serde::Serializer;
use std::u8::MAX as U8_MAX;
use transaction::SignedTx;
use transaction::UtxoStore;
use transaction::TxOut;

pub struct Block {
    header: Header,
    body: Body,
}

impl Block{
    fn new(header: Header, body: Body) -> Block{
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
    fn new(
        nonce: Nonce,
        difficulty: Difficulty,
        previous_block_hash: Hash,
        height: u32,
        body: &Body,
    ) -> Result<Header, Error>{
        let serialized = bincode::serialize(&body)?;
        let body_hash = hash(&serialized);

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

    fn verify(&self) -> Result<(), Error>{
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

const COINBASE_AMOUNT:u32 = 1000;

#[derive(Serialize)]
struct Body {
    coinbase_tx_out: TxOut,
    transactions: Vec<SignedTx>,
}

impl Body{
    fn new(
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
        self.verify_coinbase_tx();

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

struct SingleEntryUtxoStore(TxOut);

impl UtxoStore for SingleEntryUtxoStore{
    fn find(&self, _transaction_hash: &Hash, _txo_index: &u8) -> &TxOut {
        &self.0
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