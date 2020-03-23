use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoExecProcArgs {
    pub command: String,
    pub args: Vec<String>,
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProcStartedArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoWriteStdinArgs {
    pub id: u32,
    pub input: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StdinWrittenArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoGetStdoutArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StdoutContentsArgs {
    pub id: u32,
    pub output: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoGetStderrArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StderrContentsArgs {
    pub id: u32,
    pub output: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoKillProcArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoGetProcStatusArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProcStatusArgs {
    pub id: u32,
    pub is_alive: bool,
    pub exit_code: Option<i32>,
}
