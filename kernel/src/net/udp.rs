extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::sync::spinlock::SpinLock;
use super::ipv4;

type UdpHandler = fn(src_ip: [u8; 4], src_port: u16, data: &[u8]);

static UDP_PORTS: SpinLock<Option<BTreeMap<u16, UdpHandler>>> = SpinLock::new(None);

pub fn init() {
    *UDP_PORTS.lock() = Some(BTreeMap::new());
}

pub fn bind(port: u16, handler: UdpHandler) {
    let mut ports = UDP_PORTS.lock();
    if let Some(ref mut map) = *ports {
        map.insert(port, handler);
    }
}

pub fn send(dst_ip: &[u8; 4], dst_port: u16, src_port: u16, data: &[u8]) -> bool {
    let udp_len = 8 + data.len();
    let mut packet = Vec::with_capacity(udp_len);

    packet.extend_from_slice(&src_port.to_be_bytes());
    packet.extend_from_slice(&dst_port.to_be_bytes());
    packet.extend_from_slice(&(udp_len as u16).to_be_bytes());
    packet.extend_from_slice(&[0x00, 0x00]); // checksum (optional for UDP over IPv4)

    packet.extend_from_slice(data);

    ipv4::send_packet(dst_ip, ipv4::PROTO_UDP, &packet)
}

pub fn handle_udp(ip_packet: &ipv4::Ipv4Packet) {
    let data = ip_packet.payload;
    if data.len() < 8 {
        return;
    }

    let src_port = u16::from_be_bytes([data[0], data[1]]);
    let dst_port = u16::from_be_bytes([data[2], data[3]]);
    let payload = &data[8..];

    let handler = {
        let ports = UDP_PORTS.lock();
        ports.as_ref().and_then(|map| map.get(&dst_port).copied())
    };

    if let Some(handler) = handler {
        handler(ip_packet.src_ip, src_port, payload);
    }
}
