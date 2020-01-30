use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExecRequestArgs {
    pub command: String,
    pub args: Vec<String>,
    pub capture_std_out: bool,
    pub capture_std_err: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExecStreamRequestArgs {
    pub command: String,
    pub args: Vec<String>,
    pub capture_std_out: bool,
    pub capture_std_err: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExecExitArgs {
    pub exit_code: u32,
    pub std_out: Option<String>,
    pub std_err: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ExecStreamResponseArgs {
    pub exit_code: Option<u32>,
    pub std_out: Option<String>,
    pub std_err: Option<String>,
}
