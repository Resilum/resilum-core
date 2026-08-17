//! Parse the FFI re-queue request into what the router is asked for.

use data_encoding::HEXLOWER;
use leviculum_lxmf::DeliveryMethod;

use super::request::RequestError;
use super::send::parse_method;

/// The message and delivery method a re-queue names, from the two strings the
/// FFI takes. `message_id` is lowercase hex, as every id this library hands
/// out is.
pub fn parse_request(
    message_id: &str,
    method: &str,
) -> Result<([u8; 32], DeliveryMethod), RequestError> {
    let refuse = || RequestError::Hex {
        field: "message_id",
    };
    let bytes = HEXLOWER
        .decode(message_id.as_bytes())
        .map_err(|_| refuse())?;
    let id: [u8; 32] = bytes.try_into().map_err(|_| refuse())?;
    Ok((id, parse_method(method)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_request_parses_to_the_id_and_method_it_names() {
        let (id, method) = parse_request(&"ab".repeat(32), "propagated").expect("parses");
        assert_eq!(id, [0xab; 32]);
        assert_eq!(method, DeliveryMethod::Propagated);
    }

    /// A 16-byte destination hash is the neighbouring hex string a caller is
    /// most likely to pass by mistake, and decodes fine as hex.
    #[test]
    fn an_id_of_the_wrong_length_is_refused() {
        assert!(parse_request(&"ab".repeat(16), "direct").is_err());
    }

    #[test]
    fn the_delivery_method_is_no_part_of_the_message_id() {
        let identity = crate::identity::generate();
        let source_hash = crate::identity::lxmf_address(&identity);
        let request = |method: &str| {
            format!(
                r#"{{"destination":"{}","method":"{method}","content_b64":"aGk=","timestamp":1.5}}"#,
                "ab".repeat(16),
            )
        };
        let build = |method: &str| {
            crate::lxmf::send::build_message(&request(method), &identity, source_hash, 1.5)
                .expect("message")
        };

        let direct = build("direct");
        let propagated = build("propagated");

        assert_ne!(direct.method, propagated.method);
        assert_eq!(direct.message_id, propagated.message_id);
    }
}
