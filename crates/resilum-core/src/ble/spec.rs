pub const PROTOCOL: &str = "BLE_PROTOCOL v2.2";

pub const SERVICE: u128 = 0x37145b00_442d_4a94_917f_8f42c5da28e3;
pub const TX_NOTIFIED_BY_THE_PERIPHERAL: u128 = 0x37145b00_442d_4a94_917f_8f42c5da28e4;
pub const RX_WRITTEN_BY_THE_CENTRAL: u128 = 0x37145b00_442d_4a94_917f_8f42c5da28e5;
pub const IDENTITY_READ_FROM_THE_PERIPHERAL: u128 = 0x37145b00_442d_4a94_917f_8f42c5da28e6;

pub const IDENTITY_LEN: usize = 16;

pub const FRAGMENT_HEADER_LEN: usize = 5;

pub const TYPE_LONE_ACCEPTED_NEVER_SENT: u8 = 0x00;
pub const TYPE_START: u8 = 0x01;
pub const TYPE_CONTINUE: u8 = 0x02;
pub const TYPE_END: u8 = 0x03;

pub const LARGEST_PACKET: usize = 1064;

pub const PEERS_AT_ONCE: usize = 7;

pub const KEEPALIVE_PACKET: [u8; 1] = [0x00];
pub const KEEPALIVE_EVERY_MS: u64 = 15_000;
pub const ABANDON_REASSEMBLY_AFTER_MS: u64 = 30_000;
