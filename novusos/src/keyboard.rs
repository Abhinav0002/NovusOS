use uefi::proto::console::text::{Key, ScanCode};

pub enum KeyEvent {
    Char(char),
    Enter,
    Backspace,
    Escape,
    Up,
    Down,
}

pub fn poll_key() -> Option<KeyEvent> {
    uefi::system::with_stdin(|stdin| {
        let key = stdin.read_key().ok()??;
        match key {
            Key::Printable(ch) => {
                let c: char = ch.into();
                if c == '\r' || c == '\n' {
                    Some(KeyEvent::Enter)
                } else if c == '\x08' {
                    Some(KeyEvent::Backspace)
                } else {
                    Some(KeyEvent::Char(c))
                }
            }
            Key::Special(scan) => match scan {
                ScanCode::ESCAPE => Some(KeyEvent::Escape),
                ScanCode::UP => Some(KeyEvent::Up),
                ScanCode::DOWN => Some(KeyEvent::Down),
                _ => None,
            },
        }
    })
}
