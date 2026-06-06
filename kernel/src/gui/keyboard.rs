pub enum KeyEvent {
    Char(char),
    Enter,
    Backspace,
    Escape,
    Up,
    Down,
}

pub fn poll_key() -> Option<KeyEvent> {
    crate::drivers::virtio::input::poll_key()
}
