use crate::color::*;
use crate::font;
use crate::framebuffer::Framebuffer;

const TITLE_HEIGHT: usize = 22;
const BORDER_WIDTH: usize = 1;

pub struct TerminalWindow {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl TerminalWindow {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self { x, y, width, height }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        fb.fill_rect(
            self.x + 3, self.y + 3,
            self.width, self.height,
            Color::new(0, 0, 0),
        );

        fb.fill_rect(self.x, self.y, self.width, self.height, CONTENT_BG);

        fb.fill_rect(self.x, self.y, self.width, TITLE_HEIGHT, TITLE_BAR);

        font::render_str(
            fb,
            self.x + 8, self.y + 3,
            "Terminal",
            WHITE, TITLE_BAR,
        );

        let close_x = self.x + self.width - 16;
        fb.fill_rect(close_x, self.y + 6, 10, 10, RED);

        fb.fill_rect(self.x, self.y, self.width, BORDER_WIDTH, BORDER);
        fb.fill_rect(self.x, self.y + self.height - BORDER_WIDTH, self.width, BORDER_WIDTH, BORDER);
        fb.fill_rect(self.x, self.y, BORDER_WIDTH, self.height, BORDER);
        fb.fill_rect(self.x + self.width - BORDER_WIDTH, self.y, BORDER_WIDTH, self.height, BORDER);

        fb.fill_rect(self.x, self.y + TITLE_HEIGHT, self.width, BORDER_WIDTH, BORDER);
    }

    pub fn content_x(&self) -> usize {
        self.x + BORDER_WIDTH + 4
    }

    pub fn content_y(&self) -> usize {
        self.y + TITLE_HEIGHT + BORDER_WIDTH + 2
    }

    pub fn content_width(&self) -> usize {
        self.width - 2 * BORDER_WIDTH - 8
    }

    pub fn content_height(&self) -> usize {
        self.height - TITLE_HEIGHT - 2 * BORDER_WIDTH - 4
    }
}
