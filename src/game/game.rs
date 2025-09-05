use std::{
    io::{self, Write},
    process::exit,
    time::Instant,
};

use colored::Colorize;
use humantime::format_duration;

use crate::{
    game::types::{MainSelection, Message, MessageType, UserRequest},
    sudoku::{CellState, HintStatus, InsertStatus, Position, Sudoku},
    util::{confirm, prompt, prompt_select},
};

#[derive(Default)]
pub struct Game {
    board: Option<Sudoku>,
    starting_clues: u8,
    main_selection: MainSelection,
    mistakes: u8,
    additional_clues: u8,
    undo_buffer: Vec<(Position, Option<u8>)>,
    redo_buffer: Vec<(Position, Option<u8>)>,
    initital_board_layout: String,
}

impl Game {
    pub fn start_game(&mut self) {
        loop {
            let main_selection_options = vec![
                MainSelection::New,
                MainSelection::Load,
                MainSelection::Generate,
                MainSelection::Exit,
            ];

            let main_selection = prompt_select(
                "Select one of the following options",
                &main_selection_options,
            );

            self.main_selection = main_selection_options[main_selection].clone();

            match main_selection_options[main_selection] {
                MainSelection::Load => {
                    let str = prompt("Paste the puzzle input to load", "");

                    if str.is_empty() {
                        println!("expected puzzle input but found empty string");
                        continue;
                    }

                    let board = match Sudoku::from_str(&str) {
                        Ok(b) => b,
                        Err(e) => {
                            println!(
                                "invalid puzzle input loaded, please fix that and try again later: {}",
                                e.to_string()
                            );
                            continue;
                        }
                    };

                    self.set_board(board);
                    self.game_loop();
                }
                MainSelection::New => {
                    let clues = prompt("How many clues do you want in the puzzle?", "40");

                    let clues = match clues.parse::<u8>() {
                        Ok(c) => c,
                        Err(e) => {
                            println!("expected a number but found characters: {}", e.to_string());
                            continue;
                        }
                    };

                    // clears the board completely
                    self.hard_reset();

                    let board = Sudoku::generate_random_board(clues, |c| {
                        print!("\rFiltered: {c}");
                        io::stdout().flush().unwrap();
                    });

                    self.set_board(board.unwrap());
                    self.game_loop();
                }
                MainSelection::Generate => {
                    let clues = prompt("How many clues do you want to have?", "40");

                    let clues = match clues.parse::<u8>() {
                        Ok(c) => c,
                        Err(e) => {
                            println!("expected a number but found characters: {}", e.to_string());
                            continue;
                        }
                    };

                    let number_of_boards =
                        prompt("How many boards do you want to generate?", "100");

                    let number_of_boards = match number_of_boards.parse::<usize>() {
                        Ok(c) => c,
                        Err(e) => {
                            println!("expected a number but found characters: {}", e.to_string());
                            continue;
                        }
                    };

                    let just_print = confirm("Do you want to just print it here?", true);

                    let boards =
                        Sudoku::generate_random_boards(clues, number_of_boards, just_print);

                    println!("\n\nUnqiue and valid boards");

                    for board in &boards.0 {
                        println!("{}", board.to_thonky_str());
                    }

                    println!("\nBoards ({} with {} threads)", boards.0.len(), boards.1);
                }
                MainSelection::Exit => exit(1),
            }
        }
    }

    fn set_board(&mut self, board: Sudoku) {
        self.initital_board_layout = board.to_str().into();
        self.starting_clues = board.number_of_initial_clues();
        self.board = Some(board);
    }

    fn game_loop(&mut self) {
        match &self.board {
            None => {
                return;
            }
            Some(_) => (),
        };

        let mut give_up = false;
        let mut msg;
        let mut message: Option<Message> = None;
        let start_time = Instant::now();
        let mut won = false;

        loop {
            if self.board.as_mut().unwrap().is_board_solved_completely() {
                if !give_up {
                    if self.mistakes > 0 {
                        msg = format!(
                            "{}\nTime taken: {}\n\n{}",
                            "Even though you made some mistake(s) you made it. Congragulations!",
                            format_duration(start_time.elapsed()),
                            self.initital_board_layout
                        );
                    } else {
                        msg = format!(
                            "{}\nTime taken: {}\n\n{}",
                            "Congragulations!",
                            format_duration(start_time.elapsed()),
                            self.initital_board_layout
                        );
                    }

                    message = Some(Message::new(&msg, MessageType::Success));
                }
                won = true;
            }

            self.draw(&message);
            message = None;

            // end of the puzzle
            if won {
                break;
            }

            let ans = prompt(
                "Enter your guess (ex: g007 - means fill grid location 0 (x), 0 (y) with 7)",
                "",
            );

            let v = match UserRequest::parse(&ans) {
                Ok(v) => v,
                Err(e) => {
                    msg = format!("Error parsing your request: {}", e.to_string());
                    message = Some(Message::new(&msg, MessageType::Error));
                    continue;
                }
            };

            match v {
                UserRequest::Guess(pos, val) => {
                    match self.board.as_mut().unwrap().insert_at(&pos, Some(val)) {
                        InsertStatus::Wrong => {
                            self.mistakes += 1;
                            message = Some(Message::new("Value doesn't fit in this cell, please try again".into(), MessageType::Error));
                        },
                        InsertStatus::ValuePresent =>
                            message = Some(Message::new("Value is already present in the cell/block/row/column, try clearing the cell before inserting a new value or force insert".into(), MessageType::Warn)),
                        _ => (),
                    };

                    self.undo_buffer.push((pos.clone(), Some(val)));
                    self.redo_buffer.clear();
                }
                UserRequest::RemoveGuess(pos) => {
                    match self.board.as_mut().unwrap().insert_at(&pos, None) {
                        InsertStatus::ValuePresent =>
                            message = Some(Message::new("Please check the position that you are trying to remove at. Maybe it's not filled to begin with".into(), MessageType::Warn)),
                        _ => (),
                    }
                }
                UserRequest::Undo => {
                    if self.undo_buffer.is_empty() {
                        message = Some(Message::new(
                            "You can't use the undo option as there is no known previous move"
                                .into(),
                            MessageType::Warn,
                        ));
                        continue;
                    }

                    let pp = self.undo_buffer.pop().unwrap();

                    self.board.as_mut().unwrap().insert_at(&pp.0, None);
                    self.redo_buffer.push(pp);
                }
                UserRequest::Redo => {
                    if self.redo_buffer.is_empty() {
                        message = Some(Message::new(
                            "You can't use the redo option as there is nothing undid yet".into(),
                            MessageType::Warn,
                        ));
                        continue;
                    }

                    let pp = self.redo_buffer.pop().unwrap();

                    self.board.as_mut().unwrap().insert_at(&pp.0, pp.1);
                    self.undo_buffer.push(pp);
                }
                UserRequest::Hint(pos) => {
                    match self.board.as_mut().unwrap().hint(&pos) {
                        HintStatus::ValuePresent => {
                            message = Some(Message::new(
                                "Hint requested on already filled cell".into(),
                                MessageType::Warn,
                            ))
                        }
                        HintStatus::Ok => self.additional_clues += 1,
                    }
                    continue;
                }
                UserRequest::Highlight(v) => {
                    self.board.as_mut().unwrap().highlight(Some(v));
                }
                UserRequest::RemoveHighlight => {
                    self.board.as_mut().unwrap().highlight(None);
                }
                UserRequest::ShareOriginal => {
                    let v = &self.initital_board_layout;
                    message = Some(Message::new(v, MessageType::Success));
                }
                UserRequest::ShareCurrentState => {
                    msg = self.board.as_mut().unwrap().to_str();
                    message = Some(Message::new(&msg, MessageType::Success));
                }
                UserRequest::ShareThonkyVersion => {
                    msg = self.board.as_mut().unwrap().to_thonky_str();
                    message = Some(Message::new(&msg, MessageType::Success));
                }
                UserRequest::TimeElapsed => {
                    msg = format!("Time elapsed: {}", format_duration(start_time.elapsed()));
                    message = Some(Message::new(&msg, MessageType::Normal));
                }
                UserRequest::Reset => {
                    self.reset();
                }
                UserRequest::HardReset => {
                    self.hard_reset();
                }
                UserRequest::Giveup => {
                    let b = self.board.as_mut().unwrap();
                    b.reset();
                    b.solve();
                    give_up = true;

                    message = Some(Message::new(
                        &self.initital_board_layout,
                        MessageType::Highlight,
                    ));
                }
                UserRequest::Exit => {
                    break;
                }
            }
        }
    }

    fn draw(&self, message: &Option<Message>) {
        let board = match &self.board {
            None => {
                println!("No board to render!");
                return;
            }
            Some(b) => b,
        };

        let mut instructions = Game::get_instructions();

        // clears the screen without a scrollbar
        print!("{esc}c", esc = 27 as char);

        println!(
            "Initial clues: {} {} # mistakes: {} {} # hints: {}\n",
            self.starting_clues.to_string().bold(),
            "|".white().bold(),
            self.mistakes.to_string().red().bold(),
            "|".white().bold(),
            self.additional_clues.to_string().magenta().bold()
        );

        let highlighted = board.get_highlighted();
        let mut board_str = String::with_capacity(1500);

        for i in &mut board.get_grid().iter().enumerate() {
            if i.0 == 0 {
                board_str.push_str(&format!(
                    "{}",
                    "    0  1  2   3  4  5   6  7  8 \n".italic()
                ));
                board_str.push_str(&format!(
                    "{}         {}\n",
                    "   -----------------------------".blue(),
                    instructions.pop().unwrap_or_default()
                ));
            }

            board_str.push_str(&format!("{} {}", i.0.to_string().italic(), "|".blue()));

            for j in i.1.iter().enumerate() {
                match j.1.0 {
                    Some(v) => {
                        if board
                            .get_prefilled_positions()
                            .contains_key(&Position::new(i.0, j.0))
                        {
                            let mut val = v.to_string().bold();
                            if highlighted.is_some() {
                                if j.1.0.unwrap() == highlighted.unwrap() {
                                    val = v.to_string().on_bright_yellow().green().bold();
                                }
                            }

                            board_str.push_str(&format!(" {} ", val));
                        } else {
                            let mut val = match j.1.1 {
                                CellState::Hinted => v.to_string().magenta().bold(),
                                CellState::Wrong => v.to_string().red().bold(),
                                CellState::UserMarkedDefault => v.to_string().yellow().bold(),
                                _ => v.to_string().green(),
                            };

                            if highlighted.is_some() {
                                if j.1.0.unwrap() == highlighted.unwrap() {
                                    if j.1.1 == CellState::Wrong {
                                        val = val.on_bright_yellow().red().bold();
                                    } else {
                                        val = val.on_bright_yellow().green().bold();
                                    }
                                }
                            }

                            board_str.push_str(&format!(" {} ", val));
                        }
                    }
                    None => {
                        board_str.push_str("   ");
                    }
                }

                if (j.0 + 1) % 3 == 0 {
                    board_str.push_str(&format!("{}", "|".blue()));
                }
            }

            board_str.push_str(&format!(
                "        {}\n",
                instructions.pop().unwrap_or_default()
            ));

            if (i.0 + 1) % 3 == 0 {
                board_str.push_str(&format!(
                    "{}         {}\n",
                    "   -----------------------------".blue(),
                    instructions.pop().unwrap_or_default()
                ));
            }
        }

        println!("{board_str}");

        match message {
            None => println!("\n"),
            Some(m) => {
                let m = match m.get_type() {
                    MessageType::Error => m.get_msg().bold().red(),
                    MessageType::Warn => m.get_msg().bold().yellow(),
                    MessageType::Success => m.get_msg().bold().green(),
                    MessageType::Normal => m.get_msg().normal(),
                    MessageType::Highlight => m.get_msg().bold(),
                };

                println!("{}\n", m);
            }
        }
    }

    fn _r(&mut self) {
        self.additional_clues = 0;
        self.mistakes = 0;
        self.undo_buffer.clear();
        self.redo_buffer.clear();
    }

    fn reset(&mut self) {
        match &mut self.board {
            None => (),
            Some(b) => b.reset(),
        };

        self._r();
    }

    fn hard_reset(&mut self) {
        match &mut self.board {
            None => (),
            Some(b) => b.hard_reset(),
        };

        self._r();
    }
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_instructions() -> Vec<String> {
        let mut instructions = vec![];

        instructions.push(format!(""));
        instructions.push("Following commands are the way to interact with the board,".into());
        instructions.push("".into());
        instructions.push(format!(
            "{}: g007 (7 is the guess, 0 and 0 indicate x and y coordinates)",
            "Guess".bold()
        ));
        instructions.push(format!(
            "{}: o23 (2 and 3 indicate x and y coordinates)",
            "RemoveGuess".bold()
        ));
        instructions.push(format!(
            "{}: t | {}: h07 (0 and 7 indicate x and y coordinates)",
            "Time elapsed".bold(),
            "Hint".bold()
        ));
        instructions.push(format!(
            "{}: k | {}: u | {}: r | {}: i<n> (i followed by a valid number)",
            "Give up".bold(),
            "Undo".bold(),
            "Redo".bold(),
            "Highlight".bold()
        ));
        instructions.push(format!(
            "{}: s<n> (n could be 1 (Empty) or 2 (Filled) or 3 (Thonky Sudoku))",
            "Share".bold()
        ));
        instructions.push(format!(
            "{}: y | {}: z | {}: x",
            "Reset".bold(),
            "Hard Reset".bold(),
            "Exit".bold()
        ));
        instructions.push(format!(""));
        instructions.push(format!(
            "{}: {}",
            "Designed and developed by".italic(),
            "DOES IT MATTER?".italic()
        ));

        instructions.reverse();

        instructions
    }
}
