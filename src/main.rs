use crossterm::style::{Color, SetForegroundColor};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
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
    chars_typed: Arc<AtomicU16>,
    errors_typed: Arc<AtomicU16>,
    state: Arc<AtomicBool>,
}

impl<'a> TypingTest<'a> {
    fn new(word_bank: &'a [String]) -> Self {
        TypingTest {
            word_bank,
            dq: VecDeque::new(),
            cursor_index: (0, 0),
            h_idx: 0,
            chars_typed: Arc::new(AtomicU16::new(0)),
            errors_typed: Arc::new(AtomicU16::new(0)),
            state: Arc::new(AtomicBool::new(true)),
        }
    }

    fn init(&mut self) -> thread::JoinHandle<()> {
        self.state.store(true, Ordering::SeqCst);
        self.chars_typed.store(0, Ordering::SeqCst);
        self.errors_typed.store(0, Ordering::SeqCst);

        self.dq = (0..Q_SIZE)
            .map(|_| get_random_line(self.word_bank))
            .collect();
        self.dq.push_front(vec![]);

        self.cursor_index = (0, 0);
        self.h_idx = get_padding(&self.dq[1]);

        full_redraw(&self.dq);

        let state_clone = self.state.clone();
        let chars_typed_clone = self.chars_typed.clone();
        let errors_typed_clone = self.errors_typed.clone();
        thread::spawn(move || {
            update_timer(
                state_clone,
                Instant::now(),
                Duration::new(30, 0),
                chars_typed_clone,
                errors_typed_clone,
            )
        })
    }

    fn run(mut self) {
        let mut timer_thread = self.init();

        loop {
            if !self.state.load(Ordering::SeqCst) {
                match get_key_code() {
                    KeyCode::Tab => timer_thread = self.init(),
                    KeyCode::Esc => break,
                    _ => (),
                }
                continue;
            }

            match get_key_code() {
                KeyCode::Esc => {
                    self.state.store(false, Ordering::SeqCst);
                    break;
                }
                KeyCode::Tab => {
                    self.state.store(false, Ordering::SeqCst);
                    timer_thread.join().expect("Timer panicked!");
                    timer_thread = self.init()
                }
                KeyCode::Char(c) => self.process_char(c),
                KeyCode::Backspace => self.process_backspace(),
                _ => {}
            }

            if self.cursor_index.0 == self.dq[1].len() {
                self.dq.pop_front();
                self.dq.push_back(get_random_line(&self.word_bank));

                full_redraw(&self.dq);

                self.cursor_index = (0, 0);
                self.h_idx = get_padding(&self.dq[1]);
            }
        }
    }

    fn process_backspace(&mut self) {
        let line: &[&str] = &self.dq[1];

        if self.cursor_index > (0, 0) && self.chars_typed.load(Ordering::SeqCst) > 0 {
            self.chars_typed.fetch_sub(1, Ordering::SeqCst);
        }

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

        self.chars_typed.fetch_add(1, Ordering::SeqCst);

        let mut correct_ch = line[self.cursor_index.0]
            .chars()
            .nth(self.cursor_index.1)
            .unwrap_or(' ');

        if ch != correct_ch {
            self.errors_typed.fetch_add(1, Ordering::SeqCst);
            correct_ch = if correct_ch == ' ' { '_' } else { correct_ch };
            char_redraw(correct_ch, self.h_idx as u16, Color::Red);
        } else {
            char_redraw(correct_ch, self.h_idx as u16, Color::White);
        }

        self.h_idx += 1;
        if self.cursor_index.1 < word_len {
            self.cursor_index.1 += 1;
        } else {
            self.cursor_index.0 += 1;
            self.cursor_index.1 = 0;
        }
    }
}

fn update_timer(
    state: Arc<AtomicBool>,
    start_time: Instant,
    duration: Duration,
    chars_typed: Arc<AtomicU16>,
    errors_typed: Arc<AtomicU16>,
) {
    let mut stdout = std::io::stdout();

    let (width, _) = terminal_size().unwrap();
    let width = width as usize;

    let timer_padding = (width - 5) / 2;
    let metrics_padding = (width - 100) / 2;

    while state.load(Ordering::SeqCst) {
        let elapsed = start_time.elapsed();
        if elapsed >= duration {
            break;
        }

        let typed = chars_typed.load(Ordering::SeqCst) as f32;
        let errors = errors_typed.load(Ordering::SeqCst) as f32;
        let elapsed_min = elapsed.as_secs_f32() / 60.0;

        let wpm = if elapsed_min > 0.0 {
            (typed / elapsed_min / 5.0) as u16
        } else {
            0
        };

        let acc = if typed > 0.0 {
            (100.0 * (typed - errors) / typed) as u16
        } else {
            100
        };

        let wpm_text = format!("WPM {:3}", wpm);
        let acc_text = format!("ACC {:3}%", acc);

        let remaining = duration - elapsed;
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;

        execute!(
            stdout.lock(),
            SetForegroundColor(Color::White),
            MoveTo(metrics_padding as u16, 1),
            Print(wpm_text),
            MoveTo(metrics_padding as u16, 2),
            Print(acc_text),
            MoveTo(timer_padding as u16, 4),
            Print(format!("{:02}:{:02}", mins, secs)),
        )
        .unwrap();
        stdout.flush().unwrap();

        thread::sleep(Duration::from_millis(500));
    }
    state.store(false, Ordering::SeqCst);
}

fn get_key_code() -> KeyCode {
    if let Event::Key(key_event) = read().unwrap() {
        return key_event.code;
    }
    KeyCode::Char('$')
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
    execute!(
        stdout.lock(),
        MoveTo(0, 5),
        Clear(ClearType::FromCursorDown)
    )
    .unwrap();

    let (_, height) = terminal_size().unwrap();
    let vertical_center = height / 2;

    for (i, line) in dq.iter().enumerate().take(3) {
        if line.is_empty() {
            continue;
        }
        let v_idx = vertical_center - 1 + i as u16;
        let color = match i {
            0 => Color::Grey,
            _ => Color::DarkGrey,
        };
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
