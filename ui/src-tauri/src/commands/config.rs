use oracle_engine::node::Card;
use oracle_tree::GameConfig;

use crate::types::{CardDto, GameConfigDto};

#[tauri::command]
#[specta::specta]
pub fn validate_board(board: String) -> Result<Vec<CardDto>, String> {
    log::info!("validate_board: {}", board);
    let cards = Card::parse_board(&board)?;
    let len = cards.len();
    if len != 3 && len != 4 && len != 5 {
        return Err(format!(
            "Board must have 3, 4, or 5 cards, got {}",
            len
        ));
    }
    Ok(cards.iter().map(|c| CardDto::from_card(*c)).collect())
}

#[tauri::command]
#[specta::specta]
pub fn validate_config(config: GameConfigDto) -> Result<(), String> {
    log::info!("validate_config: {:?}", config);
    let board = Card::parse_board(&config.board)?;
    let gc = GameConfig {
        board,
        pot: config.pot,
        stacks: [config.stacks, config.stacks],
        bet_sizes: [
            config.bet_sizes.clone(),
            config.bet_sizes.clone(),
            config.bet_sizes.clone(),
        ],
        raise_sizes: [
            config.raise_sizes.clone(),
            config.raise_sizes.clone(),
            config.raise_sizes.clone(),
        ],
        max_raises_per_street: config.max_raises_per_street,
        allin_threshold: config.allin_threshold,
    };
    gc.validate().map_err(|e| format!("{:?}", e))
}
