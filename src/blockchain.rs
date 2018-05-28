use ring::digest::{self, Digest, SHA256, SHA256_OUTPUT_LEN};
use std::cmp::Ordering;
use std::u8::MAX as U8_MAX;

struct Difficulty([u8; SHA256_OUTPUT_LEN]);

impl Difficulty{
    pub fn min_difficulty() -> Difficulty{
        let mut array = [0 as u8; SHA256_OUTPUT_LEN];
        array[0] = U8_MAX;
        Difficulty(array)
    }

    pub fn increase(&mut self) {
        self.divide_inner_by_two()
    }

    fn divide_inner_by_two(&mut self){
        let mut index_to_split = 0;

        while self.0[index_to_split] == 0 {
            index_to_split += 1;
        }
        self.0[index_to_split] = self.0[index_to_split]/2;

        if self.0[index_to_split] == 0 {
            let next_index = index_to_split + 1;

            self.0[next_index] = U8_MAX/2;
        }
    }
}

#[derive(Clone, Debug)]
struct Hash{
    digest: Digest,
}

impl Hash{
    pub fn new(node_id: u8, nonce: &Nonce) -> Hash{
        let mut data_to_hash = [0u8; 9];

        for i in 0..8{
            data_to_hash[i] = nonce.0[i];
        }

        data_to_hash[8] = node_id;

        let digest = digest::digest(&SHA256, &data_to_hash);

        Hash{
            digest,
        }
    }

    pub fn less_than(&self, difficulty: &Difficulty) -> bool {
        let hash = self.digest.as_ref();

        // Can't use `cmp` between these because the digest's [u8] length.
        less_than_u8(hash, &difficulty.0)
    }
}

fn less_than_u8(one: &[u8], other: &[u8]) -> bool{
    // Still, we assume that `one` and `other` have the same length.
    let len = one.len();
    let mut i = 0;
    let mut temp_result = Ordering::Equal;

    while i<len && temp_result==Ordering::Equal {
        temp_result = one[i].cmp(&other[i]);
        i = i+1;
    }

    temp_result == Ordering::Less
}

struct Nonce([u8; 8]);

impl Nonce{
    pub fn new() -> Nonce {
        Nonce([0u8; 8])
    }

    pub fn increment(&mut self) {
        let mut index_to_increment = self.0.len() -1;

        while self.0[index_to_increment] == U8_MAX {
            self.0[index_to_increment] = 0;
            index_to_increment -= 1;
        }
        self.0[index_to_increment] = self.0[index_to_increment] +1;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_difficulty_allows_any_hash() {
        let difficulty = Difficulty::min_difficulty();

        let mut nonce = Nonce::new();
        for _i in 0..100 {
            nonce.increment();
            let hash = Hash::new(1, &nonce);
            assert_eq!(true, hash.less_than(&difficulty));
        }
    }

    #[test]
    fn can_increase_difficulty() {
        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();
        difficulty.increase();
        difficulty.increase();

        let number_of_tries = 100000;
        let mut number_of_valid_hashes = 0;
        let mut nonce = Nonce::new();
        for _i in 0..number_of_tries {
            nonce.increment();
            let hash = Hash::new(1, &nonce);

            if hash.less_than(&difficulty) {
                number_of_valid_hashes += 1;
            }
        }

        assert!(number_of_valid_hashes < number_of_tries/7);
        assert!(number_of_valid_hashes > number_of_tries/9);
    }
}