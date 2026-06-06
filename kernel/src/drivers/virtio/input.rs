use super::queue::{Virtqueue, VRING_DESC_F_WRITE};
use super::VirtioMmio;
use crate::gui::keyboard::KeyEvent;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtioInputEvent {
    event_type: u16,
    code: u16,
    value: u32,
}

const EV_KEY: u16 = 1;
const EVENT_BUF_COUNT: usize = 64;

static mut INPUT_DEVICE: Option<VirtioInput> = None;
static mut KEY_RING: [Option<KeyEvent>; 32] = [None; 32];
static mut KEY_RING_HEAD: usize = 0;
static mut KEY_RING_TAIL: usize = 0;

pub struct VirtioInput {
    mmio: VirtioMmio,
    eventq: Virtqueue,
    event_bufs: [VirtioInputEvent; EVENT_BUF_COUNT],
}

unsafe impl Send for VirtioInput {}

impl VirtioInput {
    pub fn new(mmio: VirtioMmio) -> Option<Self> {
        if !mmio.init_device() {
            return None;
        }

        let max = mmio.queue_num_max(0);
        if max == 0 {
            return None;
        }

        let q_size = (max as usize).min(EVENT_BUF_COUNT);
        let eventq = Virtqueue::new(q_size);
        mmio.setup_queue(0, &eventq);
        mmio.driver_ok();

        let mut dev = VirtioInput {
            mmio,
            eventq,
            event_bufs: [VirtioInputEvent::default(); EVENT_BUF_COUNT],
        };

        for i in 0..q_size {
            let buf_ptr = &dev.event_bufs[i] as *const VirtioInputEvent as u64;
            if let Some(desc_idx) = dev.eventq.alloc_desc() {
                unsafe {
                    let d = &mut *dev.eventq.desc.add(desc_idx as usize);
                    d.addr = buf_ptr;
                    d.len = core::mem::size_of::<VirtioInputEvent>() as u32;
                    d.flags = VRING_DESC_F_WRITE;
                    d.next = 0;
                }
                dev.eventq.push_avail(desc_idx);
            }
        }
        dev.mmio.notify(0);

        crate::println!("[virtio] Input device initialized with {} event buffers", q_size);
        Some(dev)
    }
}

pub fn register(dev: VirtioInput) {
    unsafe {
        INPUT_DEVICE = Some(dev);
    }
}

fn enqueue_key(ev: KeyEvent) {
    unsafe {
        let next = (KEY_RING_HEAD + 1) % KEY_RING.len();
        if next != KEY_RING_TAIL {
            KEY_RING[KEY_RING_HEAD] = Some(ev);
            KEY_RING_HEAD = next;
        }
    }
}

fn dequeue_key() -> Option<KeyEvent> {
    unsafe {
        if KEY_RING_TAIL == KEY_RING_HEAD {
            return None;
        }
        let ev = KEY_RING[KEY_RING_TAIL].take();
        KEY_RING_TAIL = (KEY_RING_TAIL + 1) % KEY_RING.len();
        ev
    }
}

fn translate_keycode(code: u16) -> Option<KeyEvent> {
    match code {
        1 => Some(KeyEvent::Escape),
        14 => Some(KeyEvent::Backspace),
        28 => Some(KeyEvent::Enter),
        57 => Some(KeyEvent::Char(' ')),
        103 => Some(KeyEvent::Up),
        108 => Some(KeyEvent::Down),

        2 => Some(KeyEvent::Char('1')),
        3 => Some(KeyEvent::Char('2')),
        4 => Some(KeyEvent::Char('3')),
        5 => Some(KeyEvent::Char('4')),
        6 => Some(KeyEvent::Char('5')),
        7 => Some(KeyEvent::Char('6')),
        8 => Some(KeyEvent::Char('7')),
        9 => Some(KeyEvent::Char('8')),
        10 => Some(KeyEvent::Char('9')),
        11 => Some(KeyEvent::Char('0')),
        12 => Some(KeyEvent::Char('-')),
        13 => Some(KeyEvent::Char('=')),

        16 => Some(KeyEvent::Char('q')),
        17 => Some(KeyEvent::Char('w')),
        18 => Some(KeyEvent::Char('e')),
        19 => Some(KeyEvent::Char('r')),
        20 => Some(KeyEvent::Char('t')),
        21 => Some(KeyEvent::Char('y')),
        22 => Some(KeyEvent::Char('u')),
        23 => Some(KeyEvent::Char('i')),
        24 => Some(KeyEvent::Char('o')),
        25 => Some(KeyEvent::Char('p')),
        26 => Some(KeyEvent::Char('[')),
        27 => Some(KeyEvent::Char(']')),

        30 => Some(KeyEvent::Char('a')),
        31 => Some(KeyEvent::Char('s')),
        32 => Some(KeyEvent::Char('d')),
        33 => Some(KeyEvent::Char('f')),
        34 => Some(KeyEvent::Char('g')),
        35 => Some(KeyEvent::Char('h')),
        36 => Some(KeyEvent::Char('j')),
        37 => Some(KeyEvent::Char('k')),
        38 => Some(KeyEvent::Char('l')),
        39 => Some(KeyEvent::Char(';')),
        40 => Some(KeyEvent::Char('\'')),
        43 => Some(KeyEvent::Char('\\')),

        44 => Some(KeyEvent::Char('z')),
        45 => Some(KeyEvent::Char('x')),
        46 => Some(KeyEvent::Char('c')),
        47 => Some(KeyEvent::Char('v')),
        48 => Some(KeyEvent::Char('b')),
        49 => Some(KeyEvent::Char('n')),
        50 => Some(KeyEvent::Char('m')),
        51 => Some(KeyEvent::Char(',')),
        52 => Some(KeyEvent::Char('.')),
        53 => Some(KeyEvent::Char('/')),

        15 => Some(KeyEvent::Char('\t')),
        41 => Some(KeyEvent::Char('`')),

        _ => None,
    }
}

fn process_events() {
    unsafe {
        let dev = match INPUT_DEVICE.as_mut() {
            Some(d) => d,
            None => return,
        };

        while let Some((desc_id, _len)) = dev.eventq.pop_used() {
            let idx = desc_id as usize;
            if idx < EVENT_BUF_COUNT {
                let ev = dev.event_bufs[idx];
                if ev.event_type == EV_KEY && ev.value == 1 {
                    if let Some(key) = translate_keycode(ev.code) {
                        enqueue_key(key);
                    }
                }

                let buf_ptr = &dev.event_bufs[idx] as *const VirtioInputEvent as u64;
                if let Some(new_desc) = dev.eventq.alloc_desc() {
                    let d = &mut *dev.eventq.desc.add(new_desc as usize);
                    d.addr = buf_ptr;
                    d.len = core::mem::size_of::<VirtioInputEvent>() as u32;
                    d.flags = VRING_DESC_F_WRITE;
                    d.next = 0;
                    dev.eventq.push_avail(new_desc);
                } else {
                    dev.eventq.free_desc(desc_id as u16);
                    let d = &mut *dev.eventq.desc.add(desc_id as usize);
                    d.addr = buf_ptr;
                    d.len = core::mem::size_of::<VirtioInputEvent>() as u32;
                    d.flags = VRING_DESC_F_WRITE;
                    d.next = 0;
                    dev.eventq.push_avail(desc_id as u16);
                }
            }
            dev.mmio.notify(0);
            dev.mmio.ack_interrupt();
        }
    }
}

pub fn poll_key() -> Option<KeyEvent> {
    process_events();
    dequeue_key()
}
