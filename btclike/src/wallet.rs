use crypto::KeyPair;
use crypto::KeyPairGenerator;
use transaction::Address;
use transaction::TxOut;
use transaction::RawTxIn;
use Error;
use transaction::RawTx;
use transaction::SignedTx;
use crypto::Hash;

/// A naive implementation of a cryptocurrency wallet.
pub struct Wallet{
    accounts: Vec<Account>,
    generator: KeyPairGenerator,
}

impl Wallet {
    pub fn new() -> Wallet{
        Wallet{
            accounts: vec![],
            generator: KeyPairGenerator::new(),
        }
    }

    pub fn new_transaction<S>(
        &mut self,
        amount: u32,
        to_address: Address,
        fees: u32,
        utxo_store: &S,
    ) -> Result<SignedTx, Error>
        where S: UtxoStore
    {
        let change_address = self.new_address()?;

        let total_cost = amount + fees;
        let mut collected_amount = 0u32;

        let mut raw_tx_ins = vec![];
        let mut key_pairs = vec![];
        {
            // PERFORMANCE We iterate over the accounts to collect the funds when
            // it would have been more efficient to track the list of funded addresses.
            let mut account_iter = self.accounts.iter();

            while collected_amount < total_cost {
                match account_iter.next() {
                    Some(account) => {
                        if let Some(utxo_reference) = utxo_store.find_for_address(&account.address) {
                            let raw_tx_in = RawTxIn{
                                prev_tx_hash: utxo_reference.tx_hash.clone(),
                                prev_tx_output_index: utxo_reference.tx_out_index,
                            };

                            raw_tx_ins.push(raw_tx_in);
                            key_pairs.push(&account.key_pair);
                            collected_amount += utxo_reference.amount;
                        }
                    },
                    None => {
                        return Err(Error::NotEnoughTokens);
                    }
                }
            }
        }

        let change = collected_amount - total_cost;
        let change_tx_out = TxOut::new(change, change_address);

        let payment_tx_out = TxOut::new(amount, to_address);

        let raw_tx = RawTx {
            input: raw_tx_ins,
            output: vec![
                change_tx_out,
                payment_tx_out,
            ],
        };

        SignedTx::from_raw_tx(raw_tx, key_pairs)
    }

    pub fn new_address(&mut self) -> Result<Address, Error> {
        let new_account = Account::new(self.generator.random_keypair()?);

        let address = new_account.address.clone();
        self.accounts.push(new_account);

        Ok(address)
    }
}

struct Account {
    key_pair: KeyPair,
    address: Address,
}

impl Account {
    fn new(key_pair: KeyPair) -> Account {
        let pub_key = key_pair.pub_key();
        let address = Address::from_pub_key(&pub_key);

        Account{
            address,
            key_pair,
        }
    }
}

pub struct TxOutReference {
    tx_hash: Hash,
    tx_out_index: u8,
    amount: u32,
}

pub trait UtxoStore {
    fn find_for_address(&self, address: &Address) -> Option<&TxOutReference>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use transaction;
    use self::map_key_pair::PairHashMap;

    /// A basic UTXO store relying on hash maps.
    struct BasicUtxoStore{
        utxos_from_address: HashMap<Address, TxOutReference>,
        utxos_from_tx_hash: PairHashMap<Hash, u8, TxOut>,
    }

    impl BasicUtxoStore {
        fn new() -> BasicUtxoStore{
            BasicUtxoStore{
                utxos_from_address: HashMap::new(),
                utxos_from_tx_hash: PairHashMap::new(),
            }
        }

        fn push(&mut self, tx_hash: Hash, tx_out: TxOut, tx_out_index: u8){
            self.utxos_from_address.insert(tx_out.to_address().clone(), TxOutReference{
                tx_out_index: 0,
                tx_hash: tx_hash.clone(),
                amount: *tx_out.amount(),
            });

            self.utxos_from_tx_hash.insert(tx_hash, tx_out_index, tx_out);
        }
    }

    impl UtxoStore for BasicUtxoStore {
        fn find_for_address(&self, address: &Address) -> Option<&TxOutReference> {
            self.utxos_from_address.get(address)
        }
    }

    impl transaction::UtxoStore for BasicUtxoStore{
        fn find(&self, transaction_hash: &Hash, txo_index: &u8) -> Option<&TxOut> {
            self.utxos_from_tx_hash.get(transaction_hash, txo_index)
        }
    }

    #[test]
    fn can_create_valid_transactions() {
        let mut wallet_a = Wallet::new();
        let mut wallet_b = Wallet::new();

        let address_a = wallet_a.new_address().unwrap();
        let address_b = wallet_b.new_address().unwrap();

        let mut utxo_store = BasicUtxoStore::new();

        let tx_out = TxOut::new(10, address_a);
        utxo_store.push(Hash::min(), tx_out, 0);

        let transaction = wallet_a.new_transaction(7, address_b, 2, &utxo_store).unwrap();
        transaction.verify(&utxo_store).unwrap();
    }

    #[test]
    fn cannot_create_transaction_if_insufficient_funds() {
        let mut wallet_a = Wallet::new();
        let mut wallet_b = Wallet::new();

        let address_b = wallet_b.new_address().unwrap();

        let utxo_store = BasicUtxoStore::new();

        wallet_a.new_transaction(7, address_b, 2, &utxo_store).err().unwrap();
    }

    mod map_key_pair {
        use std::collections::HashMap;
        use std::hash::{Hash, Hasher};
        use std::borrow::Borrow;

        #[derive(PartialEq, Eq, Hash)]
        struct Pair<A, B>(A, B);

        #[derive(PartialEq, Eq, Hash)]
        struct BorrowedPair<'a, 'b, A: 'a, B: 'b>(&'a A, &'b B);

        trait MapKeyPair<A, B> {
            /// Obtains a reference to the first element of the pair.
            fn a(&self) -> &A;
            /// Obtains a reference to the second element of the pair.
            fn b(&self) -> &B;
        }

        impl<'a, A, B> Borrow<MapKeyPair<A, B> + 'a> for Pair<A, B>
            where
                A: Eq + Hash + 'a,
                B: Eq + Hash + 'a,
        {
            fn borrow(&self) -> &(MapKeyPair<A, B> + 'a) {
                self
            }
        }

        impl<'a, A: Hash, B: Hash> Hash for (MapKeyPair<A, B> + 'a) {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.a().hash(state);
                self.b().hash(state);
            }
        }

        impl<'a, A: Eq, B: Eq> PartialEq for (MapKeyPair<A, B> + 'a) {
            fn eq(&self, other: &Self) -> bool {
                self.a() == other.a() && self.b() == other.b()
            }
        }

        impl<'a, A: Eq, B: Eq> Eq for (MapKeyPair<A, B> + 'a) {}

        /// A hash map relying on a pair of keys.
        pub struct PairHashMap<A: Eq + Hash, B: Eq + Hash, V> {
            map: HashMap<Pair<A, B>, V>,
        }

        impl<A: Eq + Hash, B: Eq + Hash, V> PairHashMap<A, B, V> {
            pub fn new() -> Self {
                PairHashMap { map: HashMap::new() }
            }

            pub fn get(&self, a: &A, b: &B) -> Option<&V> {
                self.map.get(&BorrowedPair(a, b) as &MapKeyPair<A, B>)
            }

            pub fn insert(&mut self, a: A, b: B, v: V) {
                self.map.insert(Pair(a, b), v);
            }
        }

        impl<A, B> MapKeyPair<A, B> for Pair<A, B>
            where
                A: Eq + Hash,
                B: Eq + Hash,
        {
            fn a(&self) -> &A {
                &self.0
            }
            fn b(&self) -> &B {
                &self.1
            }
        }

        impl<'a, 'b, A, B> MapKeyPair<A, B> for BorrowedPair<'a, 'b, A, B>
            where
                A: Eq + Hash + 'a,
                B: Eq + Hash + 'b,
        {
            fn a(&self) -> &A {
                self.0
            }
            fn b(&self) -> &B {
                self.1
            }
        }
    }
}