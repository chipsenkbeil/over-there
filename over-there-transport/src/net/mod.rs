pub mod tcp;
pub mod udp;

/// The Internet Assigned Numbers Authority (IANA) suggested range
/// for dynamic and private ports
///
/// - FreeBSD uses this range since release 4.6
/// - Windows Vista, 7, and Server 2008 use this range
pub const IANA_EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = (49152..=65535);

/// Common Linux kernel port range
pub const LINUX_EPHEMERAL_PORT_RANGE: std::ops::RangeInclusive<u16> = (32768..=61000);
