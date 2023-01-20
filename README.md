# crossword-builder

## About

This is a CLI tool, written in Rust, designed to help construct and validate NYT-style crossword puzzles based on the guidelines
 [here](https://www.mathpuzzle.com/MAA/19-Crossword%20Rules/mathgames_05_10_04.html) and a dictionary saved in the repo.
The tool works with text files saved to the `crosswords` directory in the repo. It can create new, random puzzle grids, display puzzle files and
validate the layout and word contents of puzzles.

Run `./target/debug/crossword-builder -h` for the usage message.

## Examples

```
$./target/debug/crossword-builder puzzle-16 new 16
▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▩ ▩ ▩ ▩ ▢ ▢ ▢ ▢
▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢
▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢
▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▩ ▩
▩ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢
▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢
▩ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢
▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢
▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩
▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▩
▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩
▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢ ▩
▩ ▩ ▩ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢ ▢
▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢
▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢
▢ ▢ ▢ ▢ ▩ ▩ ▩ ▩ ▢ ▢ ▢ ▢ ▩ ▢ ▢ ▢

$ ./target/debug/crossword-builder puzzle-16 check-base
Puzzle base is valid

$ ./target/debug/crossword-builder puzzle-5 display
▩ H A T ▩
P A L E R
A L I N E
L O B O S
▩ S I R ▩

$ ./target/debug/crossword-builder puzzle-5 check-words
Loading dictionary from ./english3.txt
Puzzle words are valid
```

## Future Improvements
+ **Grid generation** - currently, the randomly generated grids aren't always valid (especially for odd-sized grids).
+ **Saving clues** - add a way to associate clues with words and display them alongside the puzzle
+ **Word suggestions** - add an option to suggest words that would in the puzzle, given constraints of length and existing letters.
