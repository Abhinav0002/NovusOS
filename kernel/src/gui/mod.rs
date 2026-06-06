pub mod color;
pub mod commands;
pub mod console;
pub mod desktop;
pub mod font;
pub mod framebuffer;
pub mod keyboard;
pub mod shell;
pub mod timer;
pub mod window;

use alloc::format;
use boot_info::FramebufferInfo;
use color::*;
use commands::SystemCtx;
use console::{Console, ConsoleFmtWriter};
use core::fmt::Write;
use desktop::Desktop;
use framebuffer::Framebuffer;
use shell::Shell;
use timer::BootTimer;
use window::TerminalWindow;

static mut GUI_FB: Option<Framebuffer> = None;

pub fn init(fb_info: &FramebufferInfo) {
    let base = fb_info.base_phys as *mut u8;
    let pixel_format = fb_info.pixel_format;
    let fb = Framebuffer::new(
        base,
        fb_info.width as usize,
        fb_info.height as usize,
        fb_info.stride as usize,
        pixel_format,
    );
    unsafe {
        GUI_FB = Some(fb);
    }
    crate::println!(
        "[gui] Framebuffer initialized: {}x{} stride={}",
        fb_info.width,
        fb_info.height,
        fb_info.stride
    );
}

pub fn gui_main_task() -> ! {
    let fb = unsafe { GUI_FB.as_mut().expect("GUI framebuffer not initialized") };

    let scr_w = fb.width;
    let scr_h = fb.height;

    let boot_timer = BootTimer::new();

    let desk = Desktop::new(scr_w, scr_h);
    desk.draw_background(fb);

    let free = crate::mm::pmm::free_pages();
    let total_mb = (crate::mm::pmm::total_pages() * 4096) / (1024 * 1024);
    let _ = free;

    let status = format!(
        "NovusOS v1.0  |  RAM: {} MB  |  {}x{}",
        total_mb, scr_w, scr_h
    );
    desk.draw_status_bar(fb, &status);

    let margin = 40;
    let bar_h = desk.status_bar_height();
    let win = TerminalWindow::new(
        margin,
        bar_h + margin / 2,
        scr_w - 2 * margin,
        scr_h - bar_h - margin,
    );
    win.draw(fb);

    let mut con = Console::new(
        win.content_x(),
        win.content_y(),
        win.content_width(),
        win.content_height(),
        WHITE,
        CONTENT_BG,
    );

    {
        let mut wr = ConsoleFmtWriter { console: &mut con, fb };
        let _ = writeln!(wr, "NovusOS v1.0 -- AArch64 Bare-Metal OS");
        let _ = writeln!(wr, "Framebuffer: {}x{}  RAM: {} MB", scr_w, scr_h, total_mb);
        let _ = writeln!(wr, "Type 'help' for available commands.");
        let _ = writeln!(wr);
    }

    let mut shell = Shell::new();
    shell.draw_prompt(&mut con, fb);

    let sys_ctx = SystemCtx {
        timer: &boot_timer,
        desktop: &desk,
    };

    let mut tick = 0u64;
    loop {
        if let Some(key) = keyboard::poll_key() {
            let quit = shell.handle_key(key, &mut con, fb, &sys_ctx);
            if quit {
                break;
            }
        }

        tick += 1;
        if tick % 1_000_000 == 0 {
            let (h, m, s) = boot_timer.format_uptime();
            let status = format!(
                "NovusOS v1.0  |  RAM: {} MB  |  Uptime: {:02}:{:02}:{:02}",
                total_mb, h, m, s
            );
            desk.draw_status_bar(fb, &status);
        }

        core::hint::spin_loop();
    }

    loop {
        core::hint::spin_loop();
    }
}
