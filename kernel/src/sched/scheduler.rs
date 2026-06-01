use alloc::boxed::Box;
use alloc::collections::VecDeque;
use core::arch::global_asm;
use core::sync::atomic::{AtomicBool, Ordering};
use super::task::{Task, TaskState, TaskId, TaskContext};

struct SchedulerInner {
    tasks: VecDeque<Box<Task>>,
    current: Option<Box<Task>>,
    next_id: TaskId,
}

static mut SCHEDULER: Option<SchedulerInner> = None;
static SCHEDULER_ACTIVE: AtomicBool = AtomicBool::new(false);

global_asm!(
    "
    .global context_switch
    context_switch:
        // x0 = prev TaskContext*, x1 = next TaskContext*

        stp     x19, x20, [x0, #0]
        stp     x21, x22, [x0, #16]
        stp     x23, x24, [x0, #32]
        stp     x25, x26, [x0, #48]
        stp     x27, x28, [x0, #64]
        stp     x29, x30, [x0, #80]
        mov     x2, sp
        str     x2, [x0, #96]

        ldp     x19, x20, [x1, #0]
        ldp     x21, x22, [x1, #16]
        ldp     x23, x24, [x1, #32]
        ldp     x25, x26, [x1, #48]
        ldp     x27, x28, [x1, #64]
        ldp     x29, x30, [x1, #80]
        ldr     x2, [x1, #96]
        mov     sp, x2

        ret
    "
);

extern "C" {
    fn context_switch(prev: *mut TaskContext, next: *const TaskContext);
}

// Trampoline for new tasks: enables interrupts then calls the real entry point
// The entry point address is passed in x19 (callee-saved, survives context_switch)
global_asm!(
    "
    .global task_entry_trampoline
    task_entry_trampoline:
        msr     DAIFClr, #0b0010
        mov     x0, x19
        blr     x0
        b       .
    "
);

extern "C" {
    fn task_entry_trampoline();
}

pub fn init() {
    unsafe {
        SCHEDULER = Some(SchedulerInner {
            tasks: VecDeque::new(),
            current: Some(Task::new_idle(0)),
            next_id: 1,
        });
    }
    SCHEDULER_ACTIVE.store(true, Ordering::Release);
    crate::println!("[sched] Scheduler initialized");
}

pub fn spawn(name: &str, entry: fn() -> !) {
    unsafe {
        let sched = SCHEDULER.as_mut().unwrap();
        let id = sched.next_id;
        sched.next_id += 1;
        let mut task = Task::new(id, name, entry);
        // Set x19 to the real entry point, lr to the trampoline
        task.context.x19 = entry as u64;
        task.context.lr = task_entry_trampoline as u64;
        crate::println!("[sched] Spawned task {} (id={})", name, id);
        sched.tasks.push_back(task);
    }
}

pub fn schedule() {
    if !SCHEDULER_ACTIVE.load(Ordering::Relaxed) {
        return;
    }

    unsafe {
        // Disable interrupts during scheduling
        core::arch::asm!("msr DAIFSet, #0b1111");

        let sched = match SCHEDULER.as_mut() {
            Some(s) => s,
            None => {
                core::arch::asm!("msr DAIFClr, #0b0010");
                return;
            }
        };

        if sched.tasks.is_empty() {
            core::arch::asm!("msr DAIFClr, #0b0010");
            return;
        }

        let next = match sched.tasks.pop_front() {
            Some(t) => t,
            None => {
                core::arch::asm!("msr DAIFClr, #0b0010");
                return;
            }
        };

        let mut prev = sched.current.take().unwrap();
        if prev.state != TaskState::Dead {
            prev.state = TaskState::Ready;
            sched.tasks.push_back(prev);
        }

        let prev_task = sched.tasks.back_mut().unwrap();
        sched.current = Some(next);
        let current = sched.current.as_mut().unwrap();
        current.state = TaskState::Running;

        let prev_ctx = &mut prev_task.context as *mut TaskContext;
        let next_ctx = &current.context as *const TaskContext;

        context_switch(prev_ctx, next_ctx);

        // After switching back to this task, re-enable interrupts
        core::arch::asm!("msr DAIFClr, #0b0010");
    }
}

pub fn current_task_name() -> [u8; 32] {
    unsafe {
        match SCHEDULER.as_ref() {
            Some(sched) => match &sched.current {
                Some(t) => t.name,
                None => [0u8; 32],
            },
            None => [0u8; 32],
        }
    }
}
