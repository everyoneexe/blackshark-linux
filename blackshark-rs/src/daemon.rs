//! blacksharkd - HID sahipligini elinde tutan D-Bus daemon'i.
//!
//! Mimari (referans: RiskRunner0/blackshark-linux'a benzer):
//!   - Daemon /dev/hidraw* cihazini TEKELDE acar.
//!   - net.blackshark1 D-Bus servisi sunar (session bus).
//!   - Tray/GUI/CLI bu servise baglanir; kimse dogrudan HID'e dokunmaz.
//!   - Bataryayi periyodik yoklar, degisince sinyal yayinlar.
//!   - Ayar degisince config'e yazar; reconnect/boot'ta apply eder.
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

/// Batarya yoklama araligi.
const BATTERY_POLL: Duration = Duration::from_secs(300); // 5 dk

/// Paylasilan daemon durumu.
struct Inner {
    headset: Option<Headset>,
    config: Config,
    config_path: std::path::PathBuf,
    battery: i32, // -1 = bilinmiyor
    connected: bool,
}

#[derive(Clone)]
struct HeadsetService {
    inner: Arc<Mutex<Inner>>,
}

impl HeadsetService {
    /// Cihaz yoksa acmayi dener; sonucu doner.
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

    /// Cihaza komut gonderir; hata olursa cihazi dusurur (reconnect icin).
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
        Err(anyhow::anyhow!("cihaz bagli degil"))
    }

    async fn save_config(&self) {
        let g = self.inner.lock().await;
        let _ = g.config.save(&g.config_path);
    }
}

#[interface(name = "net.blackshark1.Headset")]
impl HeadsetService {
    /// Batarya yuzdesi (property). -1 = bilinmiyor.
    #[zbus(property)]
    async fn battery(&self) -> i32 {
        self.inner.lock().await.battery
    }

    /// Kulaklik bagli mi (property).
    #[zbus(property)]
    async fn connected(&self) -> bool {
        self.inner.lock().await.connected
    }

    /// EQ preset: "default" | "game" | "movie" | "music"
    async fn set_eq_preset(&self, name: String) -> zbus::fdo::Result<()> {
        let p = EqPreset::from_name(&name)
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("gecersiz preset: {name}")))?;
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

    /// Custom EQ: 10 bant dB (-6..6). sira 31..16k.
    async fn set_eq(&self, bands: Vec<i32>) -> zbus::fdo::Result<()> {
        if bands.len() != 10 {
            return Err(zbus::fdo::Error::InvalidArgs("10 bant gerekli".into()));
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

    /// Sleep timer (dakika).
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

    /// Config'teki tum ayarlari cihaza yeniden uygula.
    async fn apply(&self) -> zbus::fdo::Result<()> {
        if !self.ensure_device().await {
            return Err(zbus::fdo::Error::Failed("cihaz bagli degil".into()));
        }
        let g = self.inner.lock().await;
        if let Some(hs) = g.headset.as_ref() {
            g.config
                .apply(hs)
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        }
        Ok(())
    }

    /// Bataryayi simdi oku (senkron).
    async fn refresh_battery(&self) -> zbus::fdo::Result<i32> {
        if !self.ensure_device().await {
            return Ok(-1);
        }
        let mut g = self.inner.lock().await;
        if let Some(hs) = g.headset.as_ref() {
            if let Ok(resp) = hs.query(&proto::battery_query(0)) {
                if let Some(level) = proto::parse_battery(&resp) {
                    g.battery = level as i32;
                    return Ok(level as i32);
                }
            }
        }
        Ok(g.battery)
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

    // D-Bus session bus'a servisi yayinla.
    let conn = connection::Builder::session()?
        .name("net.blackshark1")?
        .serve_at("/net/blackshark1/Headset", service.clone())?
        .build()
        .await?;

    // Baslangicta cihazi ac ve config'i uygula.
    if service.ensure_device().await {
        let g = inner.lock().await;
        if let Some(hs) = g.headset.as_ref() {
            let _ = g.config.apply(hs);
        }
    }

    // Batarya polling gorevi.
    let poll_service = service.clone();
    let iface_ref = conn
        .object_server()
        .interface::<_, HeadsetService>("/net/blackshark1/Headset")
        .await?;
    tokio::spawn(async move {
        loop {
            poll_service.ensure_device().await;
            {
                let mut g = poll_service.inner.lock().await;
                if let Some(hs) = g.headset.as_ref() {
                    if let Ok(resp) = hs.query(&proto::battery_query(0)) {
                        if let Some(level) = proto::parse_battery(&resp) {
                            g.battery = level as i32;
                        }
                    }
                }
            }
            // Property changed sinyalleri (zbus 4.x)
            let ctxt = iface_ref.signal_context();
            let iface = iface_ref.get().await;
            let _ = iface.battery_changed(ctxt).await;
            let _ = iface.connected_changed(ctxt).await;
            tokio::time::sleep(BATTERY_POLL).await;
        }
    });

    // Ilk batarya okumasini hemen tetikle.
    let _ = service.refresh_battery().await;

    // Ctrl-C / SIGTERM'e kadar calis.
    tokio::signal::ctrl_c().await?;
    drop(conn);
    Ok(())
}
