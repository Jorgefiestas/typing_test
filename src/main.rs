use crossterm::cursor::{RestorePosition, SavePosition};
use crossterm::style::{Color, SetForegroundColor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rand::seq::SliceRandom;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, Event, KeyCode},
    execute,
    style::Print,
    terminal::{
        size as terminal_size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
};

const WORDS_PATH: &str = "words.txt";
const Q_SIZE: usize = 20;
const MAX_WIDTH: usize = 100;

struct TypingTest<'a> {
    word_bank: &'a [String],
    dq: VecDeque<Vec<&'a str>>,
    cursor_index: (usize, usize),
    h_idx: usize,
    state: Arc<AtomicBool>,
}

impl<'a> TypingTest<'a> {
    fn new(word_bank: &'a [String]) -> Self {
        TypingTest {
            word_bank,
            dq: VecDeque::new(),
            cursor_index: (0, 0),
            h_idx: 0,
            state: Arc::new(AtomicBool::new(true)),
        }
    }

    fn init(&mut self) {
        self.state.store(true, Ordering::SeqCst);

        self.dq = (0..Q_SIZE)
            .map(|_| get_random_line(self.word_bank))
            .collect();
        self.dq.push_front(vec![]);

        self.cursor_index = (0, 0);
        self.h_idx = get_padding(&self.dq[1]);

        full_redraw(&self.dq);
    }

    fn run(mut self) -> usize {
        self.init();

        let state_clone = self.state.clone();
        thread::spawn(move || update_timer(state_clone, Instant::now(), Duration::new(30, 0)));

        while self.state.load(Ordering::SeqCst) {
            if let Event::Key(key_event) = read().unwrap() {
                match key_event.code {
                    KeyCode::Esc => self.state.store(false, Ordering::SeqCst),
                    KeyCode::Tab => self.init(),
                    KeyCode::Backspace => self.process_backspace(),
                    KeyCode::Char(c) => self.process_char(c),
                    _ => {}
                }
            }

            if self.cursor_index.0 == self.dq[1].len() {
                self.dq.pop_front();
                self.dq.push_back(get_random_line(&self.word_bank));

                full_redraw(&self.dq);

                self.cursor_index = (0, 0);
                self.h_idx = get_padding(&self.dq[1]);
            }
        }

        0
    }

    fn process_backspace(&mut self) {
        let line: &[&str] = &self.dq[1];

        match self.cursor_index {
            (0, 0) => (),
            (_, 0) => {
                self.cursor_index.0 -= 1;
                self.cursor_index.1 = line[self.cursor_index.0].len();
                self.h_idx -= 1;
                char_redraw(' ', self.h_idx as u16, Color::DarkGrey);
            }
            _ => {
                self.cursor_index.1 -= 1;
                self.h_idx -= 1;

                let ch = line[self.cursor_index.0]
                    .chars()
                    .nth(self.cursor_index.1)
                    .unwrap();
                char_redraw(ch, self.h_idx as u16, Color::DarkGrey);
            }
        }
    }

    fn process_char(&mut self, ch: char) {
        let line: &[&str] = &self.dq[1];
        let word_len = line[self.cursor_index.0].len();

        let correct_ch = line[self.cursor_index.0]
            .chars()
            .nth(self.cursor_index.1)
            .unwrap_or(' ');

        let color = if ch == correct_ch {
            Color::White
        } else {
            Color::Red
        };

        char_redraw(ch, self.h_idx as u16, color);

        self.h_idx += 1;
        if self.cursor_index.1 < word_len {
            self.cursor_index.1 += 1;
        } else {
            self.cursor_index.0 += 1;
            self.cursor_index.1 = 0;
        }
    }
}

fn update_timer(state: Arc<AtomicBool>, start_time: Instant, duration: Duration) {
    let mut stdout = std::io::stdout();

    while state.load(Ordering::SeqCst) {
        let elapsed = start_time.elapsed();
        if elapsed >= duration {
            break;
        }

        let remaining = duration - elapsed;
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;

        execute!(
            stdout.lock(),
            SavePosition,
            MoveTo(0, 0),
            SetForegroundColor(Color::White),
            Print(format!("{:02}:{:02}", mins, secs)),
            RestorePosition,
        )
        .unwrap();
        stdout.flush().unwrap();

        thread::sleep(Duration::from_millis(500));
    }
    state.store(false, Ordering::SeqCst);
}

fn load_words() -> std::io::Result<Vec<String>> {
    let file = File::open(WORDS_PATH)?;
    let buf_reader = std::io::BufReader::new(file);
    buf_reader.lines().collect()
}

fn get_random_line(word_bank: &[String]) -> Vec<&str> {
    let mut rng = rand::thread_rng();

    let mut line = Vec::new();
    let mut line_size = 0;

    while line_size < MAX_WIDTH {
        let mut word = word_bank.choose(&mut rng).unwrap();
        while Some(&word.as_str()) == line.last() {
            word = word_bank.choose(&mut rng).unwrap();
        }
        line.push(word.as_str());
        line_size += word.len() + 1;
    }

    line
}

fn get_padding(words: &[&str]) -> usize {
    let text = words.join(" ");
    let text_length = text.chars().count();
    let (width, _) = terminal_size().unwrap();
    let width = width as usize;
    (width - text_length) / 2
}

fn print_centered_words(words: &[&str], row_idx: u16, color: Color) {
    let text = words.join(" ");
    let text_length = text.chars().count();

    let (width, _) = terminal_size().unwrap();
    let width = width as usize;
    let horizontal_padding = (width - text_length) / 2;

    let stdout = std::io::stdout();
    execute!(
        stdout.lock(),
        MoveTo(horizontal_padding as u16, row_idx as u16),
        SetForegroundColor(color),
        Print(text)
    )
    .expect("Failed to print text");
}

fn char_redraw(c: char, h_idx: u16, color: Color) {
    let mut stdout = std::io::stdout();

    let (_, height) = terminal_size().unwrap();
    let v_idx = height / 2;

    execute!(
        stdout.lock(),
        SetForegroundColor(color),
        MoveTo(h_idx, v_idx as u16),
        Print(c)
    )
    .unwrap();

    stdout.flush().unwrap();
}

fn full_redraw(dq: &VecDeque<Vec<&str>>) {
    let mut stdout = std::io::stdout();
    execute!(stdout.lock(), Clear(ClearType::All)).unwrap();

    let (_, height) = terminal_size().unwrap();
    let vertical_center = height / 2;

    for (i, line) in dq.iter().enumerate().take(3) {
        if line.is_empty() {
            continue;
        }
        let v_idx = vertical_center - 1 + i as u16;
        let color = if i < 1 { Color::White } else { Color::DarkGrey };
        print_centered_words(&dq[i], v_idx, color);
    }

    stdout.flush().unwrap();
}

fn main() {
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();
    enable_raw_mode().unwrap();

    let word_bank = load_words().unwrap();

    let typing_test = TypingTest::new(&word_bank);
    typing_test.run();

    disable_raw_mode().unwrap();
    execute!(stdout, Show, LeaveAlternateScreen).unwrap();
}
