#[macro_use] extern crate log;
extern crate env_logger;
extern crate ring;
extern crate untrusted;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate bincode;

mod blockchain;
mod crypto;
mod transaction;

use log::LevelFilter;
use ring::error::Unspecified;
use blockchain::Difficulty;
use transaction::Address;
use transaction::TxOut;
use crypto::KeyPairGenerator;
use crypto::Hash;
use transaction::UtxoStore;
use blockchain::Chain;

fn main() {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    let key_pair_generator = KeyPairGenerator::new();

    let wallet = key_pair_generator.random_keypair().ok().unwrap();
    let address = Address::from_pub_key(&wallet.pub_key());

    let mut difficulty = Difficulty::min_difficulty();

    for _i in 0..4 {
        difficulty.increase();
    }

    let chain = Chain::mine_new_genesis(difficulty, address).ok().unwrap();

    chain.verify(chain.head_hash(), &EmptyUtxoStore{}).ok().unwrap();
    info!("Hello world.");
}

struct EmptyUtxoStore;

impl UtxoStore for EmptyUtxoStore{
    fn find(&self, _transaction_hash: &Hash, _txo_index: &u8) -> Option<&TxOut> {
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum Error{
    InvalidNumberOfKeyPairs(String),
    SerializationError(String),
    InvalidAddress,
    InvalidTxAmount,
    CryptographyError,
    InvalidGenesis,
    HeaderAndBodyHashMismatch,
    HeadAndTailHashMismatch,
    InvalidHeaderHash,
    InvalidDifficulty,
    InvalidHeight,
    TooManyInputForCoinbaseTx,
    InvalidCoinbaseAmount,
    HashIsTooHigh,
    UtxoNotFound,
}

impl From<bincode::Error> for Error{
    fn from(err: bincode::Error) -> Self {
        Error::SerializationError(
            format!("Could not properly serialize the transaction. Reason: {}", err)
        )
    }
}

impl From<Unspecified> for Error{
    fn from(_: Unspecified) -> Self {
        Error::CryptographyError
    }
}