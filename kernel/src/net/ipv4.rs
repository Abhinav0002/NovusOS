use alloc::vec::Vec;
use super::ethernet;

pub struct Ipv4Packet<'a> {
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
    pub protocol: u8,
    pub payload: &'a [u8],
    pub header: &'a [u8],
}

pub const PROTO_ICMP: u8 = 1;
pub const PROTO_TCP: u8 = 6;
pub const PROTO_UDP: u8 = 17;

impl<'a> Ipv4Packet<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let version = data[0] >> 4;
        if version != 4 {
            return None;
        }

        let ihl = (data[0] & 0x0F) as usize * 4;
        if ihl < 20 || data.len() < ihl {
            return None;
        }

        let total_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        if data.len() < total_len {
            return None;
        }

        let protocol = data[9];
        let mut src_ip = [0u8; 4];
        let mut dst_ip = [0u8; 4];
        src_ip.copy_from_slice(&data[12..16]);
        dst_ip.copy_from_slice(&data[16..20]);

        Some(Self {
            src_ip,
            dst_ip,
            protocol,
            payload: &data[ihl..total_len],
            header: &data[..ihl],
        })
    }
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn send_packet(dst_ip: &[u8; 4], protocol: u8, payload: &[u8]) -> bool {
    let total_len = 20 + payload.len();
    let mut packet = Vec::with_capacity(total_len);

    // IPv4 header
    packet.push(0x45); // version=4, IHL=5
    packet.push(0x00); // DSCP/ECN
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.extend_from_slice(&[0x00, 0x00]); // identification
    packet.extend_from_slice(&[0x40, 0x00]); // flags=DF, fragment offset=0
    packet.push(64); // TTL
    packet.push(protocol);
    packet.extend_from_slice(&[0x00, 0x00]); // checksum placeholder
    packet.extend_from_slice(&super::OUR_IP);
    packet.extend_from_slice(dst_ip);

    // Calculate header checksum
    let cksum = checksum(&packet[..20]);
    packet[10] = (cksum >> 8) as u8;
    packet[11] = (cksum & 0xFF) as u8;

    packet.extend_from_slice(payload);

    let next_hop = if same_subnet(dst_ip) { *dst_ip } else { super::GATEWAY_IP };
    let dst_mac = match super::arp::resolve(&next_hop) {
        Some(mac) => mac,
        None => {
            crate::println!("[ipv4] ARP resolution failed for {}.{}.{}.{}",
                next_hop[0], next_hop[1], next_hop[2], next_hop[3]);
            return false;
        }
    };

    let frame = ethernet::build_frame(&dst_mac, ethernet::ETHERTYPE_IPV4, &packet);
    super::send_frame(&frame)
}

fn same_subnet(ip: &[u8; 4]) -> bool {
    for i in 0..4 {
        if (ip[i] & super::SUBNET_MASK[i]) != (super::OUR_IP[i] & super::SUBNET_MASK[i]) {
            return false;
        }
    }
    true
}

pub fn handle_ipv4(data: &[u8], _src_mac: &[u8; 6]) {
    let packet = match Ipv4Packet::parse(data) {
        Some(p) => p,
        None => return,
    };

    if packet.dst_ip != super::OUR_IP {
        return;
    }

    match packet.protocol {
        PROTO_ICMP => super::icmp::handle_icmp(&packet),
        PROTO_UDP => super::udp::handle_udp(&packet),
        PROTO_TCP => super::tcp::handle_tcp(&packet),
        _ => {}
    }
}
