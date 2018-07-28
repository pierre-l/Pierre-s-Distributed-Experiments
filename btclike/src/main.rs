#[macro_use] extern crate log;
extern crate env_logger;
extern crate ring;
extern crate untrusted;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate bincode;

mod block;
mod blockchain;
mod crypto;
mod transaction;

use log::LevelFilter;
use ring::error::Unspecified;

fn main() {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    info!("Hello world.");
}

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