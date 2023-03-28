use std::{fmt, str::Utf8Error};
use thiserror::Error;

use crate::{puzzle::PuzzleError, PERCENT_BLACK};

#[derive(Error, Debug, PartialEq)]
pub enum GridError {
    #[error("Invalid puzzle file format")]
    InvalidPuzzleFormat,
    #[error("Puzzle file not in utf8: {0}")]
    NonUtf8(Utf8Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid(pub Vec<Vec<Cell>>);

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
    pub fn new(size: usize) -> Self {
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

    pub fn from_bytes(buf: &Vec<u8>) -> Result<Self, GridError> {
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

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn transpose(&self) -> Self {
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

    pub fn rows_iter(&self) -> impl Iterator<Item = &Vec<Cell>> {
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

    pub fn set(&mut self, x: usize, y: usize, value: Cell) {
        let row = self.0.get_mut(y).unwrap();
        let cell = row.get_mut(x).unwrap();
        *cell = value.clone();
    }

    pub fn get(&self, x: usize, y: usize) -> &Cell {
        self.0.get(y).unwrap().get(x).unwrap()
    }

    pub fn get_row(&self, row: usize) -> &Vec<Cell> {
        self.0.get(row).unwrap()
    }

    #[allow(dead_code)]
    fn get_row_mut(&mut self, row: usize) -> &mut Vec<Cell> {
        self.0.get_mut(row).unwrap()
    }

    /// Rotate the puzzle 180 degrees by reversing the order of the rows and the contents of the rows
    fn rotate_180(&mut self) {
        self.0.reverse();
        for row in self.0.iter_mut() {
            row.reverse();
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut Cell {
        self.0.get_mut(y).unwrap().get_mut(x).unwrap()
    }

    pub fn is_square(&self) -> Result<(), PuzzleError> {
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
    pub fn is_symmetric(&self) -> Result<(), PuzzleError> {
        let mut flipped_grid = self.clone();
        flipped_grid.rotate_180();
        if self.black_squares_match(flipped_grid) {
            Ok(())
        } else {
            Err(PuzzleError::NotSymmetric)
        }
    }

    /// Check that the black squares account for no more than 16 percent of the total grid
    pub fn acceptable_black_square_count(&self) -> Result<(), PuzzleError> {
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
    pub fn ok_dist_to_black_or_edge(row: &[Cell]) -> bool {
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

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum Cell {
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

    pub fn as_string(cells: &[Cell]) -> String {
        cells.iter().map(|x| x.letter()).collect()
    }
}
