use std::{error::Error, fmt::Display};

use crate::sudoku::Position;

#[derive(Debug, Clone, Default)]
pub enum MainSelection {
    New,
    Load,
    #[default]
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

#[derive(Debug, Clone)]
pub enum MessageType {
    Success,
    Error,
    Warn,
    Normal,
}

#[derive(Debug)]
pub struct Message<'a> {
    msg: &'a str,
    msg_type: MessageType,
}

impl<'a> Message<'a> {
    pub fn new(msg: &'a str, msg_type: MessageType) -> Self {
        Message { msg, msg_type }
    }

    pub fn get_type(&self) -> MessageType {
        self.msg_type.clone()
    }

    pub fn get_msg(&self) -> &str {
        self.msg
    }
}

#[derive(Debug)]
pub enum UserRequest {
    Guess(Position, u8),
    Undo,
    Redo,
    Reset,
    HardReset,
    Giveup,
    Hint(Position),
    Highlight(u8),
    RemoveHighlight,
    TimeElapsed,
    Exit,
}

impl UserRequest {
    pub fn parse(ui: &str) -> Result<Self, Box<dyn Error>> {
        let ui = ui.to_lowercase();

        let chars = ui.chars().collect::<Vec<char>>();

        if chars.len() == 0 {
            return Err("expected userRequest to be of atleast 1 char long".into());
        }

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
                    return Ok(Self::RemoveHighlight);
                }

                let val = match chars[1].to_digit(10) {
                    Some(v) => v as u8,
                    None => {
                        return Ok(Self::RemoveHighlight);
                    }
                };

                if val > 9 {
                    return Ok(Self::RemoveHighlight);
                }

                return Ok(Self::Highlight(val));
            }
            't' => return Ok(Self::TimeElapsed),
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
