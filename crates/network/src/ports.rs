//! Blocked-port table, mirroring the WHATWG Fetch standard's "bad port"
//! list (a public, standard cross-protocol-smuggling defense: refuse to
//! originate requests to ports conventionally reserved for other
//! protocols, e.g. SMTP, DNS, or common internal-service ports).

const BLOCKED_PORTS: [u16; 33] = [
    1, 7, 9, 11, 13, 15, 17, 19, 20, 21, 22, 23, 25, 37, 42, 43, 53, 69, 77, 79, 87, 95, 101, 102,
    103, 104, 109, 110, 111, 113, 115, 117, 119,
];

const BLOCKED_PORTS_EXTRA: [u16; 20] = [
    123, 135, 139, 143, 161, 179, 389, 427, 465, 512, 513, 514, 515, 526, 530, 531, 532, 540, 548,
    554,
];

const BLOCKED_PORTS_SERVICES: [u16; 20] = [
    556, 563, 587, 601, 636, 989, 990, 993, 995, 1719, 1720, 1723, 2049, 3659, 4045, 5060, 5061,
    6000, 6566, 6665,
];

const BLOCKED_PORTS_MORE: [u16; 6] = [6666, 6667, 6668, 6669, 6697, 10080];

pub fn is_blocked_port(port: u16) -> bool {
    BLOCKED_PORTS.contains(&port)
        || BLOCKED_PORTS_EXTRA.contains(&port)
        || BLOCKED_PORTS_SERVICES.contains(&port)
        || BLOCKED_PORTS_MORE.contains(&port)
}

#[cfg(test)]
mod tests {
    use super::is_blocked_port;

    #[test]
    fn blocks_well_known_smtp_and_dns_ports() {
        assert!(is_blocked_port(25));
        assert!(is_blocked_port(53));
        assert!(!is_blocked_port(443));
        assert!(!is_blocked_port(80));
        assert!(!is_blocked_port(8080));
    }
}
