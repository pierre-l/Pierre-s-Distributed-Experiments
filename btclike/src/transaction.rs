use crypto::Hash;
use crypto::PubKey;
use crypto::Signature;
use crypto::KeyPair;
use crypto::hash;
use bincode;
use Error;

#[derive(Serialize, Clone, PartialEq, Eq, Hash)]
pub struct Address(Hash);

impl Address{
    pub fn from_pub_key(pub_key: &PubKey) -> Address{
        Address(hash(&pub_key.as_bytes()))
    }
}

#[derive(Serialize, Clone)]
pub struct RawTxIn{
    pub prev_tx_hash: Hash,
    pub prev_tx_output_index: u8,
}

#[derive(Serialize, Clone)]
pub struct TxOut{
    amount: u32,
    to_address: Address,
}

impl TxOut {
    pub fn new(
        amount: u32,
        to_address: Address,
    ) -> TxOut {
        TxOut{
            amount,
            to_address,
        }
    }

    pub fn amount(&self) -> &u32 {
        &self.amount
    }

    pub fn to_address(&self) -> &Address{
        &self.to_address
    }
}

#[derive(Serialize, Clone)]
pub struct RawTx {
    pub input: Vec<RawTxIn>,
    pub output: Vec<TxOut>,
}

#[derive(Serialize, Clone)]
pub struct SignedTxIn{
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

#[derive(Serialize, Clone)]
pub struct SignedTx {
    input: Vec<SignedTxIn>,
    output: Vec<TxOut>,
}

impl SignedTx {
    pub fn from_raw_tx(raw_tx: RawTx, key_pairs: Vec<&KeyPair>)
                   -> Result<SignedTx, Error>
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

        Ok(SignedTx {
            input: signed_input,
            output,
        })
    }

    fn clone_without_signatures(&self) -> RawTx {
        let output = self.output.clone();
        let mut input = vec![];

        for signed_input in &self.input {
            input.push(signed_input.clone_without_signature())
        }

        RawTx {
            input,
            output,
        }
    }

    pub fn verify<S>(&self, utxo_store: &S) -> Result<u32, Error>
    where
        S: UtxoStore,
    {
        let mut prev_tx_outs = vec![];

        for tx_in in &self.input {
            if let Some(prev_tx_out) = utxo_store.find(
                &tx_in.prev_tx_hash,
                &tx_in.prev_tx_output_index
            ) {
                prev_tx_outs.push(prev_tx_out);
            } else {
                return Err(Error::UtxoNotFound);
            }
        }

        let mut in_amount = 0;
        prev_tx_outs.iter().for_each(|tx_out|{
            in_amount += tx_out.amount;
        });

        let mut out_amount = 0;
        self.output.iter().for_each(|tx_out|{
            out_amount += tx_out.amount;
        });

        if in_amount < out_amount {
            return Err(Error::InvalidTxAmount);
        }

        let fees = in_amount - out_amount;

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

        Ok(fees)
    }
}

pub trait UtxoStore {
    fn find(&self, transaction_hash: &Hash, txo_index: &u8) -> Option<&TxOut>;
}

#[derive(Serialize, Clone)]
pub struct CoinbaseTx(pub TxOut);

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::KeyPairGenerator;

    #[test]
    fn can_sign_and_verify_transactions() {
        let key_pair_generator = KeyPairGenerator::new();

        let initial_amount = 10u32;
        let (prev_to_keypair, prev_output) = prev_context(&key_pair_generator, initial_amount);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: initial_amount,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawTx {
            input: vec![next_input],
            output: vec![next_output],
        };

        let signed_tx = SignedTx::from_raw_tx(next_tx,
                                              vec![&prev_to_keypair]).ok().unwrap();

        verify(signed_tx, prev_output).ok().unwrap();
    }

    #[test]
    fn rejects_invalid_amount() {
        let key_pair_generator = KeyPairGenerator::new();

        let initial_amount = 10u32;
        let (prev_to_keypair, prev_output) = prev_context(&key_pair_generator, initial_amount);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: initial_amount + 1,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawTx {
            input: vec![next_input],
            output: vec![next_output],
        };

        let signed_tx = SignedTx::from_raw_tx(next_tx,
                                              vec![&prev_to_keypair]).ok().unwrap();

        verify(signed_tx, prev_output).err().unwrap();
    }

    #[test]
    fn rejects_invalid_pub_key() {
        let key_pair_generator = KeyPairGenerator::new();

        let initial_amount = 10u32;
        let (prev_to_keypair, prev_output) = prev_context(&key_pair_generator, initial_amount);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: 10,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawTx {
            input: vec![next_input],
            output: vec![next_output],
        };

        let mut signed_tx = SignedTx::from_raw_tx(next_tx,
                                                  vec![&prev_to_keypair]).ok().unwrap();

        let invalid_key_pair = key_pair_generator.random_keypair().ok().unwrap();
        signed_tx.input[0].sig_public_key = invalid_key_pair.pub_key();

        verify(signed_tx, prev_output).err().unwrap();
    }

    #[test]
    fn rejects_invalid_key_pair() {
        let key_pair_generator = KeyPairGenerator::new();

        let initial_amount = 10u32;
        let (_prev_to_keypair, prev_output) = prev_context(&key_pair_generator, initial_amount);

        let next_input = RawTxIn{
            prev_tx_output_index: 0,
            prev_tx_hash: Hash::min(),
        };

        let next_output = TxOut{
            amount: 10,
            to_address: next_address(&key_pair_generator),
        };

        let next_tx = RawTx {
            input: vec![next_input],
            output: vec![next_output],
        };

        let invalid_key_pair = key_pair_generator.random_keypair().ok().unwrap();
        let signed_tx = SignedTx::from_raw_tx(next_tx,
                                              vec![&invalid_key_pair]).ok().unwrap();

        verify(signed_tx, prev_output).err().unwrap();
    }

    fn next_address(key_pair_generator: &KeyPairGenerator) -> Address {
        let next_to_keypair = key_pair_generator.random_keypair().ok().unwrap();
        let next_to_pub_key = next_to_keypair.pub_key();
        let next_to_addr = Address::from_pub_key(&next_to_pub_key);
        next_to_addr
    }

    fn prev_context(key_pair_generator: &KeyPairGenerator, amount: u32) -> (KeyPair, TxOut) {
        let prev_to_keypair = key_pair_generator.random_keypair().ok().unwrap();
        let prev_to_pub_key = prev_to_keypair.pub_key();
        let prev_to_addr = Address::from_pub_key(&prev_to_pub_key);
        let prev_output = TxOut {
            amount,
            to_address: prev_to_addr,
        };
        (prev_to_keypair, prev_output)
    }

    struct SingleEntryUtxoStore(TxOut);

    impl UtxoStore for SingleEntryUtxoStore{
        fn find(&self, _transaction_hash: &Hash, _txo_index: &u8) -> Option<&TxOut> {
            Some(&self.0)
        }
    }

    fn verify(transaction: SignedTx, utxo: TxOut) -> Result<u32, Error> {
        transaction.verify(&SingleEntryUtxoStore(utxo))?;
        Ok(0)
    }
}