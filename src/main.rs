use clap::{Args, Parser, Subcommand};
use dictionary::DICTIONARY;
use puzzle::Puzzle;
use std::fs::{self};

mod dictionary;
mod grid;
mod puzzle;
/*

Improvements:
    + display for puzzle - print name & size
    + Prettier error handling and display
    + Better file format
*/

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
/// A command line utility to help build crossword puzzles
struct Cli {
    name: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new, blank crossword puzzle.
    New(New),
    /// Fill a puzzle with random letters.
    RandomFill,
    /// Validate the base grid of a puzzle
    CheckBase,
    /// Validate the puzzle's words
    CheckWords,
    /// Display the puzzle
    Display,

    Suggest(Suggest),
}

#[derive(Args)]
struct Suggest {
    index: usize,
    direction: String,
    #[arg(default_value_t = 5)]
    count: usize,
}

#[derive(Args)]
struct New {
    #[arg(default_value_t = 3)]
    size: usize,
}

static DICTIONARY_FILE: &str = "./english3.txt";
static PUZZLE_DIR: &str = "puzzles";
static PERCENT_BLACK: usize = 16;
static MAX_WORD_LEN: usize = 30;
fn main() {
    if let Err(e) = fs::create_dir_all(PUZZLE_DIR) {
        println!("Error creating dir {}: {}", PUZZLE_DIR, e);
        return;
    }
    let cli = Cli::parse();
    let name = cli.name;

    match &cli.command {
        Commands::New(new) => {
            if new.size % 2 != 0 {
                println!("Warning: program only generates valid puzzle bases of an even size.")
            }

            let mut puzzle = Puzzle::new(name, new.size);
            puzzle.random_black();
            //let puzzle = Puzzle::random_valid_grid(name, new.size);
            println!("{}", puzzle.cells());
            match puzzle.save_to_file() {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }
        }
        Commands::RandomFill => match Puzzle::open_from_file(name) {
            Ok(mut puzzle) => {
                puzzle.random_letters();
                println!("{}", puzzle.cells());
                match puzzle.save_to_file() {
                    Ok(_) => (),
                    Err(e) => println!("Error saving puzzle to file: {}", e),
                }
            }
            Err(e) => println!("{}", e),
        },
        Commands::CheckBase => match Puzzle::open_from_file(name) {
            Ok(puzzle) => match puzzle.validate_base() {
                Ok(_) => println!("Puzzle base is valid"),
                Err(e) => println!("Puzzle base is invalid: {}", e),
            },
            Err(e) => println!("{}", e),
        },
        Commands::CheckWords => match Puzzle::open_from_file(name) {
            Ok(puzzle) => match puzzle.validate_words() {
                Ok(_) => println!("Puzzle words are valid"),
                Err(e) => println!("Puzzle words are invalid: {}", e),
            },
            Err(e) => println!("{}", e),
        },
        Commands::Display => match Puzzle::open_from_file(name) {
            Ok(puzzle) => println!("{}", puzzle.cells()),
            Err(e) => println!("{}", e),
        },
        Commands::Suggest(suggest) => match Puzzle::open_from_file(name) {
            Ok(puzzle) => {
                let partial_word = match suggest.direction.as_str() {
                    "across" => puzzle.get_across_word(suggest.index),
                    "down" => puzzle.get_down_word(suggest.index),
                    x => {
                        println!("Expected across or down, got {}", x);
                        return;
                    }
                };
                match partial_word {
                    Some(word) => {
                        let suggestions = DICTIONARY.suggest_words(word, suggest.count);
                        println!("{:?}", suggestions)
                    }
                    None => println!(
                        "There is no {} word at index {}",
                        suggest.direction, suggest.index
                    ),
                }
            }
            Err(e) => println!("{}", e),
        },
    }
}
