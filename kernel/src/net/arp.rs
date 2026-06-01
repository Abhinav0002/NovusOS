extern crate alloc;

use alloc::collections::BTreeMap;
use crate::sync::spinlock::SpinLock;
use super::ethernet;

static ARP_TABLE: SpinLock<Option<BTreeMap<[u8; 4], [u8; 6]>>> = SpinLock::new(None);

const ARP_REQUEST: u16 = 1;
const ARP_REPLY: u16 = 2;

pub fn init() {
    *ARP_TABLE.lock() = Some(BTreeMap::new());
}

pub fn lookup(ip: &[u8; 4]) -> Option<[u8; 6]> {
    let table = ARP_TABLE.lock();
    table.as_ref().and_then(|t| t.get(ip).copied())
}

pub fn resolve(ip: &[u8; 4]) -> Option<[u8; 6]> {
    if let Some(mac) = lookup(ip) {
        return Some(mac);
    }

    send_request(ip);

    for _ in 0..100_000 {
        super::poll();
        if let Some(mac) = lookup(ip) {
            return Some(mac);
        }
        core::hint::spin_loop();
    }

    None
}

fn insert(ip: [u8; 4], mac: [u8; 6]) {
    let mut table = ARP_TABLE.lock();
    if let Some(ref mut t) = *table {
        t.insert(ip, mac);
    }
}

pub fn handle_arp(data: &[u8]) {
    if data.len() < 28 {
        return;
    }

    let htype = u16::from_be_bytes([data[0], data[1]]);
    let ptype = u16::from_be_bytes([data[2], data[3]]);
    let hlen = data[4];
    let plen = data[5];
    let oper = u16::from_be_bytes([data[6], data[7]]);

    if htype != 1 || ptype != 0x0800 || hlen != 6 || plen != 4 {
        return;
    }

    let mut sender_mac = [0u8; 6];
    let mut sender_ip = [0u8; 4];
    let mut target_ip = [0u8; 4];

    sender_mac.copy_from_slice(&data[8..14]);
    sender_ip.copy_from_slice(&data[14..18]);
    target_ip.copy_from_slice(&data[24..28]);

    insert(sender_ip, sender_mac);

    if oper == ARP_REQUEST && target_ip == super::OUR_IP {
        send_reply(&sender_mac, &sender_ip);
    }
}

fn send_request(target_ip: &[u8; 4]) {
    let our_mac = super::our_mac();
    let mut arp = [0u8; 28];
    arp[0..2].copy_from_slice(&1u16.to_be_bytes()); // htype: Ethernet
    arp[2..4].copy_from_slice(&0x0800u16.to_be_bytes()); // ptype: IPv4
    arp[4] = 6; // hlen
    arp[5] = 4; // plen
    arp[6..8].copy_from_slice(&ARP_REQUEST.to_be_bytes());
    arp[8..14].copy_from_slice(&our_mac);
    arp[14..18].copy_from_slice(&super::OUR_IP);
    arp[18..24].copy_from_slice(&[0xFF; 6]);
    arp[24..28].copy_from_slice(target_ip);

    let broadcast = [0xFF; 6];
    let frame = ethernet::build_frame(&broadcast, ethernet::ETHERTYPE_ARP, &arp);
    super::send_frame(&frame);
}

fn send_reply(target_mac: &[u8; 6], target_ip: &[u8; 4]) {
    let our_mac = super::our_mac();
    let mut arp = [0u8; 28];
    arp[0..2].copy_from_slice(&1u16.to_be_bytes());
    arp[2..4].copy_from_slice(&0x0800u16.to_be_bytes());
    arp[4] = 6;
    arp[5] = 4;
    arp[6..8].copy_from_slice(&ARP_REPLY.to_be_bytes());
    arp[8..14].copy_from_slice(&our_mac);
    arp[14..18].copy_from_slice(&super::OUR_IP);
    arp[18..24].copy_from_slice(target_mac);
    arp[24..28].copy_from_slice(target_ip);

    let frame = ethernet::build_frame(target_mac, ethernet::ETHERTYPE_ARP, &arp);
    super::send_frame(&frame);
}
