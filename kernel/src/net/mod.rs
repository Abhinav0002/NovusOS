pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod tcp;

use crate::drivers::virtio::net::VirtioNet;
use crate::sync::spinlock::SpinLock;

pub const OUR_IP: [u8; 4] = [10, 0, 2, 15];
pub const GATEWAY_IP: [u8; 4] = [10, 0, 2, 2];
pub const SUBNET_MASK: [u8; 4] = [255, 255, 255, 0];

static NET_DEVICE: SpinLock<Option<VirtioNet>> = SpinLock::new(None);

pub fn init(dev: VirtioNet) {
    let mac = dev.mac;
    *NET_DEVICE.lock() = Some(dev);
    crate::println!(
        "[net] Stack initialized, IP {}.{}.{}.{}, MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        OUR_IP[0], OUR_IP[1], OUR_IP[2], OUR_IP[3],
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    arp::init();
    udp::init();
    tcp::init();
}

pub fn our_mac() -> [u8; 6] {
    let dev = NET_DEVICE.lock();
    dev.as_ref().map(|d| d.mac).unwrap_or([0; 6])
}

pub fn send_frame(frame: &[u8]) -> bool {
    let mut dev = NET_DEVICE.lock();
    match dev.as_mut() {
        Some(d) => d.send(frame),
        None => false,
    }
}

pub fn poll() {
    let frame = {
        let mut dev = NET_DEVICE.lock();
        match dev.as_mut() {
            Some(d) => d.recv(),
            None => None,
        }
    };

    if let Some(frame) = frame {
        ethernet::handle_frame(&frame);
    }
}
