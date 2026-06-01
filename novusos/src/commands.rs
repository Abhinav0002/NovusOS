use crate::color::*;
use crate::console::{Console, ConsoleFmtWriter};
use crate::desktop::Desktop;
use crate::framebuffer::Framebuffer;
use crate::memory;
use crate::timer::BootTimer;
use core::fmt::Write;

pub struct SystemCtx<'a> {
    pub timer: &'a BootTimer,
    pub desktop: &'a Desktop,
}

pub fn execute(
    input: &str,
    con: &mut Console,
    fb: &mut Framebuffer,
    ctx: &SystemCtx,
) -> bool {
    let mut parts = input.split_whitespace();
    let cmd = match parts.next() {
        Some(c) => c,
        None => return false,
    };
    let args: &str = input[cmd.len()..].trim();

    match cmd {
        "help" => cmd_help(con, fb),
        "clear" => cmd_clear(con, fb),
        "echo" => cmd_echo(args, con, fb),
        "ver" | "version" => cmd_ver(con, fb),
        "mem" | "memory" => cmd_mem(con, fb),
        "uptime" => cmd_uptime(con, fb, ctx),
        "cpuinfo" | "cpu" => cmd_cpuinfo(con, fb),
        "color" => cmd_color(args, fb, ctx),
        "reboot" => cmd_reboot(),
        "shutdown" | "halt" => cmd_shutdown(),
        _ => {
            let mut w = ConsoleFmtWriter { console: con, fb };
            let _ = writeln!(w, "Unknown command: '{}'. Type 'help' for commands.", cmd);
        }
    }
    false
}

fn cmd_help(con: &mut Console, fb: &mut Framebuffer) {
    let mut w = ConsoleFmtWriter { console: con, fb };
    let _ = writeln!(w, "NovusOS Commands:");
    let _ = writeln!(w, "  help          Show this help message");
    let _ = writeln!(w, "  ver           Show version information");
    let _ = writeln!(w, "  echo <text>   Print text");
    let _ = writeln!(w, "  clear         Clear the terminal");
    let _ = writeln!(w, "  mem           Show memory information");
    let _ = writeln!(w, "  uptime        Show system uptime");
    let _ = writeln!(w, "  cpuinfo       Show CPU information");
    let _ = writeln!(w, "  color r g b   Change desktop background");
    let _ = writeln!(w, "  reboot        Restart the system");
    let _ = writeln!(w, "  shutdown      Power off the system");
}

fn cmd_clear(con: &mut Console, fb: &mut Framebuffer) {
    con.clear(fb);
}

fn cmd_echo(args: &str, con: &mut Console, fb: &mut Framebuffer) {
    let mut w = ConsoleFmtWriter { console: con, fb };
    let _ = writeln!(w, "{}", args);
}

fn cmd_ver(con: &mut Console, fb: &mut Framebuffer) {
    let mut w = ConsoleFmtWriter { console: con, fb };
    let _ = writeln!(w, "NovusOS v1.0");
    let _ = writeln!(w, "Architecture: AArch64 (ARM64)");
    let _ = writeln!(w, "Boot: UEFI Application");
    let _ = writeln!(w, "Display: GOP Framebuffer");
}

fn cmd_mem(con: &mut Console, fb: &mut Framebuffer) {
    let info = memory::query_memory();
    let mut w = ConsoleFmtWriter { console: con, fb };
    let _ = writeln!(w, "Memory Information:");
    let _ = writeln!(w, "  Total: {} MB ({} pages)", info.total_mb(), info.total_pages);
    let _ = writeln!(w, "  Available: {} MB ({} pages)", info.free_mb(), info.free_pages);
}

fn cmd_uptime(con: &mut Console, fb: &mut Framebuffer, ctx: &SystemCtx) {
    let (h, m, s) = ctx.timer.format_uptime();
    let mut w = ConsoleFmtWriter { console: con, fb };
    let _ = writeln!(w, "Uptime: {:02}:{:02}:{:02}", h, m, s);
}

fn cmd_cpuinfo(con: &mut Console, fb: &mut Framebuffer) {
    let midr: u64;
    unsafe { core::arch::asm!("mrs {}, MIDR_EL1", out(reg) midr) };

    let implementer = ((midr >> 24) & 0xFF) as u8;
    let variant = ((midr >> 20) & 0xF) as u8;
    let architecture = ((midr >> 16) & 0xF) as u8;
    let part = ((midr >> 4) & 0xFFF) as u16;
    let revision = (midr & 0xF) as u8;

    let impl_name = match implementer {
        0x41 => "ARM",
        0x42 => "Broadcom",
        0x43 => "Cavium",
        0x44 => "DEC",
        0x4E => "NVIDIA",
        0x51 => "Qualcomm",
        0x56 => "Marvell",
        0x61 => "Apple",
        _ => "Unknown",
    };

    let part_name = match (implementer, part) {
        (0x41, 0xD08) => "Cortex-A72",
        (0x41, 0xD09) => "Cortex-A73",
        (0x41, 0xD0A) => "Cortex-A75",
        (0x41, 0xD0B) => "Cortex-A76",
        (0x41, 0xD0C) => "Neoverse-N1",
        (0x41, 0xD05) => "Cortex-A55",
        (0x41, 0xD03) => "Cortex-A53",
        _ => "Unknown",
    };

    let mut w = ConsoleFmtWriter { console: con, fb };
    let _ = writeln!(w, "CPU Information:");
    let _ = writeln!(w, "  Implementer: {} (0x{:02X})", impl_name, implementer);
    let _ = writeln!(w, "  Part: {} (0x{:03X})", part_name, part);
    let _ = writeln!(w, "  Variant: {}, Revision: {}", variant, revision);
    let _ = writeln!(w, "  Architecture: 0x{:X}", architecture);
}

fn cmd_color(args: &str, fb: &mut Framebuffer, ctx: &SystemCtx) {
    let parts: alloc::vec::Vec<&str> = args.split_whitespace().collect();
    if parts.len() != 3 {
        return;
    }
    let r: u8 = parts[0].parse().unwrap_or(20);
    let g: u8 = parts[1].parse().unwrap_or(30);
    let b: u8 = parts[2].parse().unwrap_or(60);

    let new_bg = Color::new(r, g, b);
    for y in ctx.desktop.status_bar_height()..ctx.desktop.height {
        fb.fill_rect(0, y, ctx.desktop.width, 1, new_bg);
    }
}

fn cmd_reboot() -> ! {
    uefi::runtime::reset(uefi::runtime::ResetType::COLD, uefi::Status::SUCCESS, None);
}

fn cmd_shutdown() -> ! {
    uefi::runtime::reset(uefi::runtime::ResetType::SHUTDOWN, uefi::Status::SUCCESS, None);
}
