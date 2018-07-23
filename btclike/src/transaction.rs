use ring::error::Unspecified;
use crypto::Hash;
use crypto::PubKey;
use crypto::Signature;
use crypto::KeyPair;
use crypto::hash;
use bincode;

enum Error{
    InvalidNumberOfKeyPairs(String),
    SerializationError(String),
    InvalidAddress,
    TxIoMismatch,
    CryptographyError,
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

#[derive(Serialize, Clone, PartialEq)]
pub struct Address(Hash);

impl Address{
    fn from_pub_key(pub_key: &PubKey) -> Address{
        Address(hash(&pub_key.as_bytes()))
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
            .map_err(|err|{
                Error::from(err)
            })
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
        let serialized = bincode::serialize(&raw_tx)?;

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

            let signed_tx_in = SignedTxIn::from_raw_tx_in(raw_tx_in, &serialized, key_pair);
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
        let serialized = bincode::serialize(&raw_next_tx)?;

        for (i, prev_tx_out) in prev_tx_outs.iter().enumerate() {
            let tx_in = &self.input[i];
            let address = Address::from_pub_key(&tx_in.sig_public_key);

            if address != prev_tx_out.to_address {
                return Err(Error::InvalidAddress);
            }

            tx_in.verify_signature(&serialized)?
        }

        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crypto::KeyPairGenerator;

    #[test]
    fn can_sign_and_verify_transactions() {
        let key_pair_generator = KeyPairGenerator::new();

        let (prev_to_keypair, prev_output) = prev_context(&key_pair_generator);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: 10,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawMoveTx{
            input: vec![next_input],
            output: vec![next_output],
        };

        let signed_tx = SignedMoveTx::from_raw_tx(next_tx,
                                                  vec![&prev_to_keypair]).ok().unwrap();

        signed_tx.verify_signatures(&vec![prev_output]).ok().unwrap();
    }

    #[test]
    fn rejects_invalid_pub_key() {
        let key_pair_generator = KeyPairGenerator::new();

        let (prev_to_keypair, prev_output) = prev_context(&key_pair_generator);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: 10,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawMoveTx{
            input: vec![next_input],
            output: vec![next_output],
        };

        let mut signed_tx = SignedMoveTx::from_raw_tx(next_tx,
                                                  vec![&prev_to_keypair]).ok().unwrap();

        let invalid_key_pair = key_pair_generator.random_keypair().ok().unwrap();
        signed_tx.input[0].sig_public_key = invalid_key_pair.pub_key();

        signed_tx.verify_signatures(&vec![prev_output]).err().unwrap();
    }

    #[test]
    fn rejects_invalid_key_pair() {
        let key_pair_generator = KeyPairGenerator::new();

        let (_prev_to_keypair, prev_output) = prev_context(&key_pair_generator);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: 10,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawMoveTx{
            input: vec![next_input],
            output: vec![next_output],
        };

        let invalid_key_pair = key_pair_generator.random_keypair().ok().unwrap();
        let signed_tx = SignedMoveTx::from_raw_tx(next_tx,
                                                  vec![&invalid_key_pair]).ok().unwrap();

        signed_tx.verify_signatures(&vec![prev_output]).err().unwrap();
    }

    fn next_address(key_pair_generator: &KeyPairGenerator) -> Address {
        let next_to_keypair = key_pair_generator.random_keypair().ok().unwrap();
        let next_to_pub_key = next_to_keypair.pub_key();
        let next_to_addr = Address::from_pub_key(&next_to_pub_key);
        next_to_addr
    }

    fn prev_context(key_pair_generator: &KeyPairGenerator) -> (KeyPair, TxOut) {
        let prev_to_keypair = key_pair_generator.random_keypair().ok().unwrap();
        let prev_to_pub_key = prev_to_keypair.pub_key();
        let prev_to_addr = Address::from_pub_key(&prev_to_pub_key);
        let prev_output = TxOut {
            amount: 10,
            to_address: prev_to_addr,
        };
        (prev_to_keypair, prev_output)
    }
}