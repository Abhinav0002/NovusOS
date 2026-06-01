pub mod handlers;

use crate::arch::aarch64::exceptions::ExceptionContext;

pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_OPEN: u64 = 3;
pub const SYS_CLOSE: u64 = 4;
pub const SYS_YIELD: u64 = 7;
pub const SYS_GETPID: u64 = 8;
pub const SYS_SLEEP: u64 = 9;

pub fn dispatch(ctx: &mut ExceptionContext) {
    let syscall_num = ctx.gpr[8]; // x8
    let a0 = ctx.gpr[0]; // x0
    let a1 = ctx.gpr[1]; // x1
    let a2 = ctx.gpr[2]; // x2

    let result = match syscall_num {
        SYS_EXIT => handlers::sys_exit(a0),
        SYS_WRITE => handlers::sys_write(a0, a1, a2),
        SYS_READ => handlers::sys_read(a0, a1, a2),
        SYS_OPEN => handlers::sys_open(a0, a1),
        SYS_CLOSE => handlers::sys_close(a0),
        SYS_YIELD => handlers::sys_yield(),
        SYS_GETPID => handlers::sys_getpid(),
        SYS_SLEEP => handlers::sys_sleep(a0),
        _ => {
            crate::println!("[syscall] Unknown syscall {}", syscall_num);
            (-1i64) as u64
        }
    };

    ctx.gpr[0] = result;
}
