#[macro_use] extern crate log;
extern crate env_logger;
extern crate ring;
extern crate untrusted;

use log::LevelFilter;
use ring::{rand, signature};
use untrusted::Input;
use ring::signature::Ed25519KeyPair;
use ring::rand::SystemRandom;

fn main() {
    // Always print backtrace on panic.
    ::std::env::set_var("RUST_BACKTRACE", "1");

    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .filter_level(LevelFilter::Info)
        .init();

    info!("Hello world.");
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

    fn random_keypair(&self) -> KeyPair{
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&self.rng).unwrap();

        let key_pair =
            Ed25519KeyPair::from_pkcs8(Input::from(&pkcs8_bytes)).unwrap();

        KeyPair(key_pair)
    }
}

const PUBKEY_LEN: usize = 32;

struct PubKey([u8; PUBKEY_LEN]);

const SIGNATURE_LEN: usize = 64;

struct Signature([u8; SIGNATURE_LEN]);

struct KeyPair(Ed25519KeyPair);

impl KeyPair{
    fn pub_key(&self) -> PubKey {
        // PERFORMANCE Not as efficient as it could be.
        let mut bytes = [0u8; PUBKEY_LEN];
        bytes[..32].clone_from_slice(self.0.public_key_bytes());
        PubKey(bytes)
    }

    fn sign(&self, input_bytes: &[u8]) -> Signature {
        // PERFORMANCE Not as efficient as it could be.
        let raw_signature = self.0.sign(input_bytes);

        let mut signature_bytes = [0u8; SIGNATURE_LEN];
        signature_bytes[..64].clone_from_slice(raw_signature.as_ref());

        Signature(signature_bytes)
    }
}

impl PubKey{
    fn verify_signature(&self, input_bytes: &[u8], signature: &Signature) -> Result<(), ()> {
        let peer_public_key = untrusted::Input::from(&self.0);
        let sig = untrusted::Input::from(&signature.0);
        let msg = untrusted::Input::from(input_bytes);

        signature::verify(&ring::signature::ED25519, peer_public_key, msg, sig)
            .map_err(|_err| { () })
    }
}