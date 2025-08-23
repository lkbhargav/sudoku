use colored::Colorize;
use rand::Rng;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    io::{self, Write},
};

#[derive(PartialEq, Eq, Debug, Clone)]
enum CellState {
    Normal,
    UserMarkedDefault,
    Wrong,
    Hinted,
}

pub enum InsertStatus {
    Wrong,
    Right,
    ValuePresent,
}

pub enum HintStatus {
    Ok,
    ValuePresent,
}

type Board = Vec<Vec<(Option<u8>, CellState)>>;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Position {
    x: usize,
    y: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "x: {}, y: {}", self.x, self.y)
    }
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        return Self { x, y };
    }

    pub fn parse(pos: &str) -> Result<Self, Box<dyn Error>> {
        let pos = pos.trim();

        let vals = pos.split_once("g");

        if vals.is_none() {
            return Err(
                "invalid position given, expected in g00 format. Error while splitting at g".into(),
            );
        }

        let vals = vals.unwrap().1.split_once(",");

        if vals.is_none() {
            return Err(
                "invalid position given, expected in g00 format. Error while splitting at ,".into(),
            );
        }

        let x = match vals.unwrap().0.parse::<usize>() {
            Ok(v) => v,
            Err(e) => {
                return Err(e.to_string().into());
            }
        };

        let y = match vals.unwrap().1.parse::<usize>() {
            Ok(v) => v,
            Err(e) => {
                return Err(e.to_string().into());
            }
        };

        Ok(Position::new(x, y))
    }
}

#[derive(Debug)]
pub struct Sudoku {
    grid: Board,
    prefilled_positions: HashMap<Position, u8>,
    solved_grid: Board,
    highlighted: Option<u8>,
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
                match j.1.0 {
                    Some(v) => {
                        if self
                            .prefilled_positions
                            .contains_key(&Position::new(i.0, j.0))
                        {
                            let mut val = v.to_string().bold();
                            if self.highlighted.is_some() {
                                if j.1.0.unwrap() == self.highlighted.unwrap() {
                                    val = v.to_string().on_bright_yellow().green().bold();
                                }
                            }

                            write!(f, " {} ", val).expect("error displaying board 4");
                        } else {
                            let mut val = match j.1.1 {
                                CellState::Hinted => v.to_string().magenta().bold(),
                                CellState::Wrong => v.to_string().red().bold(),
                                CellState::UserMarkedDefault => v.to_string().yellow().bold(),
                                _ => v.to_string().green(),
                            };

                            if self.highlighted.is_some() {
                                if j.1.0.unwrap() == self.highlighted.unwrap() {
                                    val = val.on_bright_yellow().green().bold();
                                }
                            }

                            write!(f, " {} ", val).expect("error displaying board 5");
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

    pub fn generate_random_board(number_of_clues: u8) -> Self {
        let mut number_of_clues = number_of_clues;

        if number_of_clues < 10 {
            number_of_clues = 10;
        } else if number_of_clues > 80 {
            number_of_clues = 80;
        }

        let mut rng = rand::rng();

        'outer: loop {
            let mut grid: Board = vec![vec![(None, CellState::Normal); 9]; 9];

            for i in 0..9 {
                for j in 0..9 {
                    let pos = Position::new(i, j);
                    let mut counter = 0;

                    loop {
                        let val = rng.random_range(1..=9) as u8;

                        if Sudoku::is_present_in_block(&grid, &pos, val)
                            || Sudoku::is_present_in_column(&grid, &pos, val)
                            || Sudoku::is_present_in_row(&grid, &pos, val)
                        {
                            counter += 1;
                            if counter >= 20 {
                                continue 'outer;
                            }

                            continue;
                        }

                        grid[i][j].0 = Some(val);
                        break;
                    }
                }
            }

            print!(",");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut number_of_removals = 81 - number_of_clues;

            while number_of_removals > 0 {
                let x = rng.random_range(0..9);
                let y = rng.random_range(0..9);

                if grid[x][y].0 != None {
                    grid[x][y].0 = None;
                    number_of_removals -= 1;
                }
            }

            print!("!");
            io::stdout().flush().expect("Failed to flush stdout");

            let mut prefilled_positions = HashMap::new();

            for i in grid.iter().enumerate() {
                for j in i.1.iter().enumerate() {
                    if j.1.0.is_some() {
                        prefilled_positions.insert(Position::new(i.0, j.0), j.1.0.unwrap());
                    }
                }
            }

            let mut board = Self {
                grid: grid.clone(),
                prefilled_positions,
                solved_grid: grid,
                highlighted: None,
            };

            if board.is_board_valid() && board.is_puzzle_valid() {
                println!("");

                board.solved_grid = board.grid.clone();
                board.reset();
                return board;
            }

            print!(".");
            io::stdout().flush().expect("Failed to flush stdout");
        }
    }

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

        let mut list: Vec<(Option<u8>, CellState)> = vec![];

        for sc in split_cells.iter().enumerate() {
            let mut v = sc.1.trim().to_string();

            if v.is_empty() {
                list.push((None, CellState::Normal));
                continue;
            }

            let mut is_user_defined = false;

            // user input number, would be in the form of u7, basically prefixed with a u
            if v.len() == 2 {
                let c = v.to_lowercase().chars().collect::<Vec<char>>();

                if c[0] != 'u' {
                    return Err(
                        "expected first char to be `u` for user input but found otherwise".into(),
                    );
                }

                is_user_defined = true;

                v = c[1].to_string();
            }

            let val = v.parse::<u8>();

            if val.is_err() {
                list.push((None, CellState::Normal));
                continue;
            }

            let val = val.unwrap();

            if val < 1 || val > 9 {
                return Err(
                    "input values cannot contain values less than 1 or greater than 9".into(),
                );
            }

            if !is_user_defined {
                prefilled_positions.insert(Position::new(sc.0 / 9, sc.0 % 9), val);
            }

            if is_user_defined {
                list.push((Some(val), CellState::UserMarkedDefault));
            } else {
                list.push((Some(val), CellState::Normal));
            }
        }

        let res = list.chunks(9).map(|v| v.to_vec()).collect::<Board>();

        let mut sudoku = Sudoku {
            grid: res.clone(),
            prefilled_positions,
            solved_grid: res,
            highlighted: None,
        };

        if sudoku.is_board_valid() && sudoku.is_puzzle_valid() {
            println!("");

            sudoku.solved_grid = sudoku.grid.clone();
            sudoku.reset();
            return Ok(sudoku);
        }

        Err("invalid board given".into())
    }

    pub fn to_str(&self) -> String {
        let mut resp = String::new();

        for i in &mut self.grid.iter().enumerate() {
            for j in i.1.iter().enumerate() {
                let mut s = "";

                if !self
                    .prefilled_positions
                    .contains_key(&Position::new(i.0, j.0))
                {
                    s = "u";
                };

                match j.1.0 {
                    Some(k) => resp = format!("{resp}{s}{k},"),
                    None => resp = format!("{resp},"),
                }
            }
        }

        resp
    }

    pub fn is_board_solved_completely(&self) -> bool {
        for i in &self.grid {
            for j in i {
                if j.0.is_none() {
                    return false;
                }
            }
        }

        true
    }

    /// board is valid if the number placements obey the row, column and block rules
    pub fn is_board_valid(&self) -> bool {
        for row in self.grid.iter().enumerate() {
            for col in row.1.iter().enumerate() {
                if col.1.0.is_some() {
                    let pos = Position::new(row.0, col.0);
                    let val = col.1.0.unwrap();

                    if !self.is_pos_valid(&pos, val) {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn is_present_in_block(grid: &Board, pos: &Position, val: u8) -> bool {
        let x = (pos.x / 3) * 3;
        let y = (pos.y / 3) * 3;

        for i in 0..3 {
            for j in 0..3 {
                if x + i == pos.x && y + j == pos.y {
                    continue;
                }

                if grid[x + i][y + j].0 == Some(val) {
                    return true;
                }
            }
        }

        false
    }

    fn is_present_in_column(grid: &Board, pos: &Position, val: u8) -> bool {
        for i in 0..9 {
            if pos.x == i {
                continue;
            }

            if grid[i][pos.y].0 == Some(val) {
                return true;
            }
        }

        false
    }

    fn is_present_in_row(grid: &Board, pos: &Position, val: u8) -> bool {
        for i in 0..9 {
            if pos.y == i {
                continue;
            }

            if grid[pos.x][i].0 == Some(val) {
                return true;
            }
        }

        false
    }

    fn insert(&mut self, pos: &Position, val: Option<u8>) {
        self.grid[pos.x][pos.y] = (val, CellState::Normal);
    }

    pub fn insert_at(&mut self, pos: &Position, val: Option<u8>) -> InsertStatus {
        let mut mv = (val, CellState::Normal);

        let mut resp = InsertStatus::Right;

        if val.is_some() {
            if self.grid[pos.x][pos.y].0.is_some() {
                return InsertStatus::ValuePresent;
            }

            if self.solved_grid[pos.x][pos.y].0 != val {
                mv.1 = CellState::Wrong;
                resp = InsertStatus::Wrong;
            }
        }

        self.grid[pos.x][pos.y] = mv;

        resp
    }

    pub fn hint(&mut self, pos: &Position) -> HintStatus {
        if self.grid[pos.x][pos.y].0.is_some() {
            return HintStatus::ValuePresent;
        }

        self.grid[pos.x][pos.y] = (self.solved_grid[pos.x][pos.y].0, CellState::Hinted);

        HintStatus::Ok
    }

    pub fn highlight(&mut self, val: u8) {
        match self.highlighted {
            Some(v) => {
                if v == val {
                    self.highlighted = None;
                } else {
                    self.highlighted = Some(val);
                }
            }
            None => self.highlighted = Some(val),
        }
    }

    fn get(&self, pos: &Position) -> Option<u8> {
        self.grid[pos.x][pos.y].0
    }

    fn is_pos_valid(&self, pos: &Position, val: u8) -> bool {
        !(Sudoku::is_present_in_block(&self.grid, &pos, val)
            || Sudoku::is_present_in_row(&self.grid, &pos, val)
            || Sudoku::is_present_in_column(&self.grid, &pos, val))
    }

    fn fetch_empty_cells(&self) -> Vec<Position> {
        let mut empty_cells = vec![];

        for i in &mut self.grid.iter().enumerate() {
            for j in i.1.iter().enumerate() {
                if j.1.0.is_none() {
                    empty_cells.push(Position::new(i.0, j.0));
                }
            }
        }

        empty_cells
    }

    pub fn solve(&mut self, seed_value: Option<u8>) -> bool {
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
                            self.insert(&filled_pos, None);
                            empty_cells_stack.push(filled_pos.clone());
                        }

                        for i in v + 1..=9 {
                            self.insert(&filled_pos, Some(i));

                            // validate pos
                            if self.is_pos_valid(&filled_pos, i) {
                                filled_stack.push(filled_pos.clone());
                                reached_dead_end = false;
                                break;
                            }

                            if i == 9 {
                                self.insert(&filled_pos, None);
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
                    self.insert(&empty_pos, Some(i));

                    // validate pos
                    if self.is_pos_valid(&empty_pos, i) {
                        filled_stack.push(empty_pos.clone());
                        break;
                    }

                    if i == 9 {
                        self.insert(&empty_pos, None);
                        empty_cells_stack.push(empty_pos.clone());
                        reached_dead_end = true;
                    }
                }
            }
        }

        self.is_board_valid()
    }

    pub fn reset(&mut self) {
        for i in &mut self.grid.clone().iter().enumerate() {
            for j in i.1.iter().enumerate() {
                let pos = Position::new(i.0, j.0);
                if !(self.prefilled_positions.contains_key(&pos)
                    || self.grid[i.0][j.0].1 == CellState::UserMarkedDefault)
                {
                    self.grid[i.0][j.0].0 = None;
                }
            }
        }
    }

    pub fn hard_reset(&mut self) {
        for i in &mut self.grid.clone().iter().enumerate() {
            for j in i.1.iter().enumerate() {
                let pos = Position::new(i.0, j.0);
                if !self.prefilled_positions.contains_key(&pos) {
                    self.grid[i.0][j.0].0 = None;
                }
            }
        }
    }

    /// puzzle can only be valid if there is only one valid solution to it
    pub fn is_puzzle_valid(&mut self) -> bool {
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

#[cfg(test)]
mod test {
    use crate::{sudoku::Position, sudoku::Sudoku};

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
        assert!(Sudoku::is_present_in_block(
            &board.grid,
            &Position::new(0, 2),
            8
        ));
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
        assert!(Sudoku::is_present_in_row(
            &board.grid,
            &Position::new(1, 6),
            4
        ));
    }

    #[test]
    pub fn invalid_column() {
        let str_val = ",8,7,,5,,,,,4,9,7,,3,6,1,,,5,1,,9,8,2,,,4,,,,,,5,4,,6,7,,,,6,9,,1,,1,,,,4,,7,5,,2,,,8,1,3,6,,9,9,4,,,,7,,3,,,,,,,4,8,,7";

        let board = Sudoku::from_str(str_val);

        assert!(!board.is_ok());

        let board = board.expect("didn't expect an error");

        println!("{board}");

        assert!(!board.is_board_valid());
        assert!(Sudoku::is_present_in_column(
            &board.grid,
            &Position::new(2, 5),
            8
        ));

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
