extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::sync::spinlock::SpinLock;
use super::ipv4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpState {
    Listen,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    LastAck,
    Closed,
}

const TH_FIN: u8 = 0x01;
const TH_SYN: u8 = 0x02;
const TH_RST: u8 = 0x04;
const TH_PSH: u8 = 0x08;
const TH_ACK: u8 = 0x10;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TcpKey {
    remote_ip: [u8; 4],
    remote_port: u16,
    local_port: u16,
}

struct TcpConnection {
    state: TcpState,
    snd_nxt: u32,
    snd_una: u32,
    rcv_nxt: u32,
    local_port: u16,
    remote_port: u16,
    remote_ip: [u8; 4],
    rx_buf: Vec<u8>,
}

type TcpAcceptHandler = fn(local_port: u16, remote_ip: [u8; 4], remote_port: u16);

static TCP_STATE: SpinLock<Option<TcpGlobal>> = SpinLock::new(None);

struct TcpGlobal {
    connections: BTreeMap<TcpKey, TcpConnection>,
    listeners: BTreeMap<u16, TcpAcceptHandler>,
    isn_counter: u32,
}

pub fn init() {
    *TCP_STATE.lock() = Some(TcpGlobal {
        connections: BTreeMap::new(),
        listeners: BTreeMap::new(),
        isn_counter: 1000,
    });
}

pub fn listen(port: u16, handler: TcpAcceptHandler) {
    let mut state = TCP_STATE.lock();
    if let Some(ref mut s) = *state {
        s.listeners.insert(port, handler);
        crate::println!("[tcp] Listening on port {}", port);
    }
}

pub fn handle_tcp(ip_packet: &ipv4::Ipv4Packet) {
    let data = ip_packet.payload;
    if data.len() < 20 {
        return;
    }

    let src_port = u16::from_be_bytes([data[0], data[1]]);
    let dst_port = u16::from_be_bytes([data[2], data[3]]);
    let seq_num = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let ack_num = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let data_offset = ((data[12] >> 4) as usize) * 4;
    let flags = data[13];

    if data_offset > data.len() {
        return;
    }

    let payload = &data[data_offset..];
    let key = TcpKey {
        remote_ip: ip_packet.src_ip,
        remote_port: src_port,
        local_port: dst_port,
    };

    let mut state = TCP_STATE.lock();
    let s = match state.as_mut() {
        Some(s) => s,
        None => return,
    };

    if let Some(conn) = s.connections.get_mut(&key) {
        match conn.state {
            TcpState::SynReceived => {
                if flags & TH_ACK != 0 {
                    conn.state = TcpState::Established;
                    conn.snd_una = ack_num;
                    crate::println!("[tcp] Connection established {}:{}",
                        format_ip(&ip_packet.src_ip), src_port);
                }
            }
            TcpState::Established => {
                if flags & TH_FIN != 0 {
                    conn.rcv_nxt = seq_num.wrapping_add(1);
                    conn.state = TcpState::CloseWait;
                    send_tcp_packet(conn, TH_ACK, &[]);
                    conn.state = TcpState::LastAck;
                    send_tcp_packet(conn, TH_FIN | TH_ACK, &[]);
                    return;
                }

                if !payload.is_empty() {
                    conn.rcv_nxt = seq_num.wrapping_add(payload.len() as u32);
                    conn.rx_buf.extend_from_slice(payload);
                    send_tcp_packet(conn, TH_ACK, &[]);

                    crate::println!("[tcp] Received {} bytes on port {}", payload.len(), dst_port);

                    // Echo back for testing
                    let echo = conn.rx_buf.clone();
                    conn.rx_buf.clear();
                    send_tcp_packet(conn, TH_ACK | TH_PSH, &echo);
                } else if flags & TH_ACK != 0 {
                    conn.snd_una = ack_num;
                }
            }
            TcpState::LastAck => {
                if flags & TH_ACK != 0 {
                    conn.state = TcpState::Closed;
                }
            }
            TcpState::FinWait1 => {
                if flags & TH_ACK != 0 {
                    conn.state = TcpState::FinWait2;
                }
            }
            TcpState::FinWait2 => {
                if flags & TH_FIN != 0 {
                    conn.rcv_nxt = seq_num.wrapping_add(1);
                    send_tcp_packet(conn, TH_ACK, &[]);
                    conn.state = TcpState::Closed;
                }
            }
            _ => {}
        }
        return;
    }

    // No existing connection — check listeners for SYN
    if flags & TH_SYN != 0 && flags & TH_ACK == 0 {
        if let Some(&handler) = s.listeners.get(&dst_port) {
            let isn = s.isn_counter;
            s.isn_counter = s.isn_counter.wrapping_add(64000);

            let mut conn = TcpConnection {
                state: TcpState::SynReceived,
                snd_nxt: isn.wrapping_add(1),
                snd_una: isn,
                rcv_nxt: seq_num.wrapping_add(1),
                local_port: dst_port,
                remote_port: src_port,
                remote_ip: ip_packet.src_ip,
                rx_buf: Vec::new(),
            };

            send_tcp_packet(&mut conn, TH_SYN | TH_ACK, &[]);
            s.connections.insert(key, conn);
            handler(dst_port, ip_packet.src_ip, src_port);
        } else {
            // Send RST for unlistened port
            send_rst(&ip_packet.src_ip, src_port, dst_port, ack_num, seq_num.wrapping_add(1));
        }
    }
}

fn send_tcp_packet(conn: &mut TcpConnection, flags: u8, payload: &[u8]) {
    let header_len = 20u8;
    let total_len = header_len as usize + payload.len();
    let mut segment = Vec::with_capacity(total_len);

    segment.extend_from_slice(&conn.local_port.to_be_bytes());
    segment.extend_from_slice(&conn.remote_port.to_be_bytes());
    segment.extend_from_slice(&conn.snd_nxt.to_be_bytes());
    segment.extend_from_slice(&conn.rcv_nxt.to_be_bytes());
    segment.push((header_len / 4) << 4); // data offset
    segment.push(flags);
    segment.extend_from_slice(&8192u16.to_be_bytes()); // window
    segment.extend_from_slice(&[0, 0]); // checksum placeholder
    segment.extend_from_slice(&[0, 0]); // urgent pointer
    segment.extend_from_slice(payload);

    // TCP checksum with pseudo-header
    let cksum = tcp_checksum(&super::OUR_IP, &conn.remote_ip, &segment);
    segment[16] = (cksum >> 8) as u8;
    segment[17] = (cksum & 0xFF) as u8;

    if flags & TH_SYN != 0 || flags & TH_FIN != 0 {
        conn.snd_nxt = conn.snd_nxt.wrapping_add(1);
    }
    conn.snd_nxt = conn.snd_nxt.wrapping_add(payload.len() as u32);

    ipv4::send_packet(&conn.remote_ip, ipv4::PROTO_TCP, &segment);
}

fn send_rst(dst_ip: &[u8; 4], dst_port: u16, src_port: u16, seq: u32, ack: u32) {
    let mut segment = Vec::with_capacity(20);
    segment.extend_from_slice(&src_port.to_be_bytes());
    segment.extend_from_slice(&dst_port.to_be_bytes());
    segment.extend_from_slice(&seq.to_be_bytes());
    segment.extend_from_slice(&ack.to_be_bytes());
    segment.push(5 << 4); // data offset
    segment.push(TH_RST | TH_ACK);
    segment.extend_from_slice(&0u16.to_be_bytes()); // window
    segment.extend_from_slice(&[0, 0]); // checksum
    segment.extend_from_slice(&[0, 0]); // urgent

    let cksum = tcp_checksum(&super::OUR_IP, dst_ip, &segment);
    segment[16] = (cksum >> 8) as u8;
    segment[17] = (cksum & 0xFF) as u8;

    ipv4::send_packet(dst_ip, ipv4::PROTO_TCP, &segment);
}

fn tcp_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], segment: &[u8]) -> u16 {
    let mut pseudo = Vec::with_capacity(12 + segment.len());
    pseudo.extend_from_slice(src_ip);
    pseudo.extend_from_slice(dst_ip);
    pseudo.push(0);
    pseudo.push(ipv4::PROTO_TCP);
    pseudo.extend_from_slice(&(segment.len() as u16).to_be_bytes());
    pseudo.extend_from_slice(segment);
    ipv4::checksum(&pseudo)
}

fn format_ip(ip: &[u8; 4]) -> alloc::string::String {
    alloc::format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}
