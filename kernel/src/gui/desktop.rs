use super::color::*;
use super::font;
use super::framebuffer::Framebuffer;

const STATUS_BAR_HEIGHT: usize = 24;

pub struct Desktop {
    pub width: usize,
    pub height: usize,
}

impl Desktop {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub fn draw_background(&self, fb: &mut Framebuffer) {
        let top = DARK_BLUE;
        let bot = MEDIUM_BLUE;

        for y in STATUS_BAR_HEIGHT..self.height {
            let t = y - STATUS_BAR_HEIGHT;
            let range = self.height - STATUS_BAR_HEIGHT;
            let r = top.r as usize + (bot.r as usize - top.r as usize) * t / range;
            let g = top.g as usize + (bot.g as usize - top.g as usize) * t / range;
            let b = top.b as usize + (bot.b as usize - top.b as usize) * t / range;
            let color = Color::new(r as u8, g as u8, b as u8);
            fb.fill_rect(0, y, self.width, 1, color);
        }
    }

    pub fn draw_status_bar(&self, fb: &mut Framebuffer, status_text: &str) {
        fb.fill_rect(0, 0, self.width, STATUS_BAR_HEIGHT, STATUS_BG);
        fb.fill_rect(0, STATUS_BAR_HEIGHT - 1, self.width, 1, BORDER);
        font::render_str(fb, 8, 4, status_text, LIGHT_GRAY, STATUS_BG);
    }

    pub fn status_bar_height(&self) -> usize {
        STATUS_BAR_HEIGHT
    }
}
