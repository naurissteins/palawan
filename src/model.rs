use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Skipped,
    Failed,
}

pub struct Step {
    pub name: String,
    pub status: StepStatus,
    pub err: Option<String>,
}

pub enum InstallerEvent {
    Log(String),
    Progress(f64),
    Step {
        index: usize,
        status: StepStatus,
        err: Option<String>,
    },
    Done(Option<String>),
    NeedSudo,
}

pub struct App {
    pub steps: Vec<Step>,
    pub progress: f64,
    pub logs: VecDeque<String>,
    pub spinner_idx: usize,
    pub done: bool,
    pub err: Option<String>,
}
