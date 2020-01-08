use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ExecResponse {
    /// Execute a command, potentially returning the completed output
    ExecResponse {
        exit_code: u32,
        std_out: Option<String>,
        std_err: Option<String>,
    },

    /// Execute a command, potentially streaming the live output
    ExecStreamResponse {
        exit_code: Option<u32>,
        std_out: Option<String>,
        std_err: Option<String>,
    },
}
