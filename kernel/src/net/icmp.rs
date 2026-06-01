use super::ipv4;

const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_ECHO_REPLY: u8 = 0;

pub fn handle_icmp(packet: &ipv4::Ipv4Packet) {
    if packet.payload.len() < 8 {
        return;
    }

    let icmp_type = packet.payload[0];
    let icmp_code = packet.payload[1];

    if icmp_type == ICMP_ECHO_REQUEST && icmp_code == 0 {
        send_echo_reply(packet);
    }
}

fn send_echo_reply(request: &ipv4::Ipv4Packet) {
    let mut reply = alloc::vec![0u8; request.payload.len()];
    reply.copy_from_slice(request.payload);

    reply[0] = ICMP_ECHO_REPLY;
    reply[1] = 0;
    // Zero checksum before recalculating
    reply[2] = 0;
    reply[3] = 0;

    let cksum = ipv4::checksum(&reply);
    reply[2] = (cksum >> 8) as u8;
    reply[3] = (cksum & 0xFF) as u8;

    ipv4::send_packet(&request.src_ip, ipv4::PROTO_ICMP, &reply);
}
