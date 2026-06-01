pub fn sys_exit(code: u64) -> u64 {
    crate::println!("[syscall] exit({})", code);
    loop {
        core::hint::spin_loop();
    }
}

pub fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> u64 {
    if fd == 1 || fd == 2 {
        // stdout / stderr → UART
        let len = len as usize;
        let ptr = buf_ptr as *const u8;
        for i in 0..len {
            let b = unsafe { core::ptr::read(ptr.add(i)) };
            crate::print!("{}", b as char);
        }
        len as u64
    } else {
        (-1i64) as u64
    }
}

pub fn sys_read(fd: u64, _buf_ptr: u64, _len: u64) -> u64 {
    let _ = fd;
    // stdin not implemented yet
    0
}

pub fn sys_open(path_ptr: u64, path_len: u64) -> u64 {
    let _ = (path_ptr, path_len);
    // File descriptor management not implemented yet
    (-1i64) as u64
}

pub fn sys_close(fd: u64) -> u64 {
    let _ = fd;
    0
}

pub fn sys_yield() -> u64 {
    crate::sched::schedule();
    0
}

pub fn sys_getpid() -> u64 {
    // Return current task ID
    0
}

pub fn sys_sleep(ms: u64) -> u64 {
    let _ = ms;
    // Basic sleep: just yield for now
    crate::sched::schedule();
    0
}
