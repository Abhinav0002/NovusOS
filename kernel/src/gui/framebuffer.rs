use super::color::Color;
use boot_info::PixelFormat;

pub struct Framebuffer {
    base: *mut u8,
    pub width: usize,
    pub height: usize,
    stride: usize,
    pixel_format: PixelFormat,
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

impl Framebuffer {
    pub fn new(
        base: *mut u8,
        width: usize,
        height: usize,
        stride: usize,
        pixel_format: PixelFormat,
    ) -> Self {
        Self { base, width, height, stride, pixel_format }
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    fn pixel_bytes(&self, color: Color) -> [u8; 4] {
        match self.pixel_format {
            PixelFormat::Bgr => color.to_bgra(),
            PixelFormat::Rgb => color.to_rgba(),
        }
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y * self.stride + x) * 4;
        let bytes = self.pixel_bytes(color);
        unsafe {
            let ptr = self.base.add(offset);
            core::ptr::write_volatile(ptr, bytes[0]);
            core::ptr::write_volatile(ptr.add(1), bytes[1]);
            core::ptr::write_volatile(ptr.add(2), bytes[2]);
            core::ptr::write_volatile(ptr.add(3), bytes[3]);
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        let bytes = self.pixel_bytes(color);
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);

        for row in y..y_end {
            let row_offset = row * self.stride * 4;
            for col in x..x_end {
                let offset = row_offset + col * 4;
                unsafe {
                    let ptr = self.base.add(offset);
                    core::ptr::write_volatile(ptr, bytes[0]);
                    core::ptr::write_volatile(ptr.add(1), bytes[1]);
                    core::ptr::write_volatile(ptr.add(2), bytes[2]);
                    core::ptr::write_volatile(ptr.add(3), bytes[3]);
                }
            }
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.fill_rect(0, 0, self.width, self.height, color);
    }

    pub fn draw_hline(&mut self, x: usize, y: usize, w: usize, color: Color) {
        self.fill_rect(x, y, w, 1, color);
    }

    pub fn draw_vline(&mut self, x: usize, y: usize, h: usize, color: Color) {
        self.fill_rect(x, y, 1, h, color);
    }

    pub fn scroll_region_up(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        amount: usize,
        bg: Color,
    ) {
        if amount >= h {
            self.fill_rect(x, y, w, h, bg);
            return;
        }

        for row in y..(y + h - amount) {
            let src_row = row + amount;
            let src_offset = (src_row * self.stride + x) * 4;
            let dst_offset = (row * self.stride + x) * 4;
            unsafe {
                core::ptr::copy(
                    self.base.add(src_offset),
                    self.base.add(dst_offset),
                    w * 4,
                );
            }
        }

        self.fill_rect(x, y + h - amount, w, amount, bg);
    }
}
