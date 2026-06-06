#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_bgra(self) -> [u8; 4] {
        [self.b, self.g, self.r, 0xFF]
    }

    pub fn to_rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, 0xFF]
    }
}

pub const BLACK: Color = Color::new(0, 0, 0);
pub const WHITE: Color = Color::new(255, 255, 255);
pub const DARK_BLUE: Color = Color::new(20, 30, 60);
pub const MEDIUM_BLUE: Color = Color::new(35, 50, 100);
pub const LIGHT_GRAY: Color = Color::new(200, 200, 200);
pub const DARK_GRAY: Color = Color::new(40, 40, 40);
pub const RED: Color = Color::new(220, 60, 60);
pub const GREEN: Color = Color::new(60, 200, 80);
pub const CYAN: Color = Color::new(80, 200, 220);
pub const YELLOW: Color = Color::new(220, 200, 60);
pub const TITLE_BAR: Color = Color::new(50, 50, 55);
pub const BORDER: Color = Color::new(100, 100, 110);
pub const CONTENT_BG: Color = Color::new(15, 15, 20);
pub const STATUS_BG: Color = Color::new(30, 30, 35);
pub const ACCENT: Color = Color::new(80, 140, 220);
