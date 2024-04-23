#[derive(Eq, PartialEq, Clone, Copy)]
pub enum GameState {
    MainMenu,
    SinglePlayer,
    MultiPlayer,
    ConfigMenu,
    Exit,
}

pub struct SelectionMenuOption {
    pub name: String,
    pub to_state: GameState,
}
