use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{read, Event, KeyCode};
use crossterm::execute;
use crossterm::style::{Attribute, Color, SetForegroundColor};
use crossterm::style::{Print, SetAttribute};
use crossterm::terminal::{size as terminal_size, Clear, ClearType};

use crate::types::SelectionMenuOption;

fn print_centered_words(words: &[&str], row_idx: u16, color: Color, errors: Option<&[usize]>) {
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
        Print(&text)
    )
    .expect("Failed to print text");

    for idx in errors.unwrap_or(&[]) {
        let ch = match text.chars().nth(idx - horizontal_padding) {
            Some(' ') => '_',
            Some(c) => c,
            None => '_',
        };

        execute!(
            stdout.lock(),
            MoveTo(*idx as u16, row_idx as u16),
            SetForegroundColor(Color::Red),
            Print(ch)
        )
        .unwrap();
    }
}

pub fn char_redraw(c: char, h_idx: u16, color: Color) {
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

pub fn full_redraw(dq: &VecDeque<Vec<&str>>, clear_metrics: bool, errors: Option<&[usize]>) {
    let mut stdout = std::io::stdout();

    if clear_metrics {
        execute!(stdout.lock(), Clear(ClearType::All)).unwrap();
    } else {
        execute!(
            stdout.lock(),
            MoveTo(0, 5),
            Clear(ClearType::FromCursorDown)
        )
        .unwrap();
    }

    let (_, height) = terminal_size().unwrap();
    let v_center = height / 2;

    if !dq[0].is_empty() {
        print_centered_words(&dq[0], v_center - 1, Color::White, errors);
    }
    for i in 1..3 {
        let v_idx = v_center - 1 + i as u16;
        print_centered_words(&dq[i], v_idx, Color::DarkGrey, None);
    }

    stdout.flush().unwrap();
}

pub fn draw_timer(wpm: u16, acc: u16, remaining: Duration, max_width: usize) {
    let mut stdout = std::io::stdout();

    let (width, _) = terminal_size().unwrap();
    let width = width as usize;

    let timer_padding = (width - 5) / 2;
    let metrics_padding = (width - max_width) / 2;

    let wpm_text = format!("WPM {:3}", wpm);
    let acc_text = format!("ACC {:3}%", acc);

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
}

pub fn get_key_code() -> KeyCode {
    match read() {
        Ok(Event::Key(key_event)) => key_event.code,
        _ => KeyCode::Char('$'),
    }
}

pub fn get_padding(words: &[&str]) -> usize {
    let text = words.join(" ");
    let text_length = text.chars().count();
    let (width, _) = terminal_size().unwrap();
    let width = width as usize;
    (width - text_length) / 2
}

pub fn draw_selection_menu(
    title: &str,
    options: &[SelectionMenuOption],
    selection_idx: usize,
    max_width: usize,
) {
    let mut stdout = std::io::stdout();
    execute!(stdout.lock(), Clear(ClearType::All)).unwrap();

    let (width, height) = terminal_size().unwrap();

    let menu_lines = 2 * options.len() + 1;
    let mut line_number = (height - menu_lines as u16) / 2;

    let title_padding = (width - title.len() as u16) / 2;
    let options_padding = (width - max_width as u16) / 2;

    execute!(
        stdout.lock(),
        MoveTo(title_padding, line_number),
        Print(title),
    )
    .unwrap();

    for (index, option) in options.iter().enumerate() {
        line_number += 2;
        execute!(stdout.lock(), MoveTo(options_padding, line_number)).unwrap();

        if index == selection_idx {
            execute!(
                stdout.lock(),
                Print("=> "),
                SetAttribute(Attribute::Underlined),
            )
            .unwrap();
        }

        execute!(stdout.lock(), Print(&option.name)).unwrap();

        if index == selection_idx {
            execute!(stdout.lock(), SetAttribute(Attribute::NoUnderline)).unwrap();
        }
    }

    stdout.flush().unwrap();
}
