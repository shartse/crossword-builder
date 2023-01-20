use dictionary::DICTIONARY;
use rand::Rng;
use std::{
    cmp::max,
    collections::HashMap,
    fmt::{self, Debug},
    fs::File,
    io::{Read, Write},
    str::Utf8Error,
};
use thiserror::Error;

use crate::{dictionary, PERCENT_BLACK, PUZZLE_DIR};

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

#[derive(Error, Debug, PartialEq)]
pub enum GridError {
    #[error("Invalid puzzle file format")]
    InvalidPuzzleFormat,
    #[error("Puzzle file not in utf8: {0}")]
    NonUtf8(Utf8Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid(Vec<Vec<Cell>>);

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.0 {
            for cell in row {
                write!(f, "{}", cell)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

impl Grid {
    fn new(size: usize) -> Self {
        let mut grid = Vec::new();
        for _n in 0..size {
            let mut row = Vec::new();
            for _i in 0..size {
                row.push(Cell::Empty);
            }
            grid.push(row);
        }
        Grid(grid)
    }

    fn from_bytes(buf: &Vec<u8>) -> Result<Self, GridError> {
        let mut cells = Vec::new();
        for row in buf.split(|x| *x == '\n' as u8) {
            if row.len() > 0 {
                let row_str = std::str::from_utf8(row).map_err(|e| GridError::NonUtf8(e))?;
                let row_cells: Result<Vec<Cell>, _> = row_str
                    .split_ascii_whitespace()
                    .map(|s| Cell::from_str(s))
                    .collect();
                let row_cells = row_cells?;
                cells.push(row_cells)
            }
        }
        Ok(Grid(cells))
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn transpose(&self) -> Self {
        assert!(!self.0.is_empty());
        Grid(
            (0..self.0[0].len())
                .map(|i| {
                    self.0
                        .iter()
                        .map(|inner| inner[i].clone())
                        .collect::<Vec<Cell>>()
                })
                .collect(),
        )
    }

    fn rows_iter(&self) -> impl Iterator<Item = &Vec<Cell>> {
        self.0.iter()
    }

    fn cells_row_major_iter(&self) -> impl Iterator<Item = &Cell> {
        let cells: Vec<&Cell> = self.0.iter().flatten().collect();
        cells.into_iter()
    }

    #[allow(dead_code)]
    fn cells_row_major_iter_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        let cells: Vec<&mut Cell> = self.0.iter_mut().flatten().collect();
        cells.into_iter()
    }

    fn set(&mut self, x: usize, y: usize, value: Cell) {
        let row = self.0.get_mut(y).unwrap();
        let cell = row.get_mut(x).unwrap();
        *cell = value.clone();
    }

    fn get(&self, x: usize, y: usize) -> &Cell {
        self.0.get(y).unwrap().get(x).unwrap()
    }

    fn get_row(&self, row: usize) -> &Vec<Cell> {
        self.0.get(row).unwrap()
    }

    /// Rotate the puzzle 180 degrees by reversing the order of the rows and the contents of the rows
    fn rotate_180(&mut self) {
        self.0.reverse();
        for row in self.0.iter_mut() {
            row.reverse();
        }
    }

    fn get_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        self.0.get_mut(y).unwrap().get_mut(x).unwrap()
    }

    fn is_square(&self) -> Result<(), PuzzleError> {
        let size = self.len();
        for row in &self.0 {
            if row.len() != size {
                return Err(PuzzleError::NotSymmetric);
            }
        }
        return Ok(());
    }

    /// Verify that the black sqaures in both puzzles are in the same locations
    fn black_squares_match(&self, other: Self) -> bool {
        for y in 0..self.len() {
            for x in 0..self.len() {
                let left = self.get(x, y);
                let right = other.get(x, y);
                if (left == &Cell::Black && right != &Cell::Black)
                    || (right == &Cell::Black && left != &Cell::Black)
                {
                    return false;
                }
            }
        }
        return true;
    }

    /// "Generally this rule means that if you turn the grid upside-down, the pattern will look the same as it
    /// does right-side-up. "
    fn is_symmetric(&self) -> Result<(), PuzzleError> {
        let mut flipped_grid = self.clone();
        flipped_grid.rotate_180();
        if self.black_squares_match(flipped_grid) {
            Ok(())
        } else {
            Err(PuzzleError::NotSymmetric)
        }
    }

    /// Check that the black squares account for no more than 16 percent of the total grid
    fn acceptable_black_square_count(&self) -> Result<(), PuzzleError> {
        let size = self.len();
        let total = size * size;
        let mut black = 0;
        for cell in self.cells_row_major_iter() {
            if let Cell::Black = cell {
                black += 1;
            }
        }
        if ((black * 100) / total) <= PERCENT_BLACK {
            Ok(())
        } else {
            Err(PuzzleError::TooManyBlackSquares(PERCENT_BLACK))
        }
    }

    /// Check that the the distance to the end of a slice or to the first black Cell is either 0 or greater than or equal to 3.
    fn ok_dist_to_black_or_edge(row: &[Cell]) -> bool {
        let mut dist = 0;
        for x in row.iter() {
            if matches!(x, Cell::Black) {
                break;
            }
            dist += 1;
        }
        return dist == 0 || dist >= 3;
    }
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

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
enum Cell {
    Black,
    Empty,
    Letter(char),
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cell = match self {
            Cell::Black => '▩',
            Cell::Empty => '▢',
            Cell::Letter(letter) => *letter,
        };
        write!(f, "{} ", cell)
    }
}

impl Cell {
    fn letter(&self) -> &char {
        match self {
            Cell::Black => panic!("Not a letter"),
            Cell::Empty => &'_',
            Cell::Letter(l) => l,
        }
    }

    fn from_str(s: &str) -> Result<Self, GridError> {
        let c = s.trim();
        let c = c.chars().next().unwrap();
        match c {
            '▩' => Ok(Cell::Black),
            '▢' => Ok(Cell::Empty),
            l => {
                if l.is_alphabetic() {
                    Ok(Cell::Letter(l))
                } else {
                    Err(GridError::InvalidPuzzleFormat)
                }
            }
        }
    }

    fn as_string(cells: &[Cell]) -> String {
        cells.iter().map(|x| x.letter()).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
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
}
