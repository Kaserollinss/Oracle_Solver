use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use oracle_engine::RangeSolver;

use crate::types::{SolveProgressDto, SolveStatus, TreeInfoDto};

pub struct SolverSession {
    pub solver: Option<RangeSolver>,
    pub status: SolveStatus,
    pub last_progress: Option<SolveProgressDto>,
    pub tree_info: Option<TreeInfoDto>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl SolverSession {
    pub fn new() -> Self {
        SolverSession {
            solver: None,
            status: SolveStatus::Idle,
            last_progress: None,
            tree_info: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn reset_cancel(&mut self) {
        self.cancel_flag.store(false, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }
}

pub type AppState = Arc<Mutex<SolverSession>>;

pub fn new_app_state() -> AppState {
    Arc::new(Mutex::new(SolverSession::new()))
}
