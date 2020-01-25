pub mod route;
pub mod state;

pub struct Server {
    state: state::ServerState,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            state: state::ServerState::default(),
        }
    }
}
