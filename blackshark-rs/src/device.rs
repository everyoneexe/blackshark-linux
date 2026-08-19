//! Device access: sending and receiving feature reports through hidapi.

use anyhow::{anyhow, Context, Result};
use hidapi::{HidApi, HidDevice};

use crate::protocol::{PID, REPORT_LEN, VID};

/// Opens the headset command channel (MI_00 vendor collection).
///
/// The device exposes several HID interfaces; candidate interfaces are tried in
/// sequence to find the one that accepts feature reports.
pub struct Headset {
    dev: HidDevice,
}

impl Headset {
    pub fn open() -> Result<Self> {
        let api = HidApi::new().context("could not initialize hidapi")?;

        // Collect every interface that belongs to 1532:0A5B.
        let mut infos: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == VID && d.product_id() == PID)
            .collect();

        if infos.is_empty() {
            return Err(anyhow!(
                "device not found ({:04x}:{:04x}). Is the headset connected?",
                VID,
                PID
            ));
        }

        // MI_00 (interface 0) is generally the command channel, so try it first.
        infos.sort_by_key(|d| d.interface_number());

        let mut last_err = None;
        for info in &infos {
            match info.open_device(&api) {
                Ok(dev) => {
                    // Opening the interface confirms it can be accessed; the first
                    // command provides the actual verification. Use the first one.
                    return Ok(Self { dev });
                }
                Err(e) => last_err = Some(e),
            }
        }

        Err(anyhow!(
            "could not open device. This may be a permissions issue (udev rule / sudo). Last error: {:?}",
            last_err
        ))
    }

    /// Sends a feature report (SET_REPORT).
    pub fn send(&self, report: &[u8; REPORT_LEN]) -> Result<()> {
        // The first byte of hidapi's send_feature_report is the report ID (0x07 at [0]).
        self.dev
            .send_feature_report(report)
            .context("could not send feature report (permissions or wrong interface?)")?;
        Ok(())
    }

    /// Sends a command and reads its response (SET followed by GET_REPORT).
    pub fn query(&self, report: &[u8; REPORT_LEN]) -> Result<Vec<u8>> {
        self.send(report)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut buf = vec![0u8; REPORT_LEN];
        buf[0] = 0x07; // report ID
        let n = self
            .dev
            .get_feature_report(&mut buf)
            .context("could not read feature report")?;
        buf.truncate(n.max(REPORT_LEN));
        Ok(buf)
    }
}
