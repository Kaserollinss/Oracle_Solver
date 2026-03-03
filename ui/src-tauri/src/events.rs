use serde::{Deserialize, Serialize};

use crate::types::SolveProgressDto;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SolveProgress(pub SolveProgressDto);

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SolveComplete(pub SolveProgressDto);
