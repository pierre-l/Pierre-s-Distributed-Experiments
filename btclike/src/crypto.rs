use ring::{rand, signature};
use ring::signature::Ed25519KeyPair;
use ring::rand::SystemRandom;
use ring::digest;
use ring::digest::SHA256;
use ring::error::Unspecified;
use ring::signature::ED25519;
use untrusted::{self, Input};

pub struct KeyPairGenerator{
    rng: SystemRandom,
}

impl KeyPairGenerator{
    pub fn new() -> KeyPairGenerator {
        KeyPairGenerator{
            rng: rand::SystemRandom::new(),
        }
    }

    pub fn random_keypair(&self) -> Result<KeyPair, Unspecified>{
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&self.rng)?;

        let key_pair =
            Ed25519KeyPair::from_pkcs8(Input::from(&pkcs8_bytes))?;

        Ok(KeyPair(key_pair))
    }
}

const PUBKEY_LEN: usize = 32;
#[derive(Clone)]
pub struct PubKey([u8; PUBKEY_LEN]);

impl PubKey{
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn verify_signature(&self, input_bytes: &[u8], signature: &Signature) -> Result<(), Unspecified> {
        let peer_public_key = untrusted::Input::from(&self.0);
        let sig = untrusted::Input::from(&signature.0);
        let msg = untrusted::Input::from(input_bytes);

        signature::verify(&ED25519, peer_public_key, msg, sig)
    }
}

const SIGNATURE_LEN: usize = 64;
#[derive(Clone)]
pub struct Signature([u8; SIGNATURE_LEN]);

const HASH_LEN: usize = 32;
#[derive(Serialize, Clone, Eq, PartialEq)]
pub struct Hash([u8; HASH_LEN]);

impl Hash {
    pub fn min() -> Hash{
        Hash([0u8; 32])
    }
}

pub struct KeyPair(Ed25519KeyPair);

impl KeyPair{
    pub fn pub_key(&self) -> PubKey {
        // PERFORMANCE Not optimal: could get rid of the copy operation.
        let mut bytes = [0u8; PUBKEY_LEN];
        bytes[..PUBKEY_LEN].clone_from_slice(self.0.public_key_bytes());
        PubKey(bytes)
    }

    pub fn sign(&self, input_bytes: &[u8]) -> Signature {
        // PERFORMANCE Not optimal: could get rid of the copy operation.
        let raw_signature = self.0.sign(input_bytes);

        let mut signature_bytes = [0u8; SIGNATURE_LEN];
        signature_bytes[..SIGNATURE_LEN].clone_from_slice(raw_signature.as_ref());

        Signature(signature_bytes)
    }
}

pub fn hash(input_bytes: &[u8]) -> Hash{
    // PERFORMANCE Not optimal: could get rid of the copy operation.
    let digest = digest::digest(&SHA256, &input_bytes);

    let mut hash_bytes = [0u8; HASH_LEN];
    hash_bytes[..HASH_LEN].clone_from_slice(digest.as_ref());

    Hash(hash_bytes)
}