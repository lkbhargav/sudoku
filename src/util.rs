use std::borrow::Cow;

use colored::Colorize;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

#[macro_export]
macro_rules! display_error {
    ($a:expr) => {
        eprintln!("{}", format!("Error: {}", $a).red())
    };
}

#[macro_export]
macro_rules! display_warn {
    ($a:expr) => {
        eprintln!("{}", format!("Warning: {}", $a).yellow())
    };
}

pub fn prompt_select<T>(q: &str, items: &Vec<T>) -> usize
where
    T: std::fmt::Display,
{
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(q)
        .default(0)
        .items(items)
        .interact()
        .expect("error trying to render a select")
}

pub fn prompt<'a>(q: &'a str, default: &str) -> Cow<'a, str> {
    Cow::Owned(
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(q)
            .default(default.to_string())
            .interact_text()
            .expect("error trying to get input"),
    )
}

pub fn confirm(q: &str, default: bool) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(q)
        .default(default)
        .interact()
        .expect("error trying to confirm")
}

pub fn prompt_with_validation<'a>(
    q: &'a str,
    default: &str,
    validator: fn(&str) -> Option<&str>,
) -> Cow<'a, str> {
    Cow::Owned(
        Input::with_theme(&ColorfulTheme::default())
            .validate_with(|i: &String| -> Result<(), &str> {
                let val = validator(i);
                if val.is_none() {
                    return Ok(());
                }
                println!("{}", val.unwrap().bright_red().on_bright_black());
                Err("validation failed")
            })
            .with_prompt(q)
            .default(default.to_string())
            .interact_text()
            .expect("error trying to get input"),
    )
}
