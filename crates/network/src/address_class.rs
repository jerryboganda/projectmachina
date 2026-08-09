//! Destination-address classification: the concrete address-class table
//! from `.agent-state/design/M2-T02-security-review.md` section 1. Every
//! function here operates on a canonical `IpAddr`, never a raw host string
//! -- this is the one rule that prevents string-prefix-check bypasses.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationClass {
    Public,
    Loopback,
    LinkLocal,
    CloudMetadata,
    Private,
    UniqueLocalV6,
    CarrierGradeNat,
    Reserved,
    Multicast,
    Broadcast,
    Unspecified,
}

impl DestinationClass {
    pub fn is_safe_default(self) -> bool {
        matches!(self, Self::Public)
    }
}

/// Well-known cloud metadata endpoints that must be blocked even though
/// they are not otherwise link-local/private by every implementation's
/// definition (e.g. Alibaba Cloud's `100.100.100.200` is a public-looking
/// address in the CGNAT range, which is already denied by default; listed
/// here for explicitness and because some are hit via a resolvable hostname
/// like `metadata.google.internal`, which lands here after resolution).
const METADATA_V4: [Ipv4Addr; 2] = [
    Ipv4Addr::new(169, 254, 169, 254),
    Ipv4Addr::new(169, 254, 170, 2),
];

fn is_metadata_v6(addr: &Ipv6Addr) -> bool {
    // fd00:ec2::254 (AWS IMDSv2 IPv6 endpoint).
    addr.segments() == [0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254]
}

/// Classify a single resolved or literal address. Recurses through
/// IPv4-mapped (`::ffff:0:0/96`) and NAT64 (`64:ff9b::/96`) IPv6 forms so an
/// embedded IPv4 address is classified by its real semantics, not its IPv6
/// wrapper.
pub fn classify(addr: IpAddr) -> DestinationClass {
    match addr {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => classify_v6(v6),
    }
}

fn classify_v4(addr: Ipv4Addr) -> DestinationClass {
    if METADATA_V4.contains(&addr) {
        return DestinationClass::CloudMetadata;
    }
    if addr == Ipv4Addr::new(100, 100, 100, 200) {
        return DestinationClass::CloudMetadata;
    }
    if addr.is_loopback() {
        return DestinationClass::Loopback;
    }
    if addr.is_unspecified() {
        return DestinationClass::Unspecified;
    }
    if addr.is_link_local() {
        return DestinationClass::LinkLocal;
    }
    if addr.is_private() {
        return DestinationClass::Private;
    }
    if is_carrier_grade_nat(addr) {
        return DestinationClass::CarrierGradeNat;
    }
    if addr.is_broadcast() {
        return DestinationClass::Broadcast;
    }
    if addr.is_multicast() {
        return DestinationClass::Multicast;
    }
    if is_reserved_v4(addr) {
        return DestinationClass::Reserved;
    }
    DestinationClass::Public
}

fn is_carrier_grade_nat(addr: Ipv4Addr) -> bool {
    // 100.64.0.0/10
    let octets = addr.octets();
    octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000
}

fn is_reserved_v4(addr: Ipv4Addr) -> bool {
    let octets = addr.octets();
    // 0.0.0.0/8 (excluding the fully-unspecified address handled above).
    if octets[0] == 0 {
        return true;
    }
    // TEST-NET-1/2/3.
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return true;
    }
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return true;
    }
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return true;
    }
    // 198.18.0.0/15 benchmarking.
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return true;
    }
    // 240.0.0.0/4 reserved for future use.
    if octets[0] >= 240 {
        return true;
    }
    false
}

fn classify_v6(addr: Ipv6Addr) -> DestinationClass {
    if let Some(mapped) = addr.to_ipv4_mapped() {
        return classify_v4(mapped);
    }
    if let Some(embedded) = nat64_embedded_v4(&addr) {
        return classify_v4(embedded);
    }
    if is_metadata_v6(&addr) {
        return DestinationClass::CloudMetadata;
    }
    if addr.is_loopback() {
        return DestinationClass::Loopback;
    }
    if addr.is_unspecified() {
        return DestinationClass::Unspecified;
    }
    if is_unique_local_v6(&addr) {
        return DestinationClass::UniqueLocalV6;
    }
    if is_link_local_v6(&addr) {
        return DestinationClass::LinkLocal;
    }
    if addr.is_multicast() {
        return DestinationClass::Multicast;
    }
    if is_documentation_v6(&addr) {
        return DestinationClass::Reserved;
    }
    DestinationClass::Public
}

/// `64:ff9b::/96` NAT64 well-known prefix: the low 32 bits carry an
/// embedded IPv4 address that must be reclassified by its real semantics.
fn nat64_embedded_v4(addr: &Ipv6Addr) -> Option<Ipv4Addr> {
    let segments = addr.segments();
    if segments[0..6] == [0x0064, 0xff9b, 0, 0, 0, 0] {
        let octets = addr.octets();
        return Some(Ipv4Addr::new(
            octets[12], octets[13], octets[14], octets[15],
        ));
    }
    None
}

fn is_unique_local_v6(addr: &Ipv6Addr) -> bool {
    // fc00::/7
    (addr.segments()[0] & 0xfe00) == 0xfc00
}

fn is_link_local_v6(addr: &Ipv6Addr) -> bool {
    // fe80::/10
    (addr.segments()[0] & 0xffc0) == 0xfe80
}

fn is_documentation_v6(addr: &Ipv6Addr) -> bool {
    // 2001:db8::/32
    addr.segments()[0] == 0x2001 && addr.segments()[1] == 0x0db8
}

#[cfg(test)]
mod tests {
    use super::{classify, DestinationClass};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn v4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn classifies_every_destination_class_from_the_security_review_table() {
        let cases: &[(IpAddr, DestinationClass)] = &[
            (v4(127, 0, 0, 1), DestinationClass::Loopback),
            (IpAddr::V6(Ipv6Addr::LOCALHOST), DestinationClass::Loopback),
            (v4(169, 254, 1, 1), DestinationClass::LinkLocal),
            (v4(169, 254, 169, 254), DestinationClass::CloudMetadata),
            (v4(169, 254, 170, 2), DestinationClass::CloudMetadata),
            (v4(100, 100, 100, 200), DestinationClass::CloudMetadata),
            (v4(10, 0, 0, 1), DestinationClass::Private),
            (v4(172, 16, 0, 1), DestinationClass::Private),
            (v4(192, 168, 1, 1), DestinationClass::Private),
            (v4(100, 64, 0, 1), DestinationClass::CarrierGradeNat),
            (v4(100, 127, 255, 255), DestinationClass::CarrierGradeNat),
            (v4(0, 0, 0, 0), DestinationClass::Unspecified),
            (v4(0, 1, 2, 3), DestinationClass::Reserved),
            (v4(192, 0, 2, 1), DestinationClass::Reserved),
            (v4(198, 51, 100, 1), DestinationClass::Reserved),
            (v4(203, 0, 113, 1), DestinationClass::Reserved),
            (v4(240, 0, 0, 1), DestinationClass::Reserved),
            (v4(255, 255, 255, 255), DestinationClass::Broadcast),
            (v4(224, 0, 0, 1), DestinationClass::Multicast),
            (v4(93, 184, 216, 34), DestinationClass::Public),
            (
                "fd00:ec2::254".parse().expect("aws imds v6"),
                DestinationClass::CloudMetadata,
            ),
            (
                "fc00::1".parse().expect("unique local"),
                DestinationClass::UniqueLocalV6,
            ),
            (
                "fe80::1".parse().expect("link local v6"),
                DestinationClass::LinkLocal,
            ),
            (
                "2001:db8::1".parse().expect("documentation"),
                DestinationClass::Reserved,
            ),
            (
                "ff02::1".parse().expect("multicast v6"),
                DestinationClass::Multicast,
            ),
            (
                "2606:4700:4700::1111".parse().expect("public v6"),
                DestinationClass::Public,
            ),
        ];
        for (addr, expected) in cases {
            assert_eq!(classify(*addr), *expected, "address {addr}");
        }
    }

    #[test]
    fn ipv4_mapped_ipv6_is_reclassified_by_embedded_v4() {
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().expect("mapped loopback");
        assert_eq!(classify(mapped), DestinationClass::Loopback);

        let mapped_metadata: IpAddr = "::ffff:169.254.169.254".parse().expect("mapped metadata");
        assert_eq!(classify(mapped_metadata), DestinationClass::CloudMetadata);
    }

    #[test]
    fn nat64_embedded_ipv4_is_reclassified() {
        let nat64: IpAddr = "64:ff9b::7f00:1".parse().expect("nat64 loopback");
        assert_eq!(classify(nat64), DestinationClass::Loopback);

        let nat64_private: IpAddr = "64:ff9b::a00:1".parse().expect("nat64 private");
        assert_eq!(classify(nat64_private), DestinationClass::Private);
    }
}
