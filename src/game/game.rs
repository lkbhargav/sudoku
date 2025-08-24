use std::{process::exit, time::Instant};

use colored::Colorize;
use humantime::format_duration;

use crate::{
    game::types::{MainSelection, Message, MessageType, UserRequest},
    sudoku::{HintStatus, InsertStatus, Position, Sudoku},
    util::{prompt, prompt_select},
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_game(&mut self) {
        loop {
            let main_selection_options =
                vec![MainSelection::New, MainSelection::Load, MainSelection::Exit];

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

                    let board = Sudoku::generate_random_board(clues);

                    self.set_board(board);
                    self.game_loop();
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
                    msg = format!(
                        "{}\nTime taken: {}\n",
                        "Congragulations!",
                        format_duration(start_time.elapsed())
                    );

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
                        InsertStatus::Wrong => self.mistakes += 1,
                        InsertStatus::ValuePresent =>
                            message = Some(Message::new("Value is already present in the cell, try clearing the cell before inserting a new value or force insert".into(), MessageType::Warn)),
                        _ => (),
                    };

                    self.undo_buffer.push((pos.clone(), Some(val)));
                    self.redo_buffer.clear();
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
                    self.board.as_mut().unwrap().reset();
                    self.board.as_mut().unwrap().solve(None);
                    give_up = true;
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

        // clears the screen without a scrollbar
        print!("{esc}c", esc = 27 as char);

        println!(
            "Original board: {}\nCurrent progress: {}\n",
            self.initital_board_layout,
            board.to_str()
        );
        println!(
            "Initial clues: {} {} # mistakes: {} {} # hints: {}\n",
            self.starting_clues.to_string().bold(),
            "|".white().bold(),
            self.mistakes.to_string().red().bold(),
            "|".white().bold(),
            self.additional_clues.to_string().magenta().bold()
        );

        println!("{board}");

        match message {
            None => println!("\n"),
            Some(m) => {
                let m = match m.get_type() {
                    MessageType::Error => m.get_msg().bold().red(),
                    MessageType::Warn => m.get_msg().bold().yellow(),
                    MessageType::Success => m.get_msg().bold().green(),
                    MessageType::Normal => m.get_msg().normal(),
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
