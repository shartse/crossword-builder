use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashSet,
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

pub struct Dictionary(Vec<HashSet<String>>);
impl Dictionary {
    fn new(size: usize) -> Self {
        let mut dictionary: Vec<HashSet<String>> = Vec::new();
        for _ in 0..size {
            dictionary.push(HashSet::new());
        }
        Dictionary(dictionary)
    }

    fn insert(&mut self, word: String) -> bool {
        if let Some(map) = self.get_mut(word.len()) {
            return map.insert(word);
        }
        false
    }

    fn get(&self, index: usize) -> Option<&HashSet<String>> {
        self.0.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut HashSet<String>> {
        self.0.get_mut(index)
    }

    pub fn is_valid(&self, word: &str) -> bool {
        if let Some(map) = self.get(word.len()) {
            return map.get(word).is_some();
        }
        false
    }

    pub fn suggest_words(&self, partial_word: SparseWord, count: usize) -> Vec<String> {
        let mut suggestions = Vec::new();
        let correct_len = self.get(partial_word.len());
        if let Some(words) = correct_len {
            for word in words {
                if partial_word.matches(word) {
                    suggestions.push(word.clone())
                }
                if suggestions.len() >= count {
                    return suggestions;
                }
            }
        }
        suggestions
    }
}

#[derive(Debug)]
pub struct SparseWord {
    regex: Regex,
    len: usize,
}
impl SparseWord {
    pub fn new(vec: Vec<Option<char>>) -> Self {
        let len = vec.len();
        // Build a case-insensitive regex of the form "..a..cd.."
        let regex = Regex::new(&vec.iter().fold("(?i)".to_string(), |acc, arg| {
            format!("{}{}", acc, arg.map_or('.', |x| x))
        }))
        .expect("Unable to build regex");
        SparseWord { regex, len }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn matches(&self, word: &str) -> bool {
        self.regex.is_match(word)
    }
}

impl PartialEq for SparseWord {
    fn eq(&self, other: &Self) -> bool {
        self.regex.to_string() == other.regex.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::dictionary::SparseWord;

    use super::DICTIONARY;

    #[test]
    fn suggest_one() {
        let suggestions =
            DICTIONARY.suggest_words(SparseWord::new(vec![Some('A'), None, Some('T')]), 1);
        assert_eq!(suggestions.len(), 1);
        let suggestions =
            DICTIONARY.suggest_words(SparseWord::new(vec![Some('A'), Some('C'), Some('T')]), 1);
        assert_eq!(suggestions, vec!["act"]);
    }

    #[test]
    fn suggest_ten() {
        let suggestions = DICTIONARY.suggest_words(
            SparseWord::new(vec![Some('A'), None, None, None, Some('T')]),
            10,
        );
        assert_eq!(suggestions.len(), 10);
    }

    #[test]
    fn suggest_impossible() {
        let suggestions = DICTIONARY.suggest_words(
            SparseWord::new(vec![Some('A'), Some('X'), Some('Z'), None, Some('T')]),
            10,
        );
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn suggest_z_words() {
        let mut suggestions = DICTIONARY.suggest_words(
            SparseWord::new(vec![
                Some('Z'),
                None,
                None,
                None,
                Some('T'),
                None,
                None,
                Some('E'),
            ]),
            10,
        );
        suggestions.sort();
        assert_eq!(suggestions, vec!["zaratite"]);

        let mut suggestions = DICTIONARY.suggest_words(
            SparseWord::new(vec![Some('Z'), None, None, None, Some('Y')]),
            10,
        );
        suggestions.sort();
        assert_eq!(
            suggestions,
            vec!["zappy", "zesty", "zincy", "zingy", "zinky", "zippy", "zloty"]
        );
    }
}
