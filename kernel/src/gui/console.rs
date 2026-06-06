use super::color::Color;
use super::font::{self, GLYPH_HEIGHT, GLYPH_WIDTH};
use super::framebuffer::Framebuffer;
use core::fmt;

pub struct Console {
    col: usize,
    row: usize,
    cols: usize,
    rows: usize,
    x_offset: usize,
    y_offset: usize,
    area_width: usize,
    area_height: usize,
    fg: Color,
    bg: Color,
}

impl Console {
    pub fn new(
        x_offset: usize,
        y_offset: usize,
        area_width: usize,
        area_height: usize,
        fg: Color,
        bg: Color,
    ) -> Self {
        Self {
            col: 0,
            row: 0,
            cols: area_width / GLYPH_WIDTH,
            rows: area_height / GLYPH_HEIGHT,
            x_offset,
            y_offset,
            area_width,
            area_height,
            fg,
            bg,
        }
    }

    pub fn clear(&mut self, fb: &mut Framebuffer) {
        fb.fill_rect(self.x_offset, self.y_offset, self.area_width, self.area_height, self.bg);
        self.col = 0;
        self.row = 0;
    }

    pub fn set_colors(&mut self, fg: Color, bg: Color) {
        self.fg = fg;
        self.bg = bg;
    }

    fn scroll_up(&mut self, fb: &mut Framebuffer) {
        fb.scroll_region_up(
            self.x_offset,
            self.y_offset,
            self.area_width,
            self.area_height,
            GLYPH_HEIGHT,
            self.bg,
        );
        self.row = self.rows - 1;
    }

    fn newline(&mut self, fb: &mut Framebuffer) {
        self.col = 0;
        self.row += 1;
        if self.row >= self.rows {
            self.scroll_up(fb);
        }
    }

    pub fn put_char(&mut self, fb: &mut Framebuffer, ch: char) {
        match ch {
            '\n' => self.newline(fb),
            '\r' => self.col = 0,
            '\t' => {
                let spaces = 4 - (self.col % 4);
                for _ in 0..spaces {
                    self.put_char(fb, ' ');
                }
            }
            _ => {
                if self.col >= self.cols {
                    self.newline(fb);
                }
                let px = self.x_offset + self.col * GLYPH_WIDTH;
                let py = self.y_offset + self.row * GLYPH_HEIGHT;
                font::render_char(fb, px, py, ch, self.fg, self.bg);
                self.col += 1;
            }
        }
    }

    pub fn put_str(&mut self, fb: &mut Framebuffer, s: &str) {
        for ch in s.chars() {
            self.put_char(fb, ch);
        }
    }

    pub fn backspace(&mut self, fb: &mut Framebuffer) {
        if self.col > 0 {
            self.col -= 1;
            let px = self.x_offset + self.col * GLYPH_WIDTH;
            let py = self.y_offset + self.row * GLYPH_HEIGHT;
            font::render_char(fb, px, py, ' ', self.fg, self.bg);
        }
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn fg(&self) -> Color {
        self.fg
    }

    pub fn bg(&self) -> Color {
        self.bg
    }
}

pub struct ConsoleFmtWriter<'a, 'b> {
    pub console: &'a mut Console,
    pub fb: &'b mut Framebuffer,
}

impl<'a, 'b> fmt::Write for ConsoleFmtWriter<'a, 'b> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.put_str(self.fb, s);
        Ok(())
    }
}
