//! Minimal BlueZ discovery used when the USB receiver is not present.
//!
//! The headset exposes its battery through `org.bluez.Battery1` while using
//! classic Bluetooth audio. Control GATT discovery is kept separate so the
//! daemon never sends USB HID reports over a Bluetooth connection.

use zbus::{fdo::ObjectManagerProxy, Connection};

#[derive(Debug, Clone, Copy)]
pub struct Status {
    pub battery: Option<u8>,
}

/// Returns the status for a connected BlackShark device known to BlueZ.
pub async fn connected_status() -> Option<Status> {
    let connection = match Connection::system().await {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("could not connect to BlueZ system bus: {error}");
            return None;
        }
    };
    let builder = match ObjectManagerProxy::builder(&connection).destination("org.bluez") {
        Ok(builder) => builder,
        Err(error) => {
            eprintln!("could not target BlueZ object manager: {error}");
            return None;
        }
    };
    let builder = match builder.path("/") {
        Ok(builder) => builder,
        Err(error) => {
            eprintln!("could not address BlueZ object manager: {error}");
            return None;
        }
    };
    let manager = match builder.build().await {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("could not create BlueZ object manager proxy: {error}");
            return None;
        }
    };
    let objects = match manager.get_managed_objects().await {
        Ok(objects) => objects,
        Err(error) => {
            eprintln!("could not enumerate BlueZ devices: {error}");
            return None;
        }
    };

    for interfaces in objects.values() {
        let device = match interfaces.get("org.bluez.Device1") {
            Some(device) => device,
            None => continue,
        };
        let name = device
            .get("Name")
            .and_then(|value| value.try_clone().ok())
            .and_then(|value| String::try_from(value).ok())
            .unwrap_or_default();
        let connected = device
            .get("Connected")
            .and_then(|value| bool::try_from(value).ok())
            .unwrap_or(false);

        if !connected || !name.to_ascii_lowercase().contains("blackshark") {
            continue;
        }

        let battery = interfaces
            .get("org.bluez.Battery1")
            .and_then(|properties| properties.get("Percentage"))
            .and_then(|value| u8::try_from(value).ok())
            .filter(|level| *level <= 100);
        return Some(Status { battery });
    }

    None
}
