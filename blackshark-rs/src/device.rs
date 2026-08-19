//! Cihaz erisimi: hidapi uzerinden feature report gonderme/okuma.

use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};

use crate::protocol::{PID, REPORT_LEN, VID};

/// Kulakligin komut kanalini (MI_00 vendor collection) acar.
///
/// Cihaz birden fazla HID arayuzu sunar; feature report kabul eden
/// dogru arayuzu bulmak icin adaylari sirayla deneriz.
pub struct Headset {
    dev: HidDevice,
}

impl Headset {
    pub fn open() -> Result<Self> {
        let api = HidApi::new().context("hidapi baslatilamadi")?;

        // 1532:0A5B'e ait tum arayuzleri topla.
        let mut infos: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == VID && d.product_id() == PID)
            .collect();

        if infos.is_empty() {
            return Err(anyhow!(
                "Cihaz bulunamadi ({:04x}:{:04x}). Kulaklik takili mi?",
                VID,
                PID
            ));
        }

        // MI_00 (interface 0) genelde komut kanalidir; onu one al.
        infos.sort_by_key(|d| d.interface_number());

        let mut last_err = None;
        for info in &infos {
            match info.open_device(&api) {
                Ok(dev) => {
                    // Bir feature report gonderilebiliyor mu diye acmis olduk;
                    // gercek dogrulama ilk komutta yapilir. Ilk aciilani kullan.
                    return Ok(Self { dev });
                }
                Err(e) => last_err = Some(e),
            }
        }

        Err(anyhow!(
            "Cihaz acilamadi. Yetki sorunu olabilir (udev kurali / sudo). Son hata: {:?}",
            last_err
        ))
    }

    /// Feature report gonderir (SET_REPORT).
    pub fn send(&self, report: &[u8; REPORT_LEN]) -> Result<()> {
        // hidapi send_feature_report'ta ilk byte report ID'dir (0x07 zaten [0]'da).
        self.dev
            .send_feature_report(report)
            .context("feature report gonderilemedi (yetki? yanlis arayuz?)")?;
        Ok(())
    }

    /// Komut gonderip cevabini okur (SET ardindan GET_REPORT).
    pub fn query(&self, report: &[u8; REPORT_LEN]) -> Result<Vec<u8>> {
        self.send(report)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut buf = vec![0u8; REPORT_LEN];
        buf[0] = 0x07; // report ID
        let n = self
            .dev
            .get_feature_report(&mut buf)
            .context("feature report okunamadi")?;
        buf.truncate(n.max(REPORT_LEN));
        Ok(buf)
    }
}
