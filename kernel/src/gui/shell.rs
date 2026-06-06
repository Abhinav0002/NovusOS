use super::commands::{self, SystemCtx};
use super::console::{Console, ConsoleFmtWriter};
use super::framebuffer::Framebuffer;
use super::keyboard::KeyEvent;
use core::fmt::Write;

const MAX_INPUT: usize = 256;
const PROMPT: &str = "novus> ";

pub struct Shell {
    buf: [u8; MAX_INPUT],
    len: usize,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            buf: [0u8; MAX_INPUT],
            len: 0,
        }
    }

    pub fn draw_prompt(&self, con: &mut Console, fb: &mut Framebuffer) {
        let mut w = ConsoleFmtWriter { console: con, fb };
        let _ = write!(w, "{}", PROMPT);
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        con: &mut Console,
        fb: &mut Framebuffer,
        ctx: &SystemCtx,
    ) -> bool {
        match key {
            KeyEvent::Char(c) => {
                if self.len < MAX_INPUT - 1 && c.is_ascii() {
                    self.buf[self.len] = c as u8;
                    self.len += 1;
                    con.put_char(fb, c);
                }
            }
            KeyEvent::Enter => {
                {
                    let mut w = ConsoleFmtWriter { console: con, fb };
                    let _ = writeln!(w);
                }
                let cmd = core::str::from_utf8(&self.buf[..self.len]).unwrap_or("");
                let cmd = cmd.trim();
                if !cmd.is_empty() {
                    let should_quit = commands::execute(cmd, con, fb, ctx);
                    if should_quit {
                        return true;
                    }
                }
                self.len = 0;
                self.draw_prompt(con, fb);
            }
            KeyEvent::Backspace => {
                if self.len > 0 {
                    self.len -= 1;
                    con.backspace(fb);
                }
            }
            _ => {}
        }
        false
    }
}
