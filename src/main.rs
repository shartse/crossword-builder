use clap::{Args, Parser, Subcommand};
use puzzle::Puzzle;
use std::fs::{self};

mod dictionary;
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
    }
}
