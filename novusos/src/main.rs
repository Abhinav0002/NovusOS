#![no_main]
#![no_std]

extern crate alloc;

mod color;
mod commands;
mod console;
mod desktop;
mod font;
mod framebuffer;
mod keyboard;
mod memory;
mod shell;
mod timer;
mod window;

use alloc::format;
use core::fmt::Write;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use framebuffer::Framebuffer;
use commands::SystemCtx;
use console::{Console, ConsoleFmtWriter};
use desktop::Desktop;
use shell::Shell;
use timer::BootTimer;
use window::TerminalWindow;
use color::*;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let fb = init_gop();
    let mut fb = match fb {
        Some(fb) => fb,
        None => {
            log::error!("Failed to initialize GOP framebuffer");
            boot::stall(5_000_000);
            return Status::ABORTED;
        }
    };

    let scr_w = fb.width;
    let scr_h = fb.height;

    let boot_timer = BootTimer::new();
    let mem_info = memory::query_memory();

    let desk = Desktop::new(scr_w, scr_h);
    desk.draw_background(&mut fb);

    let status = format!(
        "NovusOS v1.0  |  RAM: {} MB  |  {}x{}",
        mem_info.total_mb(), scr_w, scr_h
    );
    desk.draw_status_bar(&mut fb, &status);

    let margin = 40;
    let bar_h = desk.status_bar_height();
    let win = TerminalWindow::new(
        margin,
        bar_h + margin / 2,
        scr_w - 2 * margin,
        scr_h - bar_h - margin,
    );
    win.draw(&mut fb);

    let mut con = Console::new(
        win.content_x(),
        win.content_y(),
        win.content_width(),
        win.content_height(),
        WHITE,
        CONTENT_BG,
    );

    {
        let mut wr = ConsoleFmtWriter { console: &mut con, fb: &mut fb };
        let _ = writeln!(wr, "NovusOS v1.0 -- AArch64 UEFI Graphical OS");
        let _ = writeln!(wr, "Framebuffer: {}x{}  RAM: {} MB", scr_w, scr_h, mem_info.total_mb());
        let _ = writeln!(wr, "Type 'help' for available commands.");
        let _ = writeln!(wr);
    }

    let mut shell = Shell::new();
    shell.draw_prompt(&mut con, &mut fb);

    let sys_ctx = SystemCtx {
        timer: &boot_timer,
        desktop: &desk,
    };

    let mut tick = 0u64;
    loop {
        if let Some(key) = keyboard::poll_key() {
            let quit = shell.handle_key(key, &mut con, &mut fb, &sys_ctx);
            if quit {
                break;
            }
        }

        tick += 1;
        if tick % 1000 == 0 {
            let (h, m, s) = boot_timer.format_uptime();
            let status = format!(
                "NovusOS v1.0  |  RAM: {} MB  |  Uptime: {:02}:{:02}:{:02}",
                mem_info.total_mb(), h, m, s
            );
            desk.draw_status_bar(&mut fb, &status);
        }

        boot::stall(1_000);
    }

    Status::SUCCESS
}

fn init_gop() -> Option<Framebuffer> {
    let handle = boot::get_handle_for_protocol::<GraphicsOutput>().ok()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle).ok()?;

    let mut best_mode = None;
    let mut best_res = (0usize, 0usize);

    for mode in gop.modes() {
        let info = mode.info();
        let (w, h) = info.resolution();
        if w == 1024 && h == 768 {
            best_mode = Some(mode);
            break;
        }
        if w * h > best_res.0 * best_res.1 {
            best_mode = Some(mode);
            best_res = (w, h);
        }
    }

    if let Some(mode) = best_mode {
        let _ = gop.set_mode(&mode);
    }

    let mode_info = gop.current_mode_info();
    let (width, height) = mode_info.resolution();
    let stride = mode_info.stride();
    let pixel_format = mode_info.pixel_format();

    let mut frame_buffer = gop.frame_buffer();
    let base = frame_buffer.as_mut_ptr();

    Some(Framebuffer::new(base, width, height, stride, pixel_format))
}
