use ring::digest::{self, Digest, SHA256, SHA256_OUTPUT_LEN};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fmt::Error;
use std::fmt::Formatter;
use std::u8::MAX as U8_MAX;

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
}

impl Debug for Difficulty {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        print_u8_as_hexa(&self.threshold, f)
    }
}

#[derive(Clone)]
pub struct Hash {
    digest: Digest,
}

impl Hash {
    pub fn new(
        node_id: u32,
        nonce: &Nonce,
        difficulty: &Difficulty,
        height: u32,
        previous_hash: &[u8],
    ) -> Hash {
        let difficulty_bytes = difficulty.threshold.as_ref();
        let mut data_to_hash = [0u8; 8 // Length of the nonce field.
            + 4 // Length of the node_id field.
            + 4 // Length of the height field.
            + SHA256_OUTPUT_LEN // Length of the hash.
            + DIFFICULTY_BYTES_LEN];

        data_to_hash[..8].clone_from_slice(&nonce.0[..8]);

        write_array(&mut data_to_hash, &nonce.0, 0);
        write_u32(&mut data_to_hash, node_id, 8);
        write_u32(&mut data_to_hash, height, 12);
        write_array(&mut data_to_hash, &previous_hash, 16);
        write_array(&mut data_to_hash, &difficulty_bytes, 16 + SHA256_OUTPUT_LEN);

        let digest = digest::digest(&SHA256, &data_to_hash);

        Hash { digest }
    }

    pub fn less_than(&self, difficulty: &Difficulty) -> bool {
        let hash_bytes = self.bytes();
        let difficulty_bytes = &difficulty.threshold;

        debug!("Candidate:  {:?}", hash_bytes);
        debug!("Difficulty: {:?}", difficulty_bytes);

        // Can't use `cmp` between these because the digest's [u8] length.
        less_than_u8(hash_bytes, difficulty_bytes)
    }

    pub fn bytes(&self) -> &[u8] {
        self.digest.as_ref()
    }
}

fn write_u32(to_array: &mut [u8], number: u32, index: usize) {
    to_array[index] = ((number >> 24) & 0xff) as u8;
    to_array[index + 1] = ((number >> 16) & 0xff) as u8;
    to_array[index + 2] = ((number >> 8) & 0xff) as u8;
    to_array[index + 3] = (number & 0xff) as u8;
}

fn write_array(to_array: &mut [u8], array: &[u8], index: usize) {
    let array_len = array.len();
    to_array[index..(array_len + index)].clone_from_slice(&array[..array_len])
}

impl Debug for Hash {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        print_u8_as_hexa(&self.bytes(), f)
    }
}

impl PartialEq for Hash {
    fn eq(&self, other: &Hash) -> bool {
        self.digest.as_ref().eq(other.digest.as_ref())
    }
}

fn less_than_u8(one: &[u8], other: &[u8]) -> bool {
    // Still, we assume that `one` and `other` have the same length.
    let len = one.len();
    let mut i = 0;
    let mut temp_result = Ordering::Equal;

    while i < len && temp_result == Ordering::Equal {
        temp_result = one[i].cmp(&other[i]);
        i += 1;
    }

    temp_result == Ordering::Less
}

#[derive(Clone, Debug)]
pub struct Nonce([u8; 8]);

impl Nonce {
    pub fn new() -> Nonce {
        Nonce([0u8; 8])
    }

    pub fn increment(&mut self) {
        let mut index_to_increment = self.0.len() - 1;

        while self.0[index_to_increment] == U8_MAX {
            self.0[index_to_increment] = 0;
            index_to_increment -= 1;
        }
        self.0[index_to_increment] += 1;
    }
}

fn print_u8_as_hexa(bytes: &[u8], f: &mut Formatter) -> Result<(), Error> {
    let mut concatenated = String::new();
    for byte in bytes {
        let hex_byte = format!("{:x}", byte);

        if hex_byte.len() == 1 {
            concatenated += "0";
        }

        concatenated += &hex_byte;
    }
    write!(f, "{}", &concatenated)?;
    Ok(())
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
            let hash = Hash::new(1, &nonce, &difficulty, 1, &[0u8; SHA256_OUTPUT_LEN]);
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
            let hash = Hash::new(1, &nonce, &difficulty, 1, &[0u8; SHA256_OUTPUT_LEN]);

            if hash.less_than(&difficulty) {
                number_of_valid_hashes += 1;
            }
        }

        assert!(number_of_valid_hashes < number_of_tries / 7);
        assert!(number_of_valid_hashes > number_of_tries / 9);
    }
}
