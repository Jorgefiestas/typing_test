mod menu;
mod singleplayer;

use crate::types::GameState;

pub fn run(word_bank: &[String]) {
    let mut state = GameState::MainMenu;

    while state != GameState::Exit {
        state = match state {
            GameState::MainMenu => menu::run(),
            GameState::SinglePlayer => singleplayer::run(word_bank),
            _ => GameState::Exit,
        };
    }
}
