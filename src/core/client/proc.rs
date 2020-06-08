use crate::core::reply::{ProcStartedArgs, ProcStatusArgs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteProcStatus {
    pub id: u32,
    pub is_alive: bool,
    pub exit_code: Option<i32>,
}

impl From<ProcStatusArgs> for RemoteProcStatus {
    fn from(status: ProcStatusArgs) -> Self {
        Self {
            id: status.id,
            is_alive: status.is_alive,
            exit_code: status.exit_code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProc {
    pub(crate) id: u32,
}

impl RemoteProc {
    /// Creates a new remote reference without validating anything about
    /// the process running or even existing
    pub fn shallow(id: u32) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl From<ProcStartedArgs> for RemoteProc {
    fn from(args: ProcStartedArgs) -> Self {
        Self { id: args.id }
    }
}
