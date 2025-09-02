use colored::Colorize;
use dashmap::DashSet;
use rand::Rng;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs::{File, OpenOptions},
    io::{self, BufRead, ErrorKind, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Sender},
    },
    thread,
};

const MAX_NUMBER_OF_RECORDS_IN_A_FILE: usize = 100_000;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum CellState {
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
type DietBoard = [u8; 81];

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
    completed_set: Arc<DashSet<DietBoard>>,
    tx: Sender<DataTxPacket>,
}

enum DataTxPacket {
    Valid(Sudoku),
    Invalid(DietBoard),
}

#[derive(Debug, Clone)]
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

        Ok(())
    }
}

impl Sudoku {
    const TOTAL_POSITIONS: usize = 81;

    pub fn get_grid(&self) -> Board {
        self.grid
    }

    pub fn get_prefilled_positions(&self) -> HashMap<Position, u8> {
        self.prefilled_positions.clone()
    }

    pub fn get_highlighted(&self) -> Option<u8> {
        self.highlighted
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
                    &[
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

impl Sudoku {
    pub fn generate_random_board(number_of_clues: u8) -> Option<Self> {
        let number_of_clues = number_of_clues.clamp(10, 80);
        Sudoku::random_board(&number_of_clues, None)
    }

    pub fn generate_random_boards(
        number_of_clues: u8,
        number_of_puzzles: usize,
    ) -> (Vec<Self>, usize) {
        let number_of_clues = number_of_clues.clamp(10, 80);

        // let num_threads = std::cmp::max(1, num_cpus::get().saturating_sub(1));
        // let num_threads = 1;
        let num_threads = num_cpus::get();

        let mut file_number = 0;

        let mut boards = vec![];
        let mut handlers = vec![];
        let found_counter = Arc::new(AtomicUsize::new(0));
        let total_seen_counter = Arc::new(AtomicUsize::new(0));

        let dashset: Arc<DashSet<DietBoard>> = Arc::new(DashSet::new());

        let mut invalid_inps = vec![];

        // fetch data from files and feed the dashset with invalid records
        loop {
            match Sudoku::read_lines(
                Sudoku::invalid_file_name(number_of_clues, file_number),
                |v| {
                    dashset.insert(v);
                },
            ) {
                Ok(v) => {
                    if !v {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("error reading lines: {e}");
                    return (vec![], 0);
                }
            }

            file_number += 1;
        }

        // also add all the valid puzzles to the set
        match Sudoku::read_lines(Sudoku::valid_file_name(number_of_clues), |v| {
            dashset.insert(v);
        }) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("error reading lines (valid puzzles): {e}");
                return (vec![], 0);
            }
        }

        total_seen_counter.store(dashset.len(), Ordering::Relaxed);

        let (tx, rx) = mpsc::channel::<DataTxPacket>();

        for _ in 0..num_threads {
            let tx_clone = tx.clone();
            let found_counter_clone = found_counter.clone();
            let total_seen_counter_clone = total_seen_counter.clone();
            let dashset_clone = dashset.clone();

            handlers.push(thread::spawn(move || {
                loop {
                    Sudoku::random_board(
                        &number_of_clues,
                        Some(RandomBoardsRequestArgs {
                            number_of_puzzles: number_of_puzzles,
                            number_of_found_counter: found_counter_clone.clone(),
                            total_number_of_puzzles_searched: total_seen_counter_clone.clone(),
                            completed_set: dashset_clone.clone(),
                            tx: tx_clone.clone(),
                        }),
                    );

                    if found_counter_clone.load(Ordering::Relaxed) >= number_of_puzzles as usize {
                        drop(tx_clone);
                        break;
                    }
                }
            }));
        }

        drop(tx);

        for m in rx {
            match m {
                DataTxPacket::Invalid(v) => {
                    invalid_inps.push(v);
                    if invalid_inps.len() >= MAX_NUMBER_OF_RECORDS_IN_A_FILE {
                        match Sudoku::export_to_file(
                            Sudoku::invalid_file_name(number_of_clues, file_number),
                            &invalid_inps,
                        ) {
                            Ok(_) => file_number += 1,
                            Err(e) => {
                                eprintln!("Error dumping to file. Error: {e}");
                                return (vec![], 0);
                            }
                        };
                        invalid_inps = vec![];
                    }
                }
                DataTxPacket::Valid(b) => {
                    Sudoku::append_to_file(Sudoku::valid_file_name(number_of_clues), &b)
                        .expect("error writting a valid puzzle to file");

                    boards.push(b);

                    found_counter.fetch_add(1, Ordering::Relaxed);
                }
            }

            print!(
                "\rProgress: {}/{}                   ",
                found_counter.load(Ordering::Relaxed),
                total_seen_counter.load(Ordering::Relaxed)
            );
            io::stdout().flush().unwrap();
        }

        for handler in handlers {
            handler.join().expect("error join the thread handler");
        }

        if invalid_inps.len() > 0 {
            match Sudoku::export_to_file(
                Sudoku::invalid_file_name(number_of_clues, file_number),
                &invalid_inps,
            ) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Error dumping to file. Error: {e}");
                    return (vec![], 0);
                }
            };
        }

        (boards, num_threads)
    }

    pub fn from_str(inp: &str) -> Result<Self, Box<dyn Error>> {
        let mut inp = inp.to_string();

        if inp.contains(".") {
            inp = Sudoku::from_thonky_str(&inp);
        }

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
                        &[
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

            let diet_grid = Sudoku::get_diet_board(&grid);

            if let Some(cri) = conditonal_run_info.clone() {
                if !cri.completed_set.insert(diet_grid) {
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
                            &[
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

                if let Some(cri) = conditonal_run_info.clone() {
                    cri.tx
                        .send(DataTxPacket::Valid(board.clone()))
                        .expect("error sending on channel");
                };

                return Some(board);
            }

            if let Some(cri) = conditonal_run_info.clone() {
                cri.tx
                    .send(DataTxPacket::Invalid(diet_grid))
                    .expect("error send data on thread");
            };
        }
    }

    #[inline]
    fn get_diet_board(board: &Board) -> DietBoard {
        let mut v: DietBoard = [0; 81];
        let mut counter = 0;

        for i in board {
            for j in i {
                match j.0 {
                    None => (),
                    Some(k) => v[counter] = k,
                }

                counter += 1;
            }
        }

        v
    }

    /// returns true if there is a conflict
    fn check_for_conflict(maps: &[(&[u16; 9], usize); 3], v: u8) -> bool {
        maps.iter().any(|(m, i)| (m[*i] & (1u16 << v)) != 0)
    }

    fn insert_into_bitmap(map: &mut [u16; 9], idx: usize, v: u8) {
        map[idx] |= 1 << v;
    }

    fn get(&self, pos: &Position) -> Option<u8> {
        self.grid[pos.x][pos.y].0
    }

    #[inline]
    fn invalid_file_name(number_of_clues: u8, file_number: i32) -> String {
        format!("clues_{number_of_clues}/invalid_{number_of_clues}_{file_number}")
    }

    #[inline]
    fn valid_file_name(number_of_clues: u8) -> String {
        format!("clues_{number_of_clues}/valid_puzzles_{number_of_clues}")
    }

    fn read_lines<P, F>(filename: P, process_line: F) -> Result<bool, Box<dyn Error>>
    where
        P: AsRef<Path>,
        F: Fn(DietBoard),
    {
        let file = match File::open(filename) {
            Ok(r) => r,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound || e.kind() == ErrorKind::InvalidFilename {
                    return Ok(false);
                }

                return Err(e.into());
            }
        };
        let reader = io::BufReader::new(file);

        for line_result in reader.lines() {
            let line = line_result?;
            process_line(Sudoku::thonky_to_diet_board(&line)?);
        }

        Ok(true)
    }

    fn export_to_file<P>(filename: P, lines: &Vec<DietBoard>) -> Result<bool, Box<dyn Error>>
    where
        P: AsRef<Path>,
    {
        let mut file = File::create(filename)?;

        for line in lines {
            file.write_all(Sudoku::diet_board_to_thonky(&line).unwrap().as_bytes())?;
            file.write_all(b"\n")?;
        }

        Ok(true)
    }

    fn append_to_file<P>(filename: P, board: &Sudoku) -> Result<bool, Box<dyn Error>>
    where
        P: AsRef<Path>,
    {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(filename)?;

        let str_to_append = board.to_thonky_str();

        // Write the data to the end of the file.
        file.write_all(str_to_append.as_bytes())?;
        file.write_all(b"\n")?;

        Ok(true)
    }

    fn thonky_to_diet_board(s: &str) -> Result<DietBoard, String> {
        if s.len() != 81 {
            return Err(format!(
                "Input string must be 81 characters long, but it is {}",
                s.len()
            ));
        }

        let mut vec: Vec<u8> = Vec::with_capacity(81);
        for c in s.chars() {
            if c == '.' {
                vec.push(0);
            } else if let Some(digit) = c.to_digit(10) {
                vec.push(digit as u8);
            } else {
                return Err(format!("Invalid character found: {}", c));
            }
        }

        vec.try_into()
            .map_err(|v: Vec<u8>| format!("Expected a Vec of length 81, but got {}", v.len()))
    }

    fn diet_board_to_thonky(board: &DietBoard) -> Result<String, String> {
        let mut result = String::with_capacity(81);

        for &value in board.iter() {
            match value {
                0 => result.push('.'),
                1..=9 => {
                    let digit_char = std::char::from_digit(value as u32, 10)
                        .ok_or_else(|| format!("Invalid digit value: {}", value))?;
                    result.push(digit_char);
                }
                _ => {
                    return Err(format!("Invalid value in array: {}", value));
                }
            }
        }

        Ok(result)
    }

    fn from_thonky_str(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            if c == '.' {
                result.push(',');
            } else {
                result.push(c);
            }
        }
        result
    }
}
