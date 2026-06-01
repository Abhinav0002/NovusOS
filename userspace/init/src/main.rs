#![no_std]
#![no_main]

mod syscall {
    #[inline(always)]
    pub fn write(fd: u64, buf: &[u8]) -> u64 {
        let ret: u64;
        unsafe {
            core::arch::asm!(
                "svc #0",
                in("x8") 1u64,
                in("x0") fd,
                in("x1") buf.as_ptr() as u64,
                in("x2") buf.len() as u64,
                lateout("x0") ret,
                options(nostack)
            );
        }
        ret
    }

    #[inline(always)]
    pub fn r#yield() -> u64 {
        let ret: u64;
        unsafe {
            core::arch::asm!(
                "svc #0",
                in("x8") 7u64,
                lateout("x0") ret,
                options(nostack)
            );
        }
        ret
    }

    #[inline(always)]
    pub fn getpid() -> u64 {
        let ret: u64;
        unsafe {
            core::arch::asm!(
                "svc #0",
                in("x8") 8u64,
                lateout("x0") ret,
                options(nostack)
            );
        }
        ret
    }

    #[inline(always)]
    pub fn exit(code: u64) -> ! {
        unsafe {
            core::arch::asm!(
                "svc #0",
                in("x8") 0u64,
                in("x0") code,
                options(nostack, noreturn)
            );
        }
    }
}

#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello from userspace!\n";
    syscall::write(1, msg);

    let pid = syscall::getpid();
    // Can't easily format numbers without alloc, so just write PID as single digit
    let digit = b'0' + (pid as u8 % 10);
    let pid_msg = [b'[', b'i', b'n', b'i', b't', b']', b' ', b'p', b'i', b'd', b'=', digit, b'\n'];
    syscall::write(1, &pid_msg);

    let mut count = 0u64;
    loop {
        count += 1;
        if count % 5_000_000 == 0 {
            let tick = b'0' + ((count / 5_000_000) as u8 % 10);
            let tick_msg = [b'[', b'i', b'n', b'i', b't', b']', b' ', b't', b'i', b'c', b'k', b' ', tick, b'\n'];
            syscall::write(1, &tick_msg);
            syscall::r#yield();
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    syscall::exit(1);
}
