use std::{error::Error, fmt::Display, process::exit, time::SystemTime};

use colored::Colorize;
use humantime::format_duration;

use crate::{
    sudoku::{HintStatus, InsertStatus, Position, Sudoku},
    util::{prompt, prompt_select},
};

#[derive(Debug)]
enum MainSelection {
    New,
    Load,
    Exit,
}

impl Display for MainSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            MainSelection::Load => write!(f, "Load"),
            MainSelection::New => write!(f, "New"),
            MainSelection::Exit => write!(f, "Exit"),
        }
    }
}

#[derive(Debug)]
enum UserRequest {
    Guess(Position, u8),
    Undo,
    Redo,
    Reset,
    HardReset,
    Giveup,
    Hint(Position),
    Highlight(u8),
    Exit,
}

impl UserRequest {
    pub fn parse(ui: &str) -> Result<Self, Box<dyn Error>> {
        let ui = ui.to_lowercase();

        let chars = ui.chars().collect::<Vec<char>>();

        match chars[0] {
            'g' => {
                if chars.len() - 1 != 3 {
                    return Err("invalid guess made, please try again".into());
                }

                let x = match chars[1].to_digit(10) {
                    Some(v) => v as usize,
                    None => {
                        return Err("expected a digit between 1 and 9 inclusive but found something else (first digit)".into());
                    }
                };

                let y = match chars[2].to_digit(10) {
                    Some(v) => v as usize,
                    None => {
                        return Err("expected a digit between 1 and 9 inclusive but found something else (second digit)".into());
                    }
                };

                let val = match chars[3].to_digit(10) {
                    Some(v) => v as u8,
                    None => {
                        return Err("expected a digit between 1 and 9 inclusive but found something else (value digit)".into());
                    }
                };

                if x > 8 || y > 8 {
                    return Err("co-ordinates are not in range, make sure it is in between 0 and 8 inclusive".into());
                }

                if val < 1 || val > 9 {
                    return Err(
                        "values are not in range, make sure it is in between 1 and 9 inclusive"
                            .into(),
                    );
                }

                return Ok(Self::Guess(Position::new(x, y), val));
            }
            'h' => {
                if chars.len() - 1 != 2 {
                    return Err("invalid hint requested, please try again".into());
                }

                let x = match chars[1].to_digit(10) {
                    Some(v) => v as usize,
                    None => {
                        return Err("expected a digit between 1 and 9 inclusive but found something else (first digit)".into());
                    }
                };

                let y = match chars[2].to_digit(10) {
                    Some(v) => v as usize,
                    None => {
                        return Err("expected a digit between 1 and 9 inclusive but found something else (second digit)".into());
                    }
                };

                if x > 8 || y > 8 {
                    return Err("co-ordinates are not in range, make sure it is in between 0 and 8 inclusive".into());
                }

                return Ok(Self::Hint(Position::new(x, y)));
            }
            'i' => {
                if chars.len() - 1 != 1 {
                    return Err("invalid highlight requested, please try again".into());
                }

                let val = match chars[1].to_digit(10) {
                    Some(v) => v as u8,
                    None => {
                        return Err("expected a digit between 1 and 9 inclusive but found something else (value digit)".into());
                    }
                };

                return Ok(Self::Highlight(val));
            }
            'u' => return Ok(Self::Undo),
            'r' => return Ok(Self::Redo),
            'y' => return Ok(Self::Reset),
            'z' => return Ok(Self::HardReset),
            'k' => return Ok(Self::Giveup),
            'x' => return Ok(Self::Exit),
            _ => {
                return Err("Unknown option, please try again".into());
            }
        }
    }
}

pub fn start_game() {
    loop {
        let main_selection_options =
            vec![MainSelection::New, MainSelection::Load, MainSelection::Exit];

        let main_selection = prompt_select(
            "Select one of the following options",
            &main_selection_options,
        );

        match main_selection_options[main_selection] {
            MainSelection::Load => {
                let str = prompt("Paste the puzzle input to load", "");

                if str.is_empty() {
                    println!("expected puzzle input but found empty string");
                    continue;
                }

                let mut board = match Sudoku::from_str(&str) {
                    Ok(b) => b,
                    Err(e) => {
                        println!(
                            "invalid puzzle input loaded, please fix that and try again later: {}",
                            e.to_string()
                        );
                        continue;
                    }
                };

                game_loop(&mut board);
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

                let mut board = Sudoku::generate_random_board(clues);

                game_loop(&mut board);
            }
            MainSelection::Exit => exit(1),
        }
    }
}

fn game_loop(board: &mut Sudoku) {
    let mut undo_list = vec![];
    let mut redo_list = vec![];
    let mut number_of_mistakes = 0;
    let mut number_of_hints = 0;
    let mut give_up = false;
    let original_board = board.to_str();
    let mut message = String::new();

    let start_time = SystemTime::now();

    loop {
        draw(
            board,
            number_of_mistakes,
            number_of_hints,
            &original_board,
            &message,
        );

        message.clear();

        if board.is_board_solved_completely() {
            if !give_up {
                println!(
                    "{}\nTime taken: {}\n",
                    "Congragulations!".green().bold(),
                    format_duration(start_time.elapsed().unwrap())
                );
            }
            // end of the puzzle
            break;
        }

        let ans = prompt(
            "Enter your guess (ex: g007 - means fill grid location 0 (x), 0 (y) with 7)",
            "",
        );

        let v = match UserRequest::parse(&ans) {
            Ok(v) => v,
            Err(e) => {
                message = format!("Error parsing your request: {}", e.to_string());
                continue;
            }
        };

        match v {
            UserRequest::Guess(pos, val) => {
                match board.insert_at(&pos, Some(val)) {
                    InsertStatus::Wrong => number_of_mistakes += 1,
                    InsertStatus::ValuePresent =>
                        message = "Value is already present in the cell, try clearing the cell before inserting a new value or force insert".into()
                    ,
                    _ => (),
                };

                undo_list.push((pos.clone(), Some(val)));
            }
            UserRequest::Undo => {
                if undo_list.is_empty() {
                    message =
                        "You can't use the undo option as there is no known previous move".into();
                    continue;
                }

                let pp = undo_list.pop().unwrap();

                board.insert_at(&pp.0, None);
                redo_list.push(pp);
            }
            UserRequest::Redo => {
                if redo_list.is_empty() {
                    message = "You can't use the redo option as there is nothing undid yet".into();
                    continue;
                }

                let pp = redo_list.pop().unwrap();

                board.insert_at(&pp.0, pp.1);
                undo_list.push(pp);
            }
            UserRequest::Hint(pos) => {
                match board.hint(&pos) {
                    HintStatus::ValuePresent => {
                        message = "Hint requested on already filled cell".into()
                    }
                    HintStatus::Ok => number_of_hints += 1,
                }
                continue;
            }
            UserRequest::Highlight(v) => {
                board.highlight(v);
            }
            UserRequest::Reset => {
                board.reset();
                undo_list.clear();
                redo_list.clear();
            }
            UserRequest::HardReset => {
                board.hard_reset();
                undo_list.clear();
                redo_list.clear();
            }
            UserRequest::Giveup => {
                board.reset();
                board.solve(None);
                give_up = true;
            }
            UserRequest::Exit => {
                exit(1);
            }
        }
    }
}

fn draw(
    board: &Sudoku,
    number_of_mistakes: u16,
    number_of_hints: u16,
    original_board: &str,
    message: &str,
) {
    // clears the screen without a scrollbar
    print!("{esc}c", esc = 27 as char);

    println!(
        "Original board: {}\nCurrent progress: {}\n",
        original_board,
        board.to_str()
    );
    println!(
        "# mistakes: {} {} # hints: {}\n",
        number_of_mistakes.to_string().red().bold(),
        "|".white().bold(),
        number_of_hints.to_string().magenta().bold()
    );

    println!("{board}");

    println!("{}\n", message.bold().yellow());
}
