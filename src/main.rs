use std::{collections::HashMap, error::Error, fmt::Display};

use colored::Colorize;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Position {
    x: usize,
    y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        return Self { x, y };
    }
}

#[derive(Debug)]
pub struct Sudoku {
    grid: Vec<Vec<Option<u8>>>,
    prefilled_positions: HashMap<Position, u8>,
}

impl Display for Sudoku {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in &mut self.grid.iter().enumerate() {
            if i.0 == 0 {
                write!(f, "{}", "    0  1  2   3  4  5   6  7  8 \n".italic())
                    .expect("error displaying board 1");
                write!(f, "   {}\n", "-----------------------------".blue())
                    .expect("error displaying board 2");
            }

            write!(f, "{} {}", i.0.to_string().italic(), "|".blue())
                .expect("error displaying board 3");

            for j in i.1.iter().enumerate() {
                match j.1 {
                    Some(v) => {
                        if self
                            .prefilled_positions
                            .contains_key(&Position::new(i.0, j.0))
                        {
                            write!(f, " {} ", v.to_string().bold())
                                .expect("error displaying board 4");
                        } else {
                            write!(f, " {v} ").expect("error displaying board 5");
                        }
                    }
                    None => {
                        write!(f, "   ").expect("error displaying board 6");
                    }
                }

                if (j.0 + 1) % 3 == 0 {
                    write!(f, "{}", "|".blue()).expect("error displaying board 7");
                }
            }

            write!(f, "\n").expect("error displaying board 8");

            if (i.0 + 1) % 3 == 0 {
                write!(f, "   {}\n", "-----------------------------".blue())
                    .expect("error displaying board 9");
            }
        }

        // f.write_fmt(format_args!("Board:\n{str}"))
        Ok(())
    }
}

impl Sudoku {
    const TOTAL_POSITIONS: usize = 81;
    pub fn from_str(inp: &str) -> Result<Self, Box<dyn Error>> {
        let split = inp.split(",");

        let split_cells = split.collect::<Vec<&str>>();

        let cell_count = split_cells.len();

        if cell_count != Sudoku::TOTAL_POSITIONS {
            return Err(format!(
                "invalid input found, expected {} cells, found {}",
                Sudoku::TOTAL_POSITIONS,
                cell_count
            )
            .into());
        }

        let mut prefilled_positions = HashMap::new();

        let mut list = vec![];

        for sc in split_cells.iter().enumerate() {
            let v = sc.1.trim();

            if v.is_empty() {
                list.push(None);
                continue;
            }

            let val = v.parse::<u8>();

            if val.is_err() {
                list.push(None);
                continue;
            }

            let val = val.unwrap();

            if val < 1 || val > 9 {
                return Err(
                    "input values cannot contain values less than 1 or greater than 9".into(),
                );
            }

            prefilled_positions.insert(Position::new(sc.0 / 9, sc.0 % 9), val);

            list.push(Some(val));
        }

        let res = list
            .chunks(9)
            .map(|v| v.to_vec())
            .collect::<Vec<Vec<Option<u8>>>>();

        Ok(Sudoku {
            grid: res,
            prefilled_positions,
        })
    }

    fn to_str(&self) -> String {
        let mut resp = String::new();

        for i in &self.grid {
            for j in i {
                match j {
                    Some(k) => resp = format!("{resp}{k},"),
                    None => resp = format!("{resp},"),
                }
            }
        }

        resp
    }

    /// board is valid if the number placements obey the row, column and block rules
    fn is_board_valid(&self) -> bool {
        for row in self.grid.iter().enumerate() {
            for col in row.1.iter().enumerate() {
                if col.1.is_some() {
                    let pos = Position::new(row.0, col.0);
                    let val = col.1.unwrap();

                    if !self.is_pos_valid(&pos, val) {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn is_present_in_block(&self, pos: &Position, val: u8) -> bool {
        let x = (pos.x / 3) * 3;
        let y = (pos.y / 3) * 3;

        for i in 0..3 {
            for j in 0..3 {
                if x + i == pos.x && y + j == pos.y {
                    continue;
                }

                if self.grid[(x + i) as usize][(y + j) as usize] == Some(val) {
                    return true;
                }
            }
        }

        false
    }

    fn is_present_in_column(&self, pos: &Position, val: u8) -> bool {
        for i in 0..9 {
            if pos.x == i {
                continue;
            }

            if self.grid[i as usize][pos.y as usize] == Some(val) {
                return true;
            }
        }

        false
    }

    fn is_present_in_row(&self, pos: &Position, val: u8) -> bool {
        for i in 0..9 {
            if pos.y == i {
                continue;
            }

            if self.grid[pos.x as usize][i as usize] == Some(val) {
                return true;
            }
        }

        false
    }

    fn insert_at(&mut self, pos: &Position, val: Option<u8>) {
        self.grid[pos.x as usize][pos.y as usize] = val;
    }

    fn get(&self, pos: &Position) -> Option<u8> {
        self.grid[pos.x as usize][pos.y as usize]
    }

    fn is_pos_valid(&self, pos: &Position, val: u8) -> bool {
        !(self.is_present_in_block(&pos, val)
            || self.is_present_in_row(&pos, val)
            || self.is_present_in_column(&pos, val))
    }

    fn fetch_empty_cells(&self) -> Vec<Position> {
        let mut empty_cells = vec![];

        for i in &mut self.grid.iter().enumerate() {
            for j in i.1.iter().enumerate() {
                if j.1.is_none() {
                    empty_cells.push(Position::new(i.0, j.0));
                }
            }
        }

        empty_cells
    }

    // fn fetch_empty_cell(&self) -> Position {

    // }

    fn solve(&mut self, seed_value: Option<u8>) -> bool {
        let mut filled_stack = vec![];
        let mut empty_cells_stack = self.fetch_empty_cells();
        let mut reached_dead_end = false;

        let mut seed_value = seed_value;

        loop {
            // if no other way to go
            if empty_cells_stack.is_empty() && filled_stack.is_empty() {
                return false;
            }

            if empty_cells_stack.is_empty() && !reached_dead_end {
                break;
            }

            if reached_dead_end {
                if filled_stack.is_empty() {
                    reached_dead_end = false;
                    continue;
                }

                let filled_pos = filled_stack.pop().unwrap();

                let val = self.get(&filled_pos);

                match val {
                    Some(v) => {
                        if v == 9 {
                            self.insert_at(&filled_pos, None);
                            empty_cells_stack.push(filled_pos.clone());
                        }

                        for i in v + 1..=9 {
                            self.insert_at(&filled_pos, Some(i));

                            // validate pos
                            if self.is_pos_valid(&filled_pos, i) {
                                filled_stack.push(filled_pos.clone());
                                reached_dead_end = false;
                                break;
                            }

                            if i == 9 {
                                self.insert_at(&filled_pos, None);
                                empty_cells_stack.push(filled_pos.clone());
                            }
                        }
                    }
                    None => return false,
                }
            } else {
                if empty_cells_stack.is_empty() {
                    break;
                }

                let empty_pos = empty_cells_stack.pop().unwrap();

                let mut k = 1;

                if seed_value.is_some() {
                    k = seed_value.unwrap();
                    seed_value = None;
                }

                for i in k..=9 {
                    self.insert_at(&empty_pos, Some(i));

                    // validate pos
                    if self.is_pos_valid(&empty_pos, i) {
                        filled_stack.push(empty_pos.clone());
                        break;
                    }

                    if i == 9 {
                        self.insert_at(&empty_pos, None);
                        empty_cells_stack.push(empty_pos.clone());
                        reached_dead_end = true;
                    }
                }
            }
        }

        self.is_board_valid()
    }

    fn reset(&mut self) {
        for i in &mut self.grid.clone().iter().enumerate() {
            for j in i.1.iter().enumerate() {
                let pos = Position::new(i.0, j.0);
                if !self.prefilled_positions.contains_key(&pos) {
                    self.grid[i.0][j.0] = None;
                }
            }
        }
    }

    /// puzzle can only be valid if there is only one valid solution to it
    fn is_puzzle_valid(&mut self) -> bool {
        let mut prev = String::new();

        for i in 1..=9 {
            self.reset();

            if !self.solve(Some(i)) {
                return false;
            }

            let curr = self.to_str();

            if !prev.is_empty() {
                if prev != curr {
                    return false;
                }
            }

            prev = curr;
        }

        true
    }
}

fn main() {
    println!("Hello, world!");
    // tough
    // let str_val = ",,,1,,2,,,,,6,,,,,,7,,,,8,,,,9,,,4,,,,,,,,3,,5,,,,7,,,,2,,,,8,,,,1,,,9,,,,8,,5,,7,,,,,,6,,,,,3,,4,,,";

    // easy
    let str_val = "4,,9,,7,2,,1,3,7,,2,8,3,,6,,,,1,6,,4,9,8,7,,2,,,1,,,,6,,5,4,7,,,,2,,,6,9,,,,4,,3,5,8,,3,4,,,,,6,,,,,,3,1,,,,6,,9,,,,4,";

    // easy 2
    // let str_val = ",8,7,,5,,,,,4,9,,,3,6,1,,,5,1,,9,8,2,,,4,,,,,,5,4,,6,7,,,,6,9,,1,,1,,,,4,,7,5,,2,,,8,1,3,6,,9,9,4,,,,7,,3,,,,,,,4,8,,7";

    // let str_val =
    //     "8,2,,,,,,,,,,4,,,,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

    let mut sudoku = Sudoku::from_str(str_val).expect("expected a valid sudoku puzzle");

    println!("{sudoku}");

    sudoku.solve(None);

    println!("{sudoku}");

    println!("Is puzzle valid: {}", sudoku.is_puzzle_valid());
}

#[cfg(test)]
mod test {
    use crate::{Position, Sudoku};

    #[test]
    pub fn simple_test() {
        let str_val = "8,2,,,,,,,,,,4,,,,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

        let board = Sudoku::from_str(str_val);

        assert!(board.is_ok());

        let mut board = board.expect("didn't expect an error");

        println!("{}", board);

        assert_eq!(board.prefilled_positions.len(), 9);

        for pos in vec![
            Position::new(0, 0),
            Position::new(1, 2),
            Position::new(2, 2),
            Position::new(5, 0),
            Position::new(6, 6),
            Position::new(8, 4),
        ] {
            assert!(board.prefilled_positions.contains_key(&pos));
        }

        assert!(!board.prefilled_positions.contains_key(&Position::new(1, 5)));
        assert!(!board.prefilled_positions.contains_key(&Position::new(5, 8)));

        assert!(board.is_board_valid());

        assert!(!board.is_puzzle_valid());
    }

    #[test]
    pub fn invalid_block() {
        let str_val = "8,2,8,,,,,,,,,4,,,,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

        let board = Sudoku::from_str(str_val);

        assert!(board.is_ok());

        let board = board.expect("didn't expect an error");

        assert_eq!(board.prefilled_positions.len(), 10);
        assert!(board.prefilled_positions.contains_key(&Position::new(0, 0)));
        assert!(board.prefilled_positions.contains_key(&Position::new(1, 2)));
        assert!(!board.prefilled_positions.contains_key(&Position::new(1, 5)));

        assert!(!board.is_board_valid());
        assert!(board.is_present_in_block(&Position::new(0, 2), 8));
    }

    #[test]
    pub fn invalid_row() {
        let str_val = "8,2,,,,,,,,,,4,,,4,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

        let board = Sudoku::from_str(str_val);

        assert!(board.is_ok());

        let board = board.expect("didn't expect an error");

        assert_eq!(board.prefilled_positions.len(), 10);
        assert!(board.prefilled_positions.contains_key(&Position::new(0, 0)));
        assert!(board.prefilled_positions.contains_key(&Position::new(1, 2)));
        assert!(!board.prefilled_positions.contains_key(&Position::new(1, 6)));

        assert!(!board.is_board_valid());
        assert!(board.is_present_in_row(&Position::new(1, 6), 4));
    }

    #[test]
    pub fn invalid_column() {
        let str_val = "8,2,,,,,,,,,,4,,,4,,,,,,1,,,4,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

        let board = Sudoku::from_str(str_val);

        assert!(board.is_ok());

        let board = board.expect("didn't expect an error");

        println!("{board}");

        assert_eq!(board.prefilled_positions.len(), 11);
        assert!(board.prefilled_positions.contains_key(&Position::new(0, 0)));
        assert!(board.prefilled_positions.contains_key(&Position::new(1, 2)));
        assert!(!board.prefilled_positions.contains_key(&Position::new(1, 6)));

        assert!(!board.is_board_valid());
        assert!(board.is_present_in_column(&Position::new(2, 5), 4));

        dbg!(board);
    }

    #[test]
    pub fn toughest_valid() {
        let str_val = ",,,1,,2,,,,,6,,,,,,7,,,,8,,,,9,,,4,,,,,,,,3,,5,,,,7,,,,2,,,,8,,,,1,,,9,,,,8,,5,,7,,,,,,6,,,,,3,,4,,,";

        let board = Sudoku::from_str(str_val);

        assert!(board.is_ok());

        let board = board.expect("didn't expect an error");

        assert_eq!(board.prefilled_positions.len(), 20);
        assert!(board.prefilled_positions.contains_key(&Position::new(0, 3)));
        assert!(board.prefilled_positions.contains_key(&Position::new(1, 1)));
        assert!(!board.prefilled_positions.contains_key(&Position::new(2, 5)));

        assert!(board.is_board_valid());
    }

    #[test]
    pub fn extreme_26() {
        let str_val = "1,,,,6,,,,,,,3,9,,1,,4,,2,,,,,,,,7,,,,,8,,,5,,,,6,,4,,,,,3,,,5,,6,2,,,,,1,3,,5,,9,,,,,8,,,,,,,9,,,,,4,,";

        let board = Sudoku::from_str(str_val);

        assert!(board.is_ok());

        let mut board = board.expect("didn't expect an error");

        println!("{}", board);

        assert_eq!(board.prefilled_positions.len(), 23);

        assert!(board.is_board_valid());
        assert!(board.solve(None));
        assert!(board.is_puzzle_valid());
    }
}
