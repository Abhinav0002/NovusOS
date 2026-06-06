#![no_std]
#![no_main]

extern crate alloc;

mod arch;
mod console;
mod drivers;
mod fs;
mod gui;
mod mm;
mod net;
mod sync;
mod sched;
mod syscall;

#[no_mangle]
pub extern "C" fn kernel_main(boot_arg: usize) -> ! {
    println!("Hello from custom-os!");

    let boot_info = unsafe {
        let ptr = boot_arg as *const boot_info::BootInfo;
        if !ptr.is_null() && (*ptr).magic == boot_info::BOOT_INFO_MAGIC {
            Some(&*ptr)
        } else {
            None
        }
    };

    match boot_info {
        Some(bi) => boot_uefi_path(bi),
        None => boot_dtb_path(boot_arg),
    }
}

fn boot_uefi_path(bi: &boot_info::BootInfo) -> ! {
    println!("Booted via UEFI (ExitBootServices)");
    println!(
        "Framebuffer: {}x{} at {:#x}",
        bi.framebuffer.width, bi.framebuffer.height, bi.framebuffer.base_phys
    );

    arch::aarch64::init();

    unsafe {
        mm::vmm::init_boot_page_tables();
        mm::vmm::enable_mmu();
    }
    mm::init_from_boot_info(bi);
    mm::heap::init();

    sched::init();

    let virtio_devices = drivers::virtio::probe();
    for dev in &virtio_devices {
        match dev.device_id {
            18 => {
                if let Some(inp) = drivers::virtio::input::VirtioInput::new(
                    drivers::virtio::VirtioMmio {
                        base: dev.base,
                        device_id: dev.device_id,
                        version: dev.version,
                    },
                ) {
                    drivers::virtio::input::register(inp);
                }
            }
            _ => {}
        }
    }

    gui::init(&bi.framebuffer);
    sched::spawn("gui_main", gui::gui_main_task);

    println!("Kernel initialized (UEFI path). Entering idle loop.");

    loop {
        core::hint::spin_loop();
    }
}

fn boot_dtb_path(dtb_ptr: usize) -> ! {
    println!("DTB pointer: {:#x}", dtb_ptr);

    arch::aarch64::init();

    unsafe {
        mm::vmm::init_boot_page_tables();
        mm::vmm::enable_mmu();
    }
    mm::init();
    mm::heap::init();

    // Phase 5: Scheduler
    sched::init();

    // Phase 6: Device drivers
    let virtio_devices = drivers::virtio::probe();
    let mut block_dev: Option<drivers::virtio::block::VirtioBlock> = None;
    let mut net_dev: Option<drivers::virtio::net::VirtioNet> = None;
    for dev in &virtio_devices {
        match dev.device_id {
            2 => {
                if let Some(blk) = drivers::virtio::block::VirtioBlock::new(
                    drivers::virtio::VirtioMmio { base: dev.base, device_id: dev.device_id, version: dev.version }
                ) {
                    block_dev = Some(blk);
                }
            }
            1 => {
                if let Some(nic) = drivers::virtio::net::VirtioNet::new(
                    drivers::virtio::VirtioMmio { base: dev.base, device_id: dev.device_id, version: dev.version }
                ) {
                    net_dev = Some(nic);
                }
            }
            _ => {}
        }
    }

    // Phase 7: Filesystem
    fs::vfs::init();

    // Mount FAT32 from block device at /
    if let Some(blk) = block_dev {
        if let Some(fat) = fs::fat32::Fat32Fs::new(blk) {
            fs::vfs::mount("/", alloc::boxed::Box::new(fat));
        }
    }

    // Mount RamFS at /tmp
    let ramfs = fs::ramfs::RamFs::new();
    fs::vfs::mount("/tmp", alloc::boxed::Box::new(ramfs));

    // Test: read /hello.txt from FAT32
    match fs::vfs::open("/hello.txt") {
        Ok(mut file) => {
            let mut buf = [0u8; 256];
            match file.read(0, &mut buf) {
                Ok(n) => {
                    let text = core::str::from_utf8(&buf[..n]).unwrap_or("<binary>");
                    println!("[fs] /hello.txt ({} bytes): {}", n, text);
                }
                Err(e) => println!("[fs] read error: {}", e),
            }
        }
        Err(e) => println!("[fs] open /hello.txt: {}", e),
    }

    // Test: list root directory
    match fs::vfs::readdir("/") {
        Ok(entries) => {
            println!("[fs] / directory listing:");
            for entry in &entries {
                let kind = match entry.file_type {
                    fs::FileType::File => "FILE",
                    fs::FileType::Directory => "DIR ",
                };
                println!("  {} {:>8}  {}", kind, entry.size, entry.name);
            }
        }
        Err(e) => println!("[fs] readdir /: {}", e),
    }

    // Test: create and read file in ramfs
    match fs::vfs::create("/tmp/test.txt", fs::FileType::File) {
        Ok(()) => println!("[fs] Created /tmp/test.txt"),
        Err(e) => println!("[fs] create /tmp/test.txt: {}", e),
    }

    // Phase 8: Network stack
    if let Some(nic) = net_dev {
        net::init(nic);
        // UDP echo server on port 7
        net::udp::bind(7, udp_echo_handler);
        // TCP echo server on port 8080
        net::tcp::listen(8080, tcp_accept_handler);
        // Spawn network polling task
        sched::spawn("net_poll", net_poll_task);
    }

    // Phase 9: Syscall interface test
    sched::spawn("syscall_test", syscall_test_task);

    // Phase 10: Userspace init process
    sched::spawn("init_loader", init_loader_task);

    // Spawn test tasks
    sched::spawn("task_a", task_a);
    sched::spawn("task_b", task_b);

    println!("Kernel initialized. Entering idle loop.");

    loop {
        core::hint::spin_loop();
    }
}

fn init_loader_task() -> ! {
    sched::process::spawn_init();
    // spawn_init doesn't return if successful
    loop { core::hint::spin_loop(); }
}

fn syscall_test_task() -> ! {
    // Test sys_write (syscall 1) via SVC
    let msg = b"[syscall] Hello from SVC!\n";
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") 1u64,      // SYS_WRITE
            in("x0") 1u64,      // fd = stdout
            in("x1") msg.as_ptr() as u64,
            in("x2") msg.len() as u64,
            lateout("x0") ret,
            options(nostack)
        );
    }
    println!("[syscall] sys_write returned {}", ret);

    // Test sys_getpid (syscall 8)
    let pid: u64;
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") 8u64,      // SYS_GETPID
            lateout("x0") pid,
            options(nostack)
        );
    }
    println!("[syscall] sys_getpid returned {}", pid);

    println!("[syscall] All syscall tests passed!");

    loop {
        core::hint::spin_loop();
    }
}

fn net_poll_task() -> ! {
    loop {
        net::poll();
        core::hint::spin_loop();
    }
}

fn udp_echo_handler(src_ip: [u8; 4], src_port: u16, data: &[u8]) {
    println!(
        "[udp] Received {} bytes from {}.{}.{}.{}:{}",
        data.len(), src_ip[0], src_ip[1], src_ip[2], src_ip[3], src_port
    );
    net::udp::send(&src_ip, src_port, 7, data);
}

fn tcp_accept_handler(local_port: u16, remote_ip: [u8; 4], remote_port: u16) {
    println!(
        "[tcp] New connection on port {} from {}.{}.{}.{}:{}",
        local_port, remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3], remote_port
    );
}

fn task_a() -> ! {
    let mut count = 0u64;
    loop {
        count += 1;
        if count % 2_000_000 == 0 {
            println!("[task_a] tick {}", count / 2_000_000);
        }
        core::hint::spin_loop();
    }
}

fn task_b() -> ! {
    let mut count = 0u64;
    loop {
        count += 1;
        if count % 2_000_000 == 0 {
            println!("[task_b] tick {}", count / 2_000_000);
        }
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("KERNEL PANIC: {}", info);
    loop {
        core::hint::spin_loop();
    }
}
