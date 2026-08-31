use blew::gatt::{AttributePermissions, CharacteristicProperties, GattCharacteristic, GattService};

use crate::ble::spec;

pub(super) fn service(identity: [u8; spec::IDENTITY_LEN]) -> GattService {
    GattService {
        uuid: uuid::Uuid::from_u128(spec::SERVICE),
        primary: true,
        characteristics: vec![
            characteristic(
                spec::TX_NOTIFIED_BY_THE_PERIPHERAL,
                CharacteristicProperties::NOTIFY,
                AttributePermissions::READ,
                Vec::new(),
            ),
            characteristic(
                spec::RX_WRITTEN_BY_THE_CENTRAL,
                CharacteristicProperties::WRITE | CharacteristicProperties::WRITE_WITHOUT_RESPONSE,
                AttributePermissions::WRITE,
                Vec::new(),
            ),
            characteristic(
                spec::IDENTITY_READ_FROM_THE_PERIPHERAL,
                CharacteristicProperties::READ,
                AttributePermissions::READ,
                identity.to_vec(),
            ),
        ],
    }
}

fn characteristic(
    uuid: u128,
    properties: CharacteristicProperties,
    permissions: AttributePermissions,
    value: Vec<u8>,
) -> GattCharacteristic {
    GattCharacteristic {
        uuid: uuid::Uuid::from_u128(uuid),
        properties,
        permissions,
        value,
        descriptors: Vec::new(),
    }
}
