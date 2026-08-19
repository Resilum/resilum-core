//! The LXMF `custom_type` tags this bridge dispatches on, in one place: a
//! receiver routes on the exact string, so both directions have to read the
//! same constant.

pub(super) const SCHEMA_EVENT: &str = "rsl.nostr/1";
pub(super) const SCHEMA_SUBSCRIBE: &str = "rsl.relay/1";
pub(super) const SCHEMA_ACK: &str = "rsl.nostr.ack/1";
pub(super) const SCHEMA_SUBSCRIBE_ACK: &str = "rsl.relay.ack/1";
