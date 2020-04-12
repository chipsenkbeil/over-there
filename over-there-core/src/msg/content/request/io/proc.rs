use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ExecProcArgs {
    pub command: String,
    pub args: Vec<String>,
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,

    /// If provided, sets the current directory where the proc will be executed
    pub current_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct WriteProcStdinArgs {
    pub id: u32,
    pub input: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ReadProcStdoutArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ReadProcStderrArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct KillProcArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ReadProcStatusArgs {
    pub id: u32,
}
