use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
};

use crate::{DICTIONARY_FILE, MAX_WORD_LEN};

lazy_static! {
    pub static ref DICTIONARY: Dictionary = {
        println!("Loading dictionary from {}", DICTIONARY_FILE);
        let mut dictionary = Dictionary::new(MAX_WORD_LEN);
        let file = File::open(DICTIONARY_FILE);
        if let Ok(file) = file {
            let lines = io::BufReader::new(file).lines();
            for line in lines {
                if let Ok(word) = line {
                    dictionary.insert(word);
                }
            }
        }
        dictionary
    };
}

pub struct Dictionary(Vec<HashMap<String, usize>>);
impl Dictionary {
    fn new(size: usize) -> Self {
        let mut dictionary: Vec<HashMap<String, usize>> = Vec::new();
        for _ in 0..size {
            dictionary.push(HashMap::new());
        }
        Dictionary(dictionary)
    }

    fn insert(&mut self, word: String) -> Option<usize> {
        if let Some(map) = self.get_mut(word.len()) {
            return map.insert(word, 1);
        }
        None
    }

    fn get(&self, index: usize) -> Option<&HashMap<String, usize>> {
        self.0.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut HashMap<String, usize>> {
        self.0.get_mut(index)
    }

    pub fn is_valid(&self, word: &str) -> bool {
        if let Some(map) = self.get(word.len()) {
            return map.get(word).is_some();
        }
        false
    }
}
