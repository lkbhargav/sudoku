use colored::Colorize;
use dashmap::DashMap;
use rand::Rng;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
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

enum UpdateMapsType {
    Add,
    Remove,
}

type Board = [[(Option<u8>, CellState); 9]; 9];

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

#[derive(Debug, Clone)]
struct RandomBoardsRequestArgs {
    number_of_puzzles: usize,
    number_of_found_counter: Arc<AtomicUsize>,
    total_number_of_puzzles_searched: Arc<AtomicUsize>,
    completed_map: Arc<DashMap<Board, bool>>,
}

#[derive(Debug)]
pub struct Sudoku {
    grid: Board,
    prefilled_positions: HashMap<Position, u8>,
    solved_grid: Board,
    highlighted: Option<u8>,
    rows: [u16; 9],
    columns: [u16; 9],
    blocks: [u16; 9],
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
                                    if j.1.1 == CellState::Wrong {
                                        val = val.on_bright_yellow().red().bold();
                                    } else {
                                        val = val.on_bright_yellow().green().bold();
                                    }
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

    fn random_board(
        number_of_clues: &u8,
        conditonal_run_info: Option<RandomBoardsRequestArgs>,
    ) -> Option<Self> {
        let mut rng = rand::rng();

        'outer: loop {
            let mut grid: Board = [[(None, CellState::Normal); 9]; 9];

            let mut blocks: [u16; 9] = [0; 9];
            let mut rows: [u16; 9] = [0; 9];
            let mut columns: [u16; 9] = [0; 9];

            if let Some(cri) = conditonal_run_info.clone() {
                if cri.number_of_found_counter.load(Ordering::Relaxed) >= cri.number_of_puzzles {
                    return None;
                }
            };

            for i in 0..9 {
                for j in 0..9 {
                    let bid = Sudoku::get_block_id(i, j);
                    let mut counter = 0;

                    loop {
                        let val = rng.random_range(1..=9) as u8;

                        if (blocks[bid] & (1 << val)) != 0
                            || (rows[i] & (1 << val)) != 0
                            || (columns[j] & (1 << val)) != 0
                        {
                            counter += 1;
                            if counter >= 20 {
                                continue 'outer;
                            }

                            continue;
                        }

                        grid[i][j].0 = Some(val);
                        blocks[bid] |= 1 << val;
                        rows[i] |= 1 << val;
                        columns[j] |= 1 << val;

                        break;
                    }
                }
            }

            let mut number_of_removals = 81 - number_of_clues;

            while number_of_removals > 0 {
                let x = rng.random_range(0..9);
                let y = rng.random_range(0..9);

                if grid[x][y].0 != None {
                    grid[x][y].0 = None;
                    number_of_removals -= 1;
                }
            }

            if let Some(cri) = conditonal_run_info.clone() {
                if cri.completed_map.insert(grid, true).is_some() {
                    continue;
                }
            };

            let mut prefilled_positions = HashMap::new();

            let mut blocks = [0; 9];
            let mut columns = [0; 9];
            let mut rows = [0; 9];

            for i in grid.iter().enumerate() {
                for j in i.1.iter().enumerate() {
                    if j.1.0.is_some() {
                        let val = j.1.0.unwrap();
                        prefilled_positions.insert(Position::new(i.0, j.0), val);

                        if Sudoku::check_for_conflict(
                            vec![
                                (&blocks, Sudoku::get_block_id(i.0, j.0)),
                                (&rows, i.0),
                                (&columns, j.0),
                            ],
                            val,
                        ) {
                            continue 'outer;
                        }

                        Sudoku::insert_into_bitmap(&mut rows, i.0, val);
                        Sudoku::insert_into_bitmap(&mut columns, j.0, val);
                        Sudoku::insert_into_bitmap(
                            &mut blocks,
                            Sudoku::get_block_id(i.0, j.0),
                            val,
                        );
                    }
                }
            }

            // println!("{:?}", rows);
            // println!("{:?}", columns);
            // println!("{:?}", blocks);

            let mut board = Self {
                grid: grid.clone(),
                prefilled_positions,
                solved_grid: grid,
                highlighted: None,
                rows,
                columns,
                blocks,
            };

            // println!("{board}");
            // println!("\n{}\n{}\n\n", board.to_thonky_str(), board.to_str());

            // println!("board.is_puzzle_valid(): {}", board.is_puzzle_valid());

            if let Some(cri) = conditonal_run_info.clone() {
                cri.total_number_of_puzzles_searched
                    .fetch_add(1, Ordering::Relaxed);
            };

            if board.solve() {
                board.reset();
                return Some(board);
            }

            if let Some(cri) = conditonal_run_info.clone() {
                print!(
                    "\rProgress: {}/{}                   ",
                    cri.number_of_found_counter.load(Ordering::Relaxed),
                    cri.total_number_of_puzzles_searched.load(Ordering::Relaxed)
                );
                io::stdout().flush().unwrap();
            };
        }
    }

    pub fn generate_random_board(number_of_clues: u8) -> Option<Self> {
        let mut number_of_clues = number_of_clues;

        if number_of_clues < 10 {
            number_of_clues = 10;
        } else if number_of_clues > 80 {
            number_of_clues = 80;
        }

        Sudoku::random_board(&number_of_clues, None)
    }

    pub fn generate_random_boards(
        number_of_clues: u8,
        number_of_puzzles: usize,
    ) -> (Vec<Self>, usize) {
        let num_threads = num_cpus::get() - 1;

        let mut boards = vec![];
        let mut handlers = vec![];
        let found_counter = Arc::new(AtomicUsize::new(0));
        let total_seen_counter = Arc::new(AtomicUsize::new(0));

        let dashmap: Arc<DashMap<Board, bool>> = Arc::new(DashMap::new());

        let (tx, rx) = mpsc::channel::<(Sudoku, Duration)>();

        for _ in 0..num_threads {
            let tx_clone = tx.clone();
            let found_counter_clone = found_counter.clone();
            let total_seen_counter_clone = total_seen_counter.clone();
            let dashmap_clone = dashmap.clone();

            handlers.push(thread::spawn(move || {
                loop {
                    let now = Instant::now();

                    let board = Sudoku::random_board(
                        &number_of_clues,
                        Some(RandomBoardsRequestArgs {
                            number_of_puzzles: number_of_puzzles,
                            number_of_found_counter: found_counter_clone.clone(),
                            total_number_of_puzzles_searched: total_seen_counter_clone.clone(),
                            completed_map: dashmap_clone.clone(),
                        }),
                    );

                    if let Some(b) = board {
                        tx_clone
                            .send((b, now.elapsed()))
                            .expect("error sending on channel");

                        found_counter_clone.fetch_add(1, Ordering::Relaxed);
                    };

                    print!(
                        "\rProgress: {}/{}                   ",
                        found_counter_clone.load(Ordering::Relaxed),
                        total_seen_counter_clone.load(Ordering::Relaxed)
                    );
                    io::stdout().flush().unwrap();

                    if found_counter_clone.load(Ordering::Relaxed) >= number_of_puzzles as usize {
                        drop(tx_clone);
                        break;
                    }
                }
            }));
        }

        drop(tx);

        for m in rx {
            boards.push(m.0);
        }

        for handler in handlers {
            handler.join().expect("error join the thread handler");
        }

        (boards, num_threads)
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

        let mut blocks = [0; 9];
        let mut columns = [0; 9];
        let mut rows = [0; 9];

        let res: Board = list
            .chunks(9)
            .map(|chunk| chunk.try_into().expect("error chunking the input given"))
            .collect::<Vec<_>>()
            .try_into()
            .expect("given board is not of valid lengths");

        for i in res.iter().enumerate() {
            for j in i.1.iter().enumerate() {
                if j.1.0.is_some() {
                    let val = j.1.0.unwrap();

                    if Sudoku::check_for_conflict(
                        vec![
                            (&blocks, Sudoku::get_block_id(i.0, j.0)),
                            (&rows, i.0),
                            (&columns, j.0),
                        ],
                        val,
                    ) {
                        return Err("duplicate value found in row block or column".into());
                    }

                    Sudoku::insert_into_bitmap(&mut rows, i.0, val);
                    Sudoku::insert_into_bitmap(&mut columns, j.0, val);
                    Sudoku::insert_into_bitmap(&mut blocks, Sudoku::get_block_id(i.0, j.0), val);
                }
            }
        }

        let mut sudoku = Sudoku {
            grid: res.clone(),
            prefilled_positions,
            solved_grid: res,
            highlighted: None,
            blocks,
            rows,
            columns,
        };

        if sudoku.solve() {
            sudoku.reset();
            return Ok(sudoku);
        }

        Err("invalid board given".into())
    }

    pub fn to_thonky_str(&self) -> String {
        let mut resp = String::with_capacity(81);

        for i in &mut self.grid.iter().enumerate() {
            for j in i.1.iter().enumerate() {
                match j.1.0 {
                    Some(k) => resp.push_str(&k.to_string()),
                    None => resp.push_str("."),
                }
            }
        }

        assert_eq!(resp.len(), 81);

        resp
    }

    pub fn to_str(&self) -> String {
        let mut resp = String::new();

        for i in &mut self.grid.iter().enumerate() {
            for j in i.1.iter().enumerate() {
                if let Some(k) = j.1.0 {
                    if !self
                        .prefilled_positions
                        .contains_key(&Position::new(i.0, j.0))
                    {
                        resp.push('u');
                    };

                    resp.push_str(&k.to_string());
                }

                if !(i.0 + 1 >= self.grid.len() && j.0 + 1 >= self.grid[0].len()) {
                    resp.push_str(",");
                }
            }
        }

        resp
    }

    pub fn is_board_solved_completely(&self) -> bool {
        for b in self.blocks {
            if b.count_ones() != 9 {
                return false;
            }
        }

        true
    }

    pub fn number_of_initial_clues(&self) -> u8 {
        self.prefilled_positions.len() as u8
    }

    #[inline]
    fn get_block_id(row: usize, col: usize) -> usize {
        (row / 3) * 3 + (col / 3)
    }

    #[inline(always)]
    fn update_maps(
        &mut self,
        pos: &Position,
        v: u8,
        op_type: UpdateMapsType,
    ) -> Result<(), Box<dyn Error>> {
        let bid = Sudoku::get_block_id(pos.x, pos.y);
        match op_type {
            UpdateMapsType::Remove => {
                self.blocks[bid] &= !(1 << v);
                self.rows[pos.x] &= !(1 << v);
                self.columns[pos.y] &= !(1 << v);
            }
            UpdateMapsType::Add => {
                if Sudoku::check_for_conflict(
                    vec![
                        (&self.blocks, bid),
                        (&self.rows, pos.x),
                        (&self.columns, pos.y),
                    ],
                    v,
                ) {
                    return Err("given value is already present".into());
                }

                Sudoku::insert_into_bitmap(&mut self.blocks, bid, v);
                Sudoku::insert_into_bitmap(&mut self.rows, pos.x, v);
                Sudoku::insert_into_bitmap(&mut self.columns, pos.y, v);
            }
        }

        Ok(())
    }

    /// returns true if there is a conflict
    fn check_for_conflict(maps: Vec<(&[u16; 9], usize)>, v: u8) -> bool {
        for m in maps {
            if (m.0[m.1] & (1 << v)) != 0 {
                return true;
            }
        }

        return false;
    }

    fn insert_into_bitmap(map: &mut [u16; 9], idx: usize, v: u8) {
        map[idx] |= 1 << v;
    }

    #[inline(always)]
    fn insert(
        &mut self,
        pos: &Position,
        val: Option<u8>,
        cell_state: CellState,
    ) -> Result<(), Box<dyn Error>> {
        let exisiting_val = self.grid[pos.x][pos.y];

        match exisiting_val.0 {
            Some(v) => {
                self.update_maps(pos, v, UpdateMapsType::Remove)
                    .expect("removal shouldn't trigger an error");
            }
            None => (),
        };

        match val {
            None => (),
            Some(v) => {
                if self.update_maps(pos, v, UpdateMapsType::Add).is_err() {
                    return Err("value is already present".into());
                }
            }
        };

        self.grid[pos.x][pos.y] = (val, cell_state);

        Ok(())
    }

    pub fn insert_at(&mut self, pos: &Position, val: Option<u8>) -> InsertStatus {
        let mut cell_state = CellState::Normal;

        let mut resp = InsertStatus::Right;

        if val.is_some() {
            if self.grid[pos.x][pos.y].0.is_some() {
                return InsertStatus::ValuePresent;
            }

            if self.solved_grid[pos.x][pos.y].0 != val {
                cell_state = CellState::Wrong;
                resp = InsertStatus::Wrong;
            }

            if self.highlighted.is_some() {
                if self.highlighted != val {
                    self.highlight(val);
                }
            }
        }

        if self.insert(pos, val, cell_state).is_err() {
            return InsertStatus::ValuePresent;
        }

        resp
    }

    pub fn hint(&mut self, pos: &Position) -> HintStatus {
        if self.grid[pos.x][pos.y].0.is_some() {
            return HintStatus::ValuePresent;
        }

        self.grid[pos.x][pos.y] = (self.solved_grid[pos.x][pos.y].0, CellState::Hinted);

        HintStatus::Ok
    }

    pub fn highlight(&mut self, val: Option<u8>) {
        if val.is_none() {
            self.highlighted = None;
            return;
        }

        if self.highlighted == val {
            self.highlighted = None;
        } else {
            self.highlighted = val;
        }
    }

    fn get(&self, pos: &Position) -> Option<u8> {
        self.grid[pos.x][pos.y].0
    }

    pub fn fetch_next_empty_cell(&self) -> Option<Position> {
        let mut max_filled = 0;
        let mut pos = None;

        for i in 0..9 {
            for j in 0..9 {
                if self.grid[i][j].0.is_none() {
                    let sum = self.rows[i].count_ones()
                        + self.columns[j].count_ones()
                        + self.blocks[Sudoku::get_block_id(i, j)].count_ones();
                    if sum > max_filled {
                        max_filled = sum;

                        pos = Some(Position::new(i, j));
                    }
                }
            }
        }

        pos
    }

    pub fn solve(&mut self) -> bool {
        let mut filled_stack = vec![];
        // let mut empty_cells_stack = self.fetch_empty_cells();
        let mut reached_dead_end = false;

        let mut solutions = 0;
        let mut initial_solution = String::new();

        loop {
            let next_empty_cell = self.fetch_next_empty_cell();

            // if no other way to go
            if next_empty_cell.is_none() && filled_stack.is_empty() {
                return false;
            }

            if next_empty_cell.is_none() && !reached_dead_end {
                solutions += 1;

                self.solved_grid = self.grid;

                if solutions >= 2 {
                    if !initial_solution.is_empty() {
                        if initial_solution == self.to_str() {
                            return true;
                        }
                    }

                    self.solved_grid = [[(None, CellState::Normal); 9]; 9];
                    return false;
                }

                initial_solution = self.to_str();
                reached_dead_end = true;
            }

            // println!("{self}");
            // println!("{}", self.to_str());

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
                            self.insert(&filled_pos, None, CellState::Normal)
                                .expect("this is removal");
                            // empty_cells_stack.push(filled_pos.clone());
                        }

                        for i in v + 1..=9 {
                            if self.insert(&filled_pos, Some(i), CellState::Normal).is_ok() {
                                filled_stack.push(filled_pos.clone());
                                reached_dead_end = false;
                                break;
                            }

                            if i == 9 {
                                self.insert(&filled_pos, None, CellState::Normal)
                                    .expect("this is removal");
                                // empty_cells_stack.push(filled_pos.clone());
                            }
                        }
                    }
                    None => return false,
                }
            } else {
                if next_empty_cell.is_none() {
                    break;
                }

                let empty_pos = next_empty_cell.unwrap();

                for i in 1..=9 {
                    if self.insert(&empty_pos, Some(i), CellState::Normal).is_ok() {
                        filled_stack.push(empty_pos.clone());
                        break;
                    }

                    if i == 9 {
                        if filled_stack.is_empty() {
                            return solutions == 1;
                        }

                        self.insert(&empty_pos, None, CellState::Normal)
                            .expect("this is removal");
                        // empty_cells_stack.push(empty_pos.clone());
                        reached_dead_end = true;
                    }
                }
            }
        }

        self.is_board_solved_completely()
    }

    pub fn reset(&mut self) {
        for i in &mut self.grid.clone().iter().enumerate() {
            for j in i.1.iter().enumerate() {
                let pos = Position::new(i.0, j.0);
                if !(self.prefilled_positions.contains_key(&pos)
                    || self.grid[i.0][j.0].1 == CellState::UserMarkedDefault)
                {
                    match self.grid[i.0][j.0].0 {
                        Some(v) => {
                            self.update_maps(&pos, v, UpdateMapsType::Remove)
                                .expect("removal doesn't trigger error");
                        }
                        None => (),
                    }

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
                    match self.grid[i.0][j.0].0 {
                        Some(v) => {
                            self.update_maps(&pos, v, UpdateMapsType::Remove)
                                .expect("removal doesn't trigger error");
                        }
                        None => (),
                    }

                    self.grid[i.0][j.0].0 = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{sudoku::Position, sudoku::Sudoku};

    // #[test]
    // pub fn simple_test() {
    //     let str_val = "8,2,,,,,,,,,,4,,,,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

    //     let board = Sudoku::from_str(str_val);

    //     assert!(board.is_ok());

    //     let mut board = board.expect("didn't expect an error");

    //     println!("{}", board);

    //     assert_eq!(board.prefilled_positions.len(), 9);

    //     for pos in vec![
    //         Position::new(0, 0),
    //         Position::new(1, 2),
    //         Position::new(2, 2),
    //         Position::new(5, 0),
    //         Position::new(6, 6),
    //         Position::new(8, 4),
    //     ] {
    //         assert!(board.prefilled_positions.contains_key(&pos));
    //     }

    //     assert!(!board.prefilled_positions.contains_key(&Position::new(1, 5)));
    //     assert!(!board.prefilled_positions.contains_key(&Position::new(5, 8)));

    //     assert!(board.is_board_valid());

    //     assert!(!board.is_puzzle_valid());
    // }

    // #[test]
    // pub fn invalid_block() {
    //     let str_val = "8,2,8,,,,,,,,,4,,,,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

    //     let board = Sudoku::from_str(str_val);

    //     assert!(board.is_ok());

    //     let board = board.expect("didn't expect an error");

    //     assert_eq!(board.prefilled_positions.len(), 10);
    //     assert!(board.prefilled_positions.contains_key(&Position::new(0, 0)));
    //     assert!(board.prefilled_positions.contains_key(&Position::new(1, 2)));
    //     assert!(!board.prefilled_positions.contains_key(&Position::new(1, 5)));

    //     assert!(!board.is_board_valid());
    //     assert!(Sudoku::is_present_in_block(
    //         &board.grid,
    //         &Position::new(0, 2),
    //         8
    //     ));
    // }

    // #[test]
    // pub fn invalid_row() {
    //     let str_val = "8,2,,,,,,,,,,4,,,4,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

    //     let board = Sudoku::from_str(str_val);

    //     assert!(board.is_ok());

    //     let board = board.expect("didn't expect an error");

    //     assert_eq!(board.prefilled_positions.len(), 10);
    //     assert!(board.prefilled_positions.contains_key(&Position::new(0, 0)));
    //     assert!(board.prefilled_positions.contains_key(&Position::new(1, 2)));
    //     assert!(!board.prefilled_positions.contains_key(&Position::new(1, 6)));

    //     assert!(!board.is_board_valid());
    //     assert!(Sudoku::is_present_in_row(
    //         &board.grid,
    //         &Position::new(1, 6),
    //         4
    //     ));
    // }

    // #[test]
    // pub fn invalid_column() {
    //     let str_val = ",8,7,,5,,,,,4,9,7,,3,6,1,,,5,1,,9,8,2,,,4,,,,,,5,4,,6,7,,,,6,9,,1,,1,,,,4,,7,5,,2,,,8,1,3,6,,9,9,4,,,,7,,3,,,,,,,4,8,,7";

    //     let board = Sudoku::from_str(str_val);

    //     assert!(!board.is_ok());

    //     let board = board.expect("didn't expect an error");

    //     println!("{board}");

    //     assert!(!board.is_board_valid());
    //     assert!(Sudoku::is_present_in_column(
    //         &board.grid,
    //         &Position::new(2, 5),
    //         8
    //     ));

    //     dbg!(board);
    // }

    // #[test]
    // pub fn toughest_valid() {
    //     let str_val = ",,,1,,2,,,,,6,,,,,,7,,,,8,,,,9,,,4,,,,,,,,3,,5,,,,7,,,,2,,,,8,,,,1,,,9,,,,8,,5,,7,,,,,,6,,,,,3,,4,,,";

    //     let board = Sudoku::from_str(str_val);

    //     assert!(board.is_ok());

    //     let board = board.expect("didn't expect an error");

    //     assert_eq!(board.prefilled_positions.len(), 20);
    //     assert!(board.prefilled_positions.contains_key(&Position::new(0, 3)));
    //     assert!(board.prefilled_positions.contains_key(&Position::new(1, 1)));
    //     assert!(!board.prefilled_positions.contains_key(&Position::new(2, 5)));

    //     assert!(board.is_board_valid());
    // }

    // #[test]
    // pub fn extreme_26() {
    //     let str_val = "1,,,,6,,,,,,,3,9,,1,,4,,2,,,,,,,,7,,,,,8,,,5,,,,6,,4,,,,,3,,,5,,6,2,,,,,1,3,,5,,9,,,,,8,,,,,,,9,,,,,4,,";

    //     let board = Sudoku::from_str(str_val);

    //     assert!(board.is_ok());

    //     let mut board = board.expect("didn't expect an error");

    //     println!("{}", board);

    //     assert_eq!(board.prefilled_positions.len(), 23);

    //     assert!(board.is_board_valid());
    //     assert!(board.solve(None));
    //     assert!(board.is_puzzle_valid());
    // }
}
