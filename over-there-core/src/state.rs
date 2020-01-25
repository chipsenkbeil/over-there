use crate::msg::callback::CallbackManager;

pub trait State {
    fn callback_manager(&mut self) -> &mut CallbackManager;
}
