use crossterm::event::KeyCode;

use crate::types::{GameState, SelectionMenuOption};
use crate::ui;

struct SelectionMenu {
    title: String,
    selection_idx: usize,
    options: Vec<SelectionMenuOption>,
}

impl SelectionMenu {
    fn new(title: String, options: Vec<SelectionMenuOption>) -> Self {
        SelectionMenu {
            title,
            options,
            selection_idx: 0,
        }
    }

    fn run(&mut self) -> GameState {
        loop {
            ui::draw_selection_menu(&self.title, &self.options, self.selection_idx, 80);
            match ui::get_key_code() {
                KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(),
                KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(),
                KeyCode::Enter => {
                    return self.options[self.selection_idx].to_state;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    return GameState::Exit;
                }
                _ => (),
            }
        }
    }

    fn move_selection_up(&mut self) {
        if self.selection_idx > 0 {
            self.selection_idx -= 1
        } else {
            self.selection_idx = self.options.len() - 1
        }
    }

    fn move_selection_down(&mut self) {
        if self.selection_idx < self.options.len() - 1 {
            self.selection_idx += 1
        } else {
            self.selection_idx = 0;
        }
    }
}

pub fn run() -> GameState {
    let mut main_menu = SelectionMenu::new(
        "Main Menu".to_string(),
        vec![
            SelectionMenuOption {
                name: "Single player".to_string(),
                to_state: GameState::SinglePlayer,
            },
            SelectionMenuOption {
                name: "Multiplayer".to_string(),
                to_state: GameState::SinglePlayer,
            },
            SelectionMenuOption {
                name: "Exit".to_string(),
                to_state: GameState::Exit,
            },
        ],
    );

    main_menu.run()
}
