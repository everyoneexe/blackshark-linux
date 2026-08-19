//! blacksharkd - D-Bus daemon that exclusively owns the HID device.
//!
//! Architecture:
//!   - The daemon exclusively opens /dev/hidraw*.
//!   - It exposes the net.blackshark1 D-Bus service on the session bus.
//!   - The tray, GUI, and CLI connect to this service; none touch HID directly.
//!   - It polls the battery periodically and emits a signal when it changes.
//!   - It saves changed settings and reapplies them on reconnect and boot.
//!
//! D-Bus:
//!   service : net.blackshark1
//!   path    : /net/blackshark1/Headset
//!   iface   : net.blackshark1.Headset

mod config;
mod device;
mod protocol;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Mutex;
use zbus::{connection, interface};

use config::Config;
use device::Headset;
use protocol::{self as proto, EqPreset};

/// Battery polling interval.
const BATTERY_POLL: Duration = Duration::from_secs(3);

/// Shared daemon state.
struct Inner {
    headset: Option<Headset>,
    config: Config,
    config_path: std::path::PathBuf,
    battery: i32, // -1 = unknown
    connected: bool,
}

#[derive(Clone)]
struct HeadsetService {
    inner: Arc<Mutex<Inner>>,
}

impl HeadsetService {
    /// Tries to open the device if it is not already available.
    async fn ensure_device(&self) -> bool {
        let mut g = self.inner.lock().await;
        if g.headset.is_some() {
            return true;
        }
        match Headset::open() {
            Ok(hs) => {
                g.headset = Some(hs);
                g.connected = true;
                true
            }
            Err(_) => {
                g.connected = false;
                false
            }
        }
    }

    /// Sends a command to the device; drops it on failure so it can reconnect.
    async fn send(&self, report: &[u8; proto::REPORT_LEN]) -> Result<()> {
        let mut g = self.inner.lock().await;
        if let Some(hs) = g.headset.as_ref() {
            match hs.send(report) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    g.headset = None;
                    g.connected = false;
                    return Err(e);
                }
            }
        }
        Err(anyhow::anyhow!("device is not connected"))
    }

    async fn save_config(&self) {
        let g = self.inner.lock().await;
        let _ = g.config.save(&g.config_path);
    }
}

#[interface(name = "net.blackshark1.Headset")]
impl HeadsetService {
    /// Battery percentage property. -1 = unknown.
    #[zbus(property)]
    async fn battery(&self) -> i32 {
        self.inner.lock().await.battery
    }

    /// Whether the headset is connected.
    #[zbus(property)]
    async fn connected(&self) -> bool {
        self.inner.lock().await.connected
    }

    /// Active EQ profile name. `custom` means a manually edited band curve.
    #[zbus(property)]
    async fn eq_preset(&self) -> String {
        let g = self.inner.lock().await;
        g.config.eq_preset.clone().unwrap_or_else(|| {
            if g.config.eq.is_some() {
                "custom".into()
            } else {
                "default".into()
            }
        })
    }

    /// Saved custom EQ bands as a comma-separated string. An empty string means
    /// a device preset is active. A string avoids client-specific D-Bus array
    /// marshalling issues while keeping the property easy to consume.
    #[zbus(property)]
    async fn eq_bands(&self) -> String {
        self.inner
            .lock()
            .await
            .config
            .eq
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|band| band.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Saved sidetone level, 0..15.
    #[zbus(property)]
    async fn sidetone(&self) -> u32 {
        self.inner.lock().await.config.sidetone.unwrap_or(0) as u32
    }

    /// Saved microphone noise-cancellation state.
    #[zbus(property)]
    async fn noise_cancel(&self) -> bool {
        self.inner.lock().await.config.noise_cancel.unwrap_or(false)
    }

    /// Saved sleep timer in minutes.
    #[zbus(property)]
    async fn sleep(&self) -> u32 {
        self.inner.lock().await.config.sleep_minutes.unwrap_or(0) as u32
    }

    /// Saved power-saving state.
    #[zbus(property)]
    async fn power_save(&self) -> bool {
        self.inner.lock().await.config.power_save.unwrap_or(false)
    }

    /// List saved custom EQ profile names.
    async fn list_eq_profiles(&self) -> Vec<String> {
        self.inner
            .lock()
            .await
            .config
            .eq_profiles
            .keys()
            .cloned()
            .collect()
    }

    /// Return a profile as comma-separated dB values for D-Bus clients.
    async fn get_eq_profile(&self, name: String) -> zbus::fdo::Result<String> {
        let g = self.inner.lock().await;
        let bands =
            g.config.eq_profiles.get(&name).ok_or_else(|| {
                zbus::fdo::Error::InvalidArgs(format!("unknown EQ profile: {name}"))
            })?;
        Ok(bands
            .iter()
            .map(|band| band.to_string())
            .collect::<Vec<_>>()
            .join(","))
    }

    /// Save a named custom EQ profile without changing the active EQ.
    async fn save_eq_profile(&self, name: String, bands: Vec<i32>) -> zbus::fdo::Result<()> {
        let name = name.trim();
        if name.is_empty() || name.len() > 48 {
            return Err(zbus::fdo::Error::InvalidArgs(
                "profile name must contain 1 to 48 characters".into(),
            ));
        }
        if bands.len() != 10 {
            return Err(zbus::fdo::Error::InvalidArgs(
                "10 bands are required".into(),
            ));
        }
        let values = bands
            .iter()
            .map(|band| (*band).clamp(-6, 6) as i8)
            .collect();
        {
            let mut g = self.inner.lock().await;
            g.config.eq_profiles.insert(name.to_string(), values);
        }
        self.save_config().await;
        Ok(())
    }

    /// Delete a named custom EQ profile.
    async fn delete_eq_profile(&self, name: String) -> zbus::fdo::Result<()> {
        let mut g = self.inner.lock().await;
        if g.config.eq_profiles.remove(&name).is_none() {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "unknown EQ profile: {name}"
            )));
        }
        drop(g);
        self.save_config().await;
        Ok(())
    }

    /// Apply a saved custom EQ profile to the device.
    async fn apply_eq_profile(&self, name: String) -> zbus::fdo::Result<()> {
        let bands = {
            let g = self.inner.lock().await;
            g.config.eq_profiles.get(&name).cloned().ok_or_else(|| {
                zbus::fdo::Error::InvalidArgs(format!("unknown EQ profile: {name}"))
            })?
        };
        let mut arr = [0i8; 10];
        arr.copy_from_slice(&bands);
        self.send(&proto::eq_custom(&arr, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.eq_preset = None;
            g.config.eq = Some(bands);
        }
        self.save_config().await;
        Ok(())
    }

    /// EQ preset: "default" | "game" | "movie" | "music"
    async fn set_eq_preset(&self, name: String) -> zbus::fdo::Result<()> {
        let p = EqPreset::from_name(&name)
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("invalid preset: {name}")))?;
        self.send(&proto::eq_preset(p, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.eq_preset = Some(name.to_lowercase());
            g.config.eq = None;
        }
        self.save_config().await;
        Ok(())
    }

    /// Custom EQ: 10 dB bands (-6..6), ordered from 31 Hz to 16 kHz.
    async fn set_eq(&self, bands: Vec<i32>) -> zbus::fdo::Result<()> {
        if bands.len() != 10 {
            return Err(zbus::fdo::Error::InvalidArgs(
                "10 bands are required".into(),
            ));
        }
        let mut arr = [0i8; 10];
        for (i, b) in bands.iter().enumerate() {
            arr[i] = (*b).clamp(-6, 6) as i8;
        }
        self.send(&proto::eq_custom(&arr, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.eq_preset = None;
            g.config.eq = Some(arr.to_vec());
        }
        self.save_config().await;
        Ok(())
    }

    /// Sidetone 0..15.
    async fn set_sidetone(&self, level: u32) -> zbus::fdo::Result<()> {
        let level = level.min(15) as u8;
        self.send(&proto::sidetone(level, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.sidetone = Some(level);
        }
        self.save_config().await;
        Ok(())
    }

    /// Mic noise cancellation.
    async fn set_noise_cancel(&self, on: bool) -> zbus::fdo::Result<()> {
        self.send(&proto::noise_cancel(on, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.noise_cancel = Some(on);
        }
        self.save_config().await;
        Ok(())
    }

    /// Sleep timer in minutes.
    async fn set_sleep(&self, minutes: u32) -> zbus::fdo::Result<()> {
        let m = minutes as u16;
        self.send(&proto::sleep_timer(m, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.sleep_minutes = Some(m);
        }
        self.save_config().await;
        Ok(())
    }

    /// Power saving.
    async fn set_power_save(&self, on: bool) -> zbus::fdo::Result<()> {
        self.send(&proto::power_save(on, 0))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        {
            let mut g = self.inner.lock().await;
            g.config.power_save = Some(on);
        }
        self.save_config().await;
        Ok(())
    }

    /// Reapply every saved configuration setting to the device.
    async fn apply(&self) -> zbus::fdo::Result<()> {
        if !self.ensure_device().await {
            return Err(zbus::fdo::Error::Failed("device is not connected".into()));
        }
        let g = self.inner.lock().await;
        if let Some(hs) = g.headset.as_ref() {
            g.config
                .apply(hs)
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        }
        Ok(())
    }

    /// Read the battery now, synchronously.
    async fn refresh_battery(&self) -> zbus::fdo::Result<i32> {
        if !self.ensure_device().await {
            return Ok(-1);
        }
        let mut g = self.inner.lock().await;
        let response = match g.headset.as_ref() {
            Some(hs) => hs.query(&proto::battery_query(0)),
            None => return Ok(-1),
        };
        match response.ok().and_then(|resp| proto::parse_battery(&resp)) {
            Some(level) => {
                g.battery = level as i32;
                g.connected = true;
                Ok(level as i32)
            }
            None => {
                // The USB dongle can remain present after the headset itself is
                // switched off. An invalid or absent battery reply means the
                // radio headset is unavailable, not that it has 255% charge.
                g.headset = None;
                g.connected = false;
                g.battery = -1;
                Ok(-1)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = config::default_path();
    let cfg = Config::load(&config_path).unwrap_or_default();

    let inner = Arc::new(Mutex::new(Inner {
        headset: None,
        config: cfg,
        config_path,
        battery: -1,
        connected: false,
    }));

    let service = HeadsetService {
        inner: inner.clone(),
    };

    // Publish the service on the D-Bus session bus.
    let conn = connection::Builder::session()?
        .name("net.blackshark1")?
        .serve_at("/net/blackshark1/Headset", service.clone())?
        .build()
        .await?;

    // Open the device and apply the configuration at startup.
    if service.ensure_device().await {
        let g = inner.lock().await;
        if let Some(hs) = g.headset.as_ref() {
            let _ = g.config.apply(hs);
        }
    }

    // Battery polling task.
    let poll_service = service.clone();
    let iface_ref = conn
        .object_server()
        .interface::<_, HeadsetService>("/net/blackshark1/Headset")
        .await?;
    tokio::spawn(async move {
        loop {
            let _ = poll_service.refresh_battery().await;
            // Property-changed signals (zbus 4.x).
            let ctxt = iface_ref.signal_context();
            let iface = iface_ref.get().await;
            let _ = iface.battery_changed(ctxt).await;
            let _ = iface.connected_changed(ctxt).await;
            tokio::time::sleep(BATTERY_POLL).await;
        }
    });

    // Trigger the first battery read immediately.
    let _ = service.refresh_battery().await;

    // Run until Ctrl-C or SIGTERM.
    tokio::signal::ctrl_c().await?;
    drop(conn);
    Ok(())
}
