pub mod state;

pub struct Server {
    state: state::State,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            state: state::State::default(),
        }
    }
}
