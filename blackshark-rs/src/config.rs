//! Persistent settings management.
//!
//! Settings are stored in a TOML file (default: ~/.config/blackshark/config.toml).
//! Because the device firmware forgets most settings after reboot/reconnect,
//! `apply` re-sends the saved settings to the device. The daemon calls `apply`
//! on startup and on reconnect.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::device::Headset;
use crate::protocol::{self as proto, EqPreset};

/// Varsayilan config yolu: ~/.config/blackshark/config.toml
pub fn default_path() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "blackshark") {
        return dirs.config_dir().join("config.toml");
    }
    PathBuf::from("/etc/blackshark/config.toml")
}

/// Diskte saklanan tum ayarlar. Alanlarin hepsi opsiyonel:
/// yalnizca kullanicinin ayarladiklari uygulanir.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// EQ preset adi: default | game | movie | music
    pub eq_preset: Option<String>,
    /// Custom EQ, 10 bant dB (-6..6). eq_preset ile birlikte verilirse
    /// bu custom EQ uygulanir (preset'ten sonra).
    pub eq: Option<Vec<i8>>,
    /// Sidetone seviyesi 0..15
    pub sidetone: Option<u8>,
    /// Mic noise cancellation
    pub noise_cancel: Option<bool>,
    /// Uyku zamanlayicisi (dakika)
    pub sleep_minutes: Option<u16>,
    /// Guc tasarrufu
    pub power_save: Option<bool>,
}

impl Config {
    /// Verilen yoldan yukler; dosya yoksa bos config doner.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("config okunamadi: {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("config parse edilemedi: {}", path.display()))?;
        Ok(cfg)
    }

    /// Atomik olarak diske yazar (once .tmp'ye, sonra rename).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("dizin olusturulamadi: {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("config serialize edilemedi")?;
        let tmp = tmp_path(path);
        fs::write(&tmp, text.as_bytes())
            .with_context(|| format!("config yazilamadi: {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("config tasinamadi: {}", path.display()))?;
        Ok(())
    }

    /// Config'teki tum ayarlari cihaza uygular.
    pub fn apply(&self, hs: &Headset) -> Result<()> {
        let tid = 0x00;

        if let Some(name) = &self.eq_preset {
            if let Some(p) = EqPreset::from_name(name) {
                hs.send(&proto::eq_preset(p, tid))?;
            }
        }
        if let Some(bands) = &self.eq {
            if bands.len() == 10 {
                let mut arr = [0i8; 10];
                arr.copy_from_slice(bands);
                hs.send(&proto::eq_custom(&arr, tid))?;
            }
        }
        if let Some(level) = self.sidetone {
            hs.send(&proto::sidetone(level, tid))?;
        }
        if let Some(on) = self.noise_cancel {
            hs.send(&proto::noise_cancel(on, tid))?;
        }
        if let Some(min) = self.sleep_minutes {
            hs.send(&proto::sleep_timer(min, tid))?;
        }
        if let Some(on) = self.power_save {
            hs.send(&proto::power_save(on, tid))?;
        }
        Ok(())
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}
