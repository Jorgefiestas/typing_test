use std::collections::VecDeque;

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use crossterm::style::Color;

use crate::types::GameState;
use crate::ui;
use crate::utils;

const Q_SIZE: usize = 5;
const MAX_WIDTH: usize = 80;
const TIME: u64 = 30;

#[derive(Eq, PartialEq)]
enum TypingTestReturn {
    Restart,
    Back,
}

pub struct TypingTest<'a> {
    word_bank: &'a [String],
    lines_deque: VecDeque<Vec<&'a str>>,
    cursor_index: usize,
    line_index: (usize, usize),
    line_errors: Vec<usize>,
    chars_typed: Arc<AtomicU16>,
    errors_typed: Arc<AtomicU16>,
    state: Arc<AtomicBool>,
}

impl<'a> TypingTest<'a> {
    fn new(word_bank: &'a [String]) -> Self {
        let mut lines_deque: VecDeque<Vec<&str>> = VecDeque::new();

        lines_deque.push_front(vec![]);
        for _ in 0..Q_SIZE {
            lines_deque.push_back(utils::get_random_line(word_bank, MAX_WIDTH));
        }

        let cursor_index = ui::get_padding(&lines_deque[1]);

        TypingTest {
            word_bank,
            lines_deque,
            cursor_index,
            line_index: (0, 0),
            line_errors: Vec::new(),
            chars_typed: Arc::new(AtomicU16::new(0)),
            errors_typed: Arc::new(AtomicU16::new(0)),
            state: Arc::new(AtomicBool::new(true)),
        }
    }

    fn run(mut self) -> TypingTestReturn {
        ui::full_redraw(&self.lines_deque, true, None);

        let state_clone = self.state.clone();
        let chars_typed_clone = self.chars_typed.clone();
        let errors_typed_clone = self.errors_typed.clone();

        let timer_thread = thread::spawn(move || {
            Self::update_timer(
                state_clone,
                chars_typed_clone,
                errors_typed_clone,
                Instant::now(),
                Duration::new(TIME, 0),
            )
        });

        loop {
            let key = ui::get_key_code();

            if !self.state.load(Ordering::Relaxed) {
                break;
            }

            match key {
                KeyCode::Char(c) => self.process_char(c),
                KeyCode::Backspace => self.process_backspace(),
                KeyCode::Esc => {
                    self.state.store(false, Ordering::Relaxed);
                    timer_thread.join().expect("Timer panicked!");
                    return TypingTestReturn::Back;
                }
                KeyCode::Tab => {
                    self.state.store(false, Ordering::Relaxed);
                    timer_thread.join().expect("Timer panicked!");
                    return TypingTestReturn::Restart;
                }
                _ => {}
            }
        }

        timer_thread.join().expect("Timer panicked!");

        // TODO: show stats screen

        loop {
            match ui::get_key_code() {
                KeyCode::Esc => {
                    return TypingTestReturn::Back;
                }
                KeyCode::Tab => {
                    return TypingTestReturn::Restart;
                }
                _ => (),
            }
        }
    }

    fn update_timer(
        state: Arc<AtomicBool>,
        chars_typed: Arc<AtomicU16>,
        errors_typed: Arc<AtomicU16>,
        start_time: Instant,
        duration: Duration,
    ) {
        while state.load(Ordering::SeqCst) {
            if start_time.elapsed() >= duration {
                state.store(false, Ordering::Relaxed);
                break;
            }

            let elapsed = start_time.elapsed();
            let typed = chars_typed.load(Ordering::Relaxed) as f32;
            let errors = errors_typed.load(Ordering::Relaxed) as f32;
            let elapsed_min = elapsed.as_secs_f32() / 60.0;

            let wpm = if elapsed_min > 0.0 {
                (typed / 5.0 / elapsed_min) as u16
            } else {
                0
            };
            let acc = if typed > 0.0 {
                (100.0 * (typed - errors) / typed) as u16
            } else {
                100
            };
            let remaining = duration - elapsed;

            ui::draw_timer(wpm, acc, remaining, MAX_WIDTH);
            thread::sleep(Duration::from_millis(500));
        }
    }

    fn process_line_change(&mut self) {
        let new_line = utils::get_random_line(&self.word_bank, MAX_WIDTH);
        self.lines_deque.push_back(new_line);
        self.lines_deque.pop_front();

        ui::full_redraw(&self.lines_deque, false, Some(&self.line_errors));

        self.line_errors.clear();
        self.line_index = (0, 0);
        self.cursor_index = ui::get_padding(&self.lines_deque[1]);
    }

    fn process_backspace(&mut self) {
        let line: &[&str] = &self.lines_deque[1];

        if self.line_index > (0, 0) {
            self.chars_typed.fetch_sub(1, Ordering::Relaxed);
        }

        match self.line_index {
            (0, 0) => (),
            (_, 0) => {
                self.line_index.0 -= 1;
                self.line_index.1 = line[self.line_index.0].len();
                self.cursor_index -= 1;
                ui::char_redraw(' ', self.cursor_index as u16, Color::DarkGrey);
            }
            _ => {
                self.line_index.1 -= 1;
                self.cursor_index -= 1;

                let ch = line[self.line_index.0]
                    .chars()
                    .nth(self.line_index.1)
                    .unwrap();
                ui::char_redraw(ch, self.cursor_index as u16, Color::DarkGrey);
            }
        }

        if let Some(&last_error) = self.line_errors.last() {
            if self.cursor_index == last_error {
                self.errors_typed.fetch_sub(1, Ordering::Relaxed);
                self.line_errors.pop();
            }
        }
    }

    fn process_char(&mut self, ch: char) {
        let line: &[&str] = &self.lines_deque[1];
        let word_len = line[self.line_index.0].len();

        self.chars_typed.fetch_add(1, Ordering::Relaxed);

        let mut correct_ch = line[self.line_index.0]
            .chars()
            .nth(self.line_index.1)
            .unwrap_or(' ');

        if ch != correct_ch {
            self.errors_typed.fetch_add(1, Ordering::Relaxed);
            self.line_errors.push(self.cursor_index);
            correct_ch = if correct_ch == ' ' { '_' } else { correct_ch };
            ui::char_redraw(correct_ch, self.cursor_index as u16, Color::Red);
        } else {
            ui::char_redraw(correct_ch, self.cursor_index as u16, Color::White);
        }

        self.cursor_index += 1;
        if self.line_index.1 < word_len {
            self.line_index.1 += 1;
        } else {
            self.line_index.0 += 1;
            self.line_index.1 = 0;
        }

        if self.line_index.0 == self.lines_deque[1].len() {
            self.process_line_change();
        }
    }
}

pub fn run(word_bank: &[String]) -> GameState {
    loop {
        let typing_test = TypingTest::new(&word_bank);
        let state = typing_test.run();
        if state == TypingTestReturn::Back {
            break;
        }
    }

    GameState::MainMenu
}
