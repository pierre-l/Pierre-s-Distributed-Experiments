#[macro_use] extern crate log;
extern crate env_logger;
extern crate ring;
extern crate untrusted;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json; // TODO Use a binary format.

use log::LevelFilter;
use ring::{rand, signature};
use untrusted::Input;
use ring::signature::Ed25519KeyPair;
use ring::rand::SystemRandom;
use ring::digest;
use ring::digest::SHA256;
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

enum Error{
    InvalidNumberOfKeyPairs(String),
    SerializationError(String),
    InvalidAddress,
    TxIoMismatch,
    CryptographyError,
}

impl From<serde_json::Error> for Error{
    fn from(err: serde_json::Error) -> Self {
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

struct KeyPairGenerator{
    rng: SystemRandom,
}

impl KeyPairGenerator{
    fn new() -> KeyPairGenerator {
        KeyPairGenerator{
            rng: rand::SystemRandom::new(),
        }
    }

    fn random_keypair(&self) -> Result<KeyPair, Error>{
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&self.rng)?;

        let key_pair =
            Ed25519KeyPair::from_pkcs8(Input::from(&pkcs8_bytes))?;

        Ok(KeyPair(key_pair))
    }
}

const PUBKEY_LEN: usize = 32;
#[derive(Clone)]
struct PubKey([u8; PUBKEY_LEN]);

const SIGNATURE_LEN: usize = 64;
#[derive(Clone)]
struct Signature([u8; SIGNATURE_LEN]);

const HASH_LEN: usize = 32;
#[derive(Serialize, Clone, Eq, PartialEq)]
struct Hash([u8; HASH_LEN]);

struct KeyPair(Ed25519KeyPair);

impl KeyPair{
    fn pub_key(&self) -> PubKey {
        // PERFORMANCE Not optimal: could get rid of the copy operation.
        let mut bytes = [0u8; PUBKEY_LEN];
        bytes[..PUBKEY_LEN].clone_from_slice(self.0.public_key_bytes());
        PubKey(bytes)
    }

    fn sign(&self, input_bytes: &[u8]) -> Signature {
        // PERFORMANCE Not optimal: could get rid of the copy operation.
        let raw_signature = self.0.sign(input_bytes);

        let mut signature_bytes = [0u8; SIGNATURE_LEN];
        signature_bytes[..SIGNATURE_LEN].clone_from_slice(raw_signature.as_ref());

        Signature(signature_bytes)
    }
}

impl PubKey{
    fn verify_signature(&self, input_bytes: &[u8], signature: &Signature) -> Result<(), Error> {
        let peer_public_key = untrusted::Input::from(&self.0);
        let sig = untrusted::Input::from(&signature.0);
        let msg = untrusted::Input::from(input_bytes);

        signature::verify(&ring::signature::ED25519, peer_public_key, msg, sig)
            .map_err(|err|{
                Error::from(err)
            })
    }
}

fn hash(input_bytes: &[u8]) -> Hash{
    // PERFORMANCE Not optimal: could get rid of the copy operation.
    let digest = digest::digest(&SHA256, &input_bytes);

    let mut hash_bytes = [0u8; HASH_LEN];
    hash_bytes[..HASH_LEN].clone_from_slice(digest.as_ref());

    Hash(hash_bytes)
}

#[derive(Serialize, Clone, PartialEq)]
struct Address(Hash);

impl Address{
    fn from_pub_key(pub_key: &PubKey) -> Address{
        Address(hash(&pub_key.0))
    }
}

#[derive(Serialize, Clone)]
struct RawTxIn{
    prev_tx_hash: Hash,
    prev_tx_output_index: u8,
}

#[derive(Serialize, Clone)]
struct TxOut{
    amount: u32,
    to_address: Address,
}

#[derive(Serialize, Clone)]
struct RawMoveTx{
    input: Vec<RawTxIn>,
    output: Vec<TxOut>,
}

#[derive(Clone)]
struct SignedTxIn{
    prev_tx_hash: Hash,
    prev_tx_output_index: u8,
    tx_signature: Signature,
    sig_public_key: PubKey,
}

impl SignedTxIn{
    fn from_raw_tx_in(raw_tx_in: RawTxIn, serialized_tx: &[u8], key_pair: &KeyPair)
                      -> SignedTxIn
    {
        let signature = key_pair.sign(&serialized_tx);
        let pub_key = key_pair.pub_key();

        SignedTxIn{
            prev_tx_output_index: raw_tx_in.prev_tx_output_index,
            prev_tx_hash: raw_tx_in.prev_tx_hash,
            tx_signature: signature,
            sig_public_key: pub_key,
        }
    }

    fn clone_without_signature(&self) -> RawTxIn {
        RawTxIn{
            prev_tx_hash: self.prev_tx_hash.clone(),
            prev_tx_output_index: self.prev_tx_output_index,
        }
    }

    fn verify_signature(&self, tx_bytes: &[u8]) -> Result<(), Error> {
        self.sig_public_key.verify_signature(tx_bytes, &self.tx_signature)
    }
}

struct SignedMoveTx{
    input: Vec<SignedTxIn>,
    output: Vec<TxOut>,
}

impl SignedMoveTx{
    fn from_raw_tx(raw_tx: RawMoveTx, key_pairs: Vec<&KeyPair>)
                   -> Result<SignedMoveTx, Error>
    {
        let serialized: String = serde_json::to_string(&raw_tx)?;

        let mut raw_input = raw_tx.input;
        let output = raw_tx.output;

        if raw_input.len() != key_pairs.len() {
            return Err(Error::InvalidNumberOfKeyPairs(
                format!("Expected {} key pairs, got {}", raw_input.len(), key_pairs.len()))
            );
        }

        let mut signed_input = vec![];
        for key_pair in key_pairs {
            let raw_tx_in = raw_input.pop().unwrap();

            let signed_tx_in = SignedTxIn::from_raw_tx_in(raw_tx_in, serialized.as_bytes(), key_pair);
            signed_input.push(signed_tx_in);
        }

        Ok(SignedMoveTx{
            input: signed_input,
            output,
        })
    }

    fn clone_without_signatures(&self) -> RawMoveTx{
        let output = self.output.clone();
        let mut input = vec![];

        for signed_input in &self.input {
            input.push(signed_input.clone_without_signature())
        }

        RawMoveTx{
            input,
            output,
        }
    }

    fn verify_signatures(self, prev_tx_outs: &Vec<TxOut>) -> Result<(), Error>{
        if prev_tx_outs.len() != self.input.len() {
            return Err(Error::TxIoMismatch);
        }

        let raw_next_tx = self.clone_without_signatures();
        let serialized = serde_json::to_string(&raw_next_tx)?;

        for (i, prev_tx_out) in prev_tx_outs.iter().enumerate() {
            let tx_in = &self.input[i];
            let address = Address::from_pub_key(&tx_in.sig_public_key);

            if address != prev_tx_out.to_address {
                return Err(Error::InvalidAddress);
            }

            tx_in.verify_signature(serialized.as_bytes())?
        }

        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_sign_and_verify_transactions() {
        let key_pair_generator = KeyPairGenerator::new();

        let prev_to_keypair = key_pair_generator.random_keypair().ok().unwrap();
        let prev_to_pub_key = prev_to_keypair.pub_key();
        let prev_to_addr = Address::from_pub_key(&prev_to_pub_key);

        let prev_output = TxOut{
            amount: 10,
            to_address: prev_to_addr,
        };

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash([0u8; 32]),
        };

        let next_to_keypair = key_pair_generator.random_keypair().ok().unwrap();
        let next_to_pub_key = next_to_keypair.pub_key();
        let next_to_addr = Address::from_pub_key(&next_to_pub_key);

        let next_output = TxOut{
            amount: 10,
            to_address: next_to_addr,
        };

        let next_tx = RawMoveTx{
            input: vec![next_input],
            output: vec![next_output],
        };

        let signed_tx = SignedMoveTx::from_raw_tx(next_tx, vec![&prev_to_keypair]).ok().unwrap();

        signed_tx.verify_signatures(&vec![prev_output]).ok().unwrap();
    }
}