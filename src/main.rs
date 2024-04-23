mod game;
mod types;
mod ui;
mod utils;

use std::fs::File;
use std::io::BufRead;

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

const WORDS_PATH: &str = "words.txt";

fn load_words() -> std::io::Result<Vec<String>> {
    let file = File::open(WORDS_PATH)?;
    let buf_reader = std::io::BufReader::new(file);
    buf_reader.lines().collect()
}

fn main() {
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();
    enable_raw_mode().unwrap();

    let word_bank = load_words().unwrap();

    game::run(&word_bank);

    disable_raw_mode().unwrap();
    execute!(stdout, Show, LeaveAlternateScreen).unwrap();
}
