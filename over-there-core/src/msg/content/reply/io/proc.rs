use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProcStartedArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct StdinWrittenArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct StdoutContentsArgs {
    pub id: u32,
    pub output: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct StderrContentsArgs {
    pub id: u32,
    pub output: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProcKilledArgs {
    pub id: u32,
    pub exit_code: Option<i32>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProcStatusArgs {
    pub id: u32,
    pub is_alive: bool,
    pub exit_code: Option<i32>,
}
