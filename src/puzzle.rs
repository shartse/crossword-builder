use dictionary::DICTIONARY;
use rand::Rng;
use std::{
    cmp::max,
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::{Read, Write},
};
use thiserror::Error;

use crate::{
    dictionary::{self, SparseWord},
    grid::{Cell, Grid, GridError},
    PERCENT_BLACK, PUZZLE_DIR,
};

/// The rules for American crosswords are as follows:
///
/// 1. The pattern of black-and-white squares must be symmetrical.  Generally this rule means that if you turn the grid
///    upside-down, the pattern will look the same as it does right-side-up.
/// 2. Do not use too many black squares.  In the old days of puzzles, black squares were not allowed to occupy more than
///    16% of a grid.  Nowadays there is no strict limit, in order to allow maximum flexibility for the placement of theme entries.  Still,
///    "cheater" black squares (ones that do not affect the number of words in the puzzle, but are added to make constructing easier) should
///    be kept to a minimum, and large clumps of black squares anywhere in a grid are strongly discouraged.
/// 3. Do not use unkeyed letters (letters that appear in only one word across or down).  In fairness to solvers,
///    every letter has to be appear in both an Across and a Down word.
/// 4. Do not use two-letter words.  The minimum word length is three letters.
/// 5. The grid must have all-over interlock.  In other words, the black squares may not cut the grid up into separate
///    pieces.  A solver, theoretically, should be able to able to proceed from any section of the grid to any other
///    without having to stop and start over.
/// 6. Long theme entries must be symmetrically placed.  If there is a major theme entry three rows down from the top of
///    the grid, for instance, then there must be another theme entry in the same position three rows up from the bottom.
///    Also, as a general rule, no nontheme entry should be longer than any theme entry.
/// 7. Do not repeat words in the grid.
/// 8. Do not make up words and phrases.  Every answer must have a reference or else be in common use in everyday speech or
///    writing.
/// 9. (Modern rule) The vocabulary in a crossword must be lively and have very little obscurity.
#[derive(Error, Debug, PartialEq)]
pub enum PuzzleError {
    #[error("The black squares are not placed symmetrically")]
    NotSymmetric,
    #[error("More than {0} percent of the puzzle squares are black")]
    TooManyBlackSquares(usize),
    #[error("The word \"{0}\" is shorter than 3 letters")]
    WordTooShort(String),
    #[error("The word \"{0}\" is repeated")]
    RepeatWord(String),
    #[error("\"{0}\" are not in the dictionary")]
    MadeUpWord(String),
    #[error("Unable create the file \'{0}\'")]
    FileCreationError(String),
    #[error("Unable open the file \'{0}\'")]
    FileOpenError(String),
    #[error("Unable to parse this puzzle due to: \"{0}\"")]
    ParseError(GridError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Puzzle {
    name: String,
    size: usize,
    cells: Grid,
    transpose: Grid,
}

impl Puzzle {
    pub fn new(name: String, size: usize) -> Self {
        let cells = Grid::new(size);
        let transpose = cells.transpose();
        Puzzle {
            name,
            size,
            cells,
            transpose,
        }
    }

    pub fn save_to_file(&self) -> Result<(), PuzzleError> {
        let path = format!("{}/{}.txt", PUZZLE_DIR, self.name);
        let mut f =
            File::create(path.clone()).map_err(|_e| PuzzleError::FileCreationError(path))?;
        let puzzle = format!("{}", self.cells());
        f.write_all(puzzle.as_bytes()).unwrap();
        Ok(())
    }

    pub fn open_from_file(name: String) -> Result<Self, PuzzleError> {
        let path = format!("{}/{}.txt", PUZZLE_DIR, name);
        let mut f = File::open(path.clone()).map_err(|_e| PuzzleError::FileOpenError(path))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).unwrap();

        let cells = Grid::from_bytes(&buffer).map_err(|e| PuzzleError::ParseError(e))?;
        Ok(Puzzle::from_grid(name, cells))
    }

    fn from_grid(name: String, cells: Grid) -> Self {
        let size = cells.len();
        let transpose = cells.transpose();
        Puzzle {
            name,
            size,
            cells,
            transpose,
        }
    }

    pub fn cells(&self) -> &Grid {
        &self.cells
    }

    /// Get the down word that starts at index, where cells are numbered left to right, 0 to (size*size - 1), starting in the top left
    pub fn get_down_word(&self, index: usize) -> Option<SparseWord> {
        let row_num = index / self.size;
        let col_num = index % self.size;
        let col = self.transpose.get_row(col_num);
        Puzzle::take_word(col, row_num)
    }

    /// Get the across word that starts at index, where cells are numbered left to right, 0 to (size*size - 1), starting in the top left
    pub fn get_across_word(&self, index: usize) -> Option<SparseWord> {
        let row_num = index / self.size;
        let col_num = index % self.size;
        let row = self.cells.get_row(row_num);
        Puzzle::take_word(row, col_num)
    }

    fn take_word(cells: &Vec<Cell>, start: usize) -> Option<SparseWord> {
        let mut idx = start;
        let mut chars: Vec<Option<char>> = Vec::new();
        loop {
            match cells.get(idx) {
                Some(cell) => match cell {
                    Cell::Black => break,
                    Cell::Empty => chars.push(None),
                    Cell::Letter(l) => chars.push(Some(*l)),
                },
                None => break,
            }
            idx += 1;
        }
        if chars.len() > 0 {
            Some(SparseWord::new(chars))
        } else {
            None
        }
    }

    /// iterate through each row, separating by black cells
    fn words_across_iter(&self) -> impl Iterator<Item = &[Cell]> {
        self.cells.rows_iter().flat_map(|row| {
            row.split(|cell| matches!(cell, Cell::Black))
                .filter(|x| !x.is_empty())
        })
    }

    /// iterate through each col, separating by black cells
    fn words_down_iter(&self) -> impl Iterator<Item = &[Cell]> {
        self.transpose.rows_iter().flat_map(|row| {
            row.split(|cell| matches!(cell, Cell::Black))
                .filter(|x| !x.is_empty())
        })
    }

    fn all_words_iter(&self) -> impl Iterator<Item = &[Cell]> {
        self.words_across_iter().chain(self.words_down_iter())
    }

    /// Validate that the puzzle "base" (the grid, with black cells but without letters) is valid according to the spec:
    /// 1. The grid is square
    /// 2. The positions of the blacks squares are rotationally symmetric
    /// 3. That the black squares don't represent too high a proportion of the total grid.
    /// 4. All words are 3 characters or longer
    pub fn validate_base(&self) -> Result<(), PuzzleError> {
        self.cells.is_square()?;
        self.cells.is_symmetric()?;
        self.cells.acceptable_black_square_count()?;
        self.no_too_short_words()?;
        Ok(())
    }

    /// Validate that the words in the puzzle meet the spec:
    /// 1. Not repeat workds
    /// 2. All words are 3 characters or longer
    /// 3. All words appear in the dictionary we're using
    pub fn validate_words(&self) -> Result<(), PuzzleError> {
        self.no_repeat_words()?;
        self.no_too_short_words()?;
        self.valid_words()?;
        Ok(())
    }

    fn no_repeat_words(&self) -> Result<(), PuzzleError> {
        let mut words = HashMap::new();
        for word in self.all_words_iter().map(|x| Cell::as_string(x)) {
            if word.len() > 0 {
                match words.insert(word.clone(), 1) {
                    Some(_) => return Err(PuzzleError::RepeatWord(word)),
                    None => (),
                }
            }
        }
        Ok(())
    }

    fn no_too_short_words(&self) -> Result<(), PuzzleError> {
        for word in self.all_words_iter().map(|x| Cell::as_string(x)) {
            if word.len() < 3 {
                return Err(PuzzleError::WordTooShort(word));
            }
        }
        Ok(())
    }

    fn valid_words(&self) -> Result<(), PuzzleError> {
        let mut invalid_words = Vec::new();
        for word in self.all_words_iter().map(|x| Cell::as_string(x)) {
            if !DICTIONARY.is_valid(&word.to_ascii_lowercase()) {
                invalid_words.push(word);
            }
        }
        if invalid_words.is_empty() {
            Ok(())
        } else {
            return Err(PuzzleError::MadeUpWord(invalid_words.join(", ")));
        }
    }

    fn valid_black_placement(&self, (x, y): (usize, usize)) -> bool {
        // Capture the slices of the puzzle right, left, above and below the suggested black-placement and validate that it would leave
        // enough space in each direction
        let mut row: Vec<Cell> = self.cells.get_row(y).clone();
        let mut col: Vec<Cell> = self.transpose.get_row(x).clone();
        let (left, mut right) = row.split_at_mut(x);
        let (up, mut down) = col.split_at_mut(y);

        // Truncate right and down since `split_at_mut` is inclusive.
        if right.len() > 0 {
            right = &mut right[1..];
        }
        if down.len() > 0 {
            down = &mut down[1..];
        }

        // Reverse left and up so we just have to look left to right each slice.
        left.reverse();
        up.reverse();

        Grid::ok_dist_to_black_or_edge(left)
            && Grid::ok_dist_to_black_or_edge(right)
            && Grid::ok_dist_to_black_or_edge(up)
            && Grid::ok_dist_to_black_or_edge(down)
    }

    /// Generate a random configuration of black squares to form a symmetric puzzle
    pub fn random_black(&mut self) {
        // It's not possible to have valid black squares for puzzles 4 and smaller, since all words must be at least 3 letters
        // and the puzzle must be symmetric
        if self.size < 5 {
            return;
        }
        let quadrant = max(2, self.size / 2);
        let mut rng = rand::thread_rng();
        let upper_threshold_black = (self.size * self.size * PERCENT_BLACK) / 100;
        let mut black_set = 0;

        loop {
            for row in 0..quadrant {
                for col in 0..quadrant {
                    let cell = self.get(col, row);
                    if !matches!(cell, Cell::Black) {
                        if self.valid_black_placement((col, row)) {
                            // A random chance of setting the cell to black
                            let x = rng.gen_bool(1.0 / 2.0);
                            if x {
                                self.set_symmetric((col, row), Cell::Black);
                                black_set += 1;
                                if black_set >= upper_threshold_black / 4 {
                                    return;
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn set_symmetric(&mut self, (x, y): (usize, usize), val: Cell) {
        self.set(x, y, val.clone());
        self.set(self.size - (y + 1), x, val.clone());
        self.set(self.size - (x + 1), self.size - (y + 1), val.clone());
        self.set(y, self.size - (x + 1), val);
    }

    /// Trying to generate a random, valid puzzle with this takes too long for anything larger than
    /// a 3x3 puzzle. Instead, can I organize the words in such a way that I can pick words by length
    /// and verify that a substring could fit with existing letters?
    pub fn random_letters(&mut self) {
        let mut rng = rand::thread_rng();
        for row in 0..self.size {
            for col in 0..self.size {
                let cell = self.get_mut(col, row);
                if let Cell::Empty = cell {
                    let x: char = rng.gen_range(b'A'..b'Z' + 1) as char;
                    self.set(col, row, Cell::Letter(x));
                }
            }
        }
    }

    fn set(&mut self, x: usize, y: usize, value: Cell) {
        self.cells.set(x, y, value.clone());
        self.transpose.set(y, x, value);
    }

    #[allow(dead_code)]
    fn get(&self, x: usize, y: usize) -> &Cell {
        self.cells.get(x, y)
    }

    fn get_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        self.cells.get_mut(x, y)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dictionary::SparseWord,
        puzzle::{Cell, Grid, PuzzleError},
        Puzzle,
    };

    #[test]
    fn valid_empty_grid() {
        let empty = Puzzle::new("x".to_string(), 10);
        println!("{}", empty.cells());
        assert_eq!(empty.validate_base(), Ok(()));
    }

    #[test]
    fn valid_random_grid() {
        let mut random = Puzzle::new("x".to_string(), 14);
        random.random_black();
        println!("{}", random.cells());
        assert_eq!(random.validate_base(), Ok(()));
    }

    #[test]
    fn valid_black_placement() {
        let cells = Grid(vec![
            vec![
                Cell::Black,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
            ],
            vec![
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
            ],
            vec![
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
            ],
            vec![
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
            ],
            vec![
                Cell::Empty,
                Cell::Empty,
                Cell::Letter('B'),
                Cell::Letter('A'),
                Cell::Empty,
            ],
        ]);
        let puzzle = Puzzle::from_grid("x".to_string(), cells);
        assert_eq!(puzzle.valid_black_placement((0, 1)), true);
        assert_eq!(puzzle.valid_black_placement((1, 1)), false);
        assert_eq!(puzzle.valid_black_placement((2, 2)), false);
        assert_eq!(puzzle.valid_black_placement((3, 4)), false);
        assert_eq!(puzzle.valid_black_placement((4, 4)), true);
    }

    #[test]
    fn valid_words() {
        let cells = Grid(vec![
            vec![Cell::Letter('S'), Cell::Letter('I'), Cell::Letter('T')],
            vec![Cell::Letter('A'), Cell::Letter('T'), Cell::Letter('E')],
            vec![Cell::Letter('P'), Cell::Letter('A'), Cell::Letter('N')],
        ]);
        let puzzle = Puzzle::from_grid("x".to_string(), cells);
        assert_eq!(puzzle.validate_words(), Ok(()));
    }

    #[test]
    fn words_too_short() {
        let cells = Grid(vec![
            vec![Cell::Letter('S'), Cell::Letter('I'), Cell::Letter('T')],
            vec![Cell::Letter('A'), Cell::Black, Cell::Letter('E')],
            vec![Cell::Letter('P'), Cell::Letter('U'), Cell::Letter('N')],
        ]);
        let puzzle = Puzzle::from_grid("x".to_string(), cells);
        assert_eq!(
            puzzle.validate_words(),
            Err(PuzzleError::WordTooShort("A".to_string()))
        );
    }

    #[test]
    fn words_iter() {
        let cells = Grid(vec![
            vec![Cell::Letter('S'), Cell::Letter('I'), Cell::Letter('T')],
            vec![Cell::Letter('A'), Cell::Letter('C'), Cell::Letter('E')],
            vec![Cell::Letter('P'), Cell::Letter('E'), Cell::Letter('N')],
        ]);
        let puzzle = Puzzle::from_grid("x".to_string(), cells);

        let across_words: Vec<String> = puzzle
            .words_across_iter()
            .map(|x| Cell::as_string(x))
            .collect();
        let down_words: Vec<String> = puzzle
            .words_down_iter()
            .map(|x| Cell::as_string(x))
            .collect();

        assert_eq!(vec!["SIT", "ACE", "PEN"], across_words);
        assert_eq!(vec!["SAP", "ICE", "TEN"], down_words);
    }

    #[test]
    fn get_words() {
        let cells = Grid(vec![
            vec![
                Cell::Black,
                Cell::Letter('S'),
                Cell::Letter('I'),
                Cell::Letter('T'),
                Cell::Black,
            ],
            vec![
                Cell::Letter('F'),
                Cell::Letter('A'),
                Cell::Letter('C'),
                Cell::Letter('E'),
                Cell::Letter('S'),
            ],
            vec![
                Cell::Letter('F'),
                Cell::Letter('A'),
                Cell::Black,
                Cell::Letter('E'),
                Cell::Letter('S'),
            ],
            vec![
                Cell::Letter('F'),
                Cell::Letter('A'),
                Cell::Letter('C'),
                Cell::Letter('E'),
                Cell::Letter('S'),
            ],
            vec![
                Cell::Black,
                Cell::Letter('P'),
                Cell::Letter('E'),
                Cell::Letter('N'),
                Cell::Black,
            ],
        ]);
        let puzzle = Puzzle::from_grid("x".to_string(), cells);

        assert_eq!(
            puzzle.get_across_word(1),
            Some(SparseWord::new(vec![Some('S'), Some('I'), Some('T')]))
        );
        assert_eq!(
            puzzle.get_across_word(10),
            Some(SparseWord::new(vec![Some('F'), Some('A')]))
        );
        assert_eq!(
            puzzle.get_across_word(13),
            Some(SparseWord::new(vec![Some('E'), Some('S')]))
        );

        assert_eq!(
            puzzle.get_down_word(1),
            Some(SparseWord::new(vec![
                Some('S'),
                Some('A'),
                Some('A'),
                Some('A'),
                Some('P')
            ]))
        );
        assert_eq!(
            puzzle.get_down_word(3),
            Some(SparseWord::new(vec![
                Some('T'),
                Some('E'),
                Some('E'),
                Some('E'),
                Some('N')
            ]))
        );
        assert_eq!(
            puzzle.get_down_word(2),
            Some(SparseWord::new(vec![Some('I'), Some('C')]))
        );
        assert_eq!(
            puzzle.get_down_word(17),
            Some(SparseWord::new(vec![Some('C'), Some('E')]))
        );

        assert_eq!(puzzle.get_across_word(0), None);
        assert_eq!(puzzle.get_down_word(0), None);
    }
}
