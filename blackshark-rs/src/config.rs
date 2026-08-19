//! Persistent settings management.
//!
//! Settings are stored in a TOML file (default: ~/.config/blackshark/config.toml).
//! Because the device firmware forgets most settings after reboot/reconnect,
//! `apply` re-sends the saved settings to the device. The daemon calls `apply`
//! on startup and on reconnect.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::device::Headset;
use crate::protocol::{self as proto, EqPreset};

/// Default configuration path: ~/.config/blackshark/config.toml
pub fn default_path() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "blackshark") {
        return dirs.config_dir().join("config.toml");
    }
    PathBuf::from("/etc/blackshark/config.toml")
}

/// All settings persisted to disk. Every field is optional; only user-configured
/// values are applied.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// EQ preset name: default | game | movie | music.
    pub eq_preset: Option<String>,
    /// Custom EQ, 10 dB bands (-6..6). When set with eq_preset, this custom EQ
    /// is applied after the preset.
    pub eq: Option<Vec<i8>>,
    /// Sidetone level, 0..15.
    pub sidetone: Option<u8>,
    /// Mic noise cancellation
    pub noise_cancel: Option<bool>,
    /// Sleep timer in minutes.
    pub sleep_minutes: Option<u16>,
    /// Power saving.
    pub power_save: Option<bool>,
    /// Named custom EQ profiles. Each value contains the 10 dB bands.
    pub eq_profiles: BTreeMap<String, Vec<i8>>,
}

impl Config {
    /// Loads from the specified path; returns an empty configuration if absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new_install_defaults());
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("could not read configuration: {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("could not parse configuration: {}", path.display()))?;
        Ok(cfg)
    }

    /// Defaults used before a user configuration file exists.
    ///
    /// Keeping this separate from `Default` preserves the meaning of missing
    /// fields in existing configuration files.
    fn new_install_defaults() -> Self {
        Self {
            eq_preset: Some("default".into()),
            ..Self::default()
        }
    }

    /// Writes atomically to disk (first to .tmp, then renames it).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("could not create directory: {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("could not serialize configuration")?;
        let tmp = tmp_path(path);
        fs::write(&tmp, text.as_bytes())
            .with_context(|| format!("could not write configuration: {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| {
            format!(
                "could not move configuration into place: {}",
                path.display()
            )
        })?;
        Ok(())
    }

    /// Applies every saved setting to the device.
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
