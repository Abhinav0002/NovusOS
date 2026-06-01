use alloc::boxed::Box;
use crate::mm::{pmm, PAGE_SIZE};

const KERNEL_STACK_PAGES: usize = 4; // 16 KB per task

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Dead,
}

#[repr(C)]
pub struct TaskContext {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,
    pub lr: u64,
    pub sp: u64,
}

pub type TaskId = u64;

pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub context: TaskContext,
    pub stack_base: usize,
    pub name: [u8; 32],
    pub name_len: usize,
}

impl Task {
    pub fn new(id: TaskId, name: &str, entry: fn() -> !) -> Box<Self> {
        // Allocate kernel stack
        let mut stack_base = 0usize;
        for i in 0..KERNEL_STACK_PAGES {
            let page = pmm::alloc_page().expect("OOM allocating task stack");
            if i == 0 {
                stack_base = page.as_usize();
            }
        }

        let stack_top = stack_base + KERNEL_STACK_PAGES * PAGE_SIZE;

        let mut name_buf = [0u8; 32];
        let len = name.len().min(31);
        name_buf[..len].copy_from_slice(&name.as_bytes()[..len]);

        Box::new(Self {
            id,
            state: TaskState::Ready,
            context: TaskContext {
                x19: 0, x20: 0, x21: 0, x22: 0,
                x23: 0, x24: 0, x25: 0, x26: 0,
                x27: 0, x28: 0,
                x29: 0,
                lr: entry as u64,
                sp: stack_top as u64,
            },
            stack_base,
            name: name_buf,
            name_len: len,
        })
    }

    pub fn new_idle(id: TaskId) -> Box<Self> {
        Box::new(Self {
            id,
            state: TaskState::Running,
            context: TaskContext {
                x19: 0, x20: 0, x21: 0, x22: 0,
                x23: 0, x24: 0, x25: 0, x26: 0,
                x27: 0, x28: 0, x29: 0,
                lr: 0, sp: 0,
            },
            stack_base: 0,
            name: {
                let mut n = [0u8; 32];
                n[..4].copy_from_slice(b"idle");
                n
            },
            name_len: 4,
        })
    }

    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("???")
    }
}
