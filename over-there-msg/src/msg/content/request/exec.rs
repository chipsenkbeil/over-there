use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ExecRequest {
    /// Execute a command, potentially returning the completed output
    ExecRequest {
        command: String,
        args: Vec<String>,
        capture_std_out: bool,
        capture_std_err: bool,
    },

    /// Execute a command, potentially streaming the live output
    ExecStreamRequest {
        command: String,
        args: Vec<String>,
        capture_std_out: bool,
        capture_std_err: bool,
    },
}
