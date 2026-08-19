//! Razer BlackShark V3 X HyperSpeed (Xbox) HID protokolu.
//!
//! Synapse'in USB HID trafigi reverse-engineer edilerek cikarilmistir.
//! Tum komutlar Synapse'in gonderdikleriyle byte-byte dogrulanmistir.
//!
//! Report yapisi (64 byte):
//! ```text
//!  [0]   report_id      = 0x07
//!  [1]   status         = 0x00
//!  [2]   transaction_id (artan sayac; cihaz umursamiyor, 0 da kabul)
//!  [3]   0x00
//!  [4]   0x00
//!  [5]   0x00
//!  [6]   data_size
//!  [7]   command_class
//!  [8]   command_id
//!  [9..] arguments
//!  [62]  crc = XOR(byte[3..=60])
//!  [63]  reserved = 0x00
//! ```

pub const VID: u16 = 0x1532;
pub const PID: u16 = 0x0A5B;
pub const REPORT_LEN: usize = 64;

/// Razer CRC: byte[3]..=byte[60] XOR.
fn crc(report: &[u8; REPORT_LEN]) -> u8 {
    let mut c = 0u8;
    for &b in &report[3..=60] {
        c ^= b;
    }
    c
}

/// Bir Razer HID report'u olusturur ve CRC'sini yerlestirir.
pub fn build_report(
    data_size: u8,
    cmd_class: u8,
    cmd_id: u8,
    args: &[u8],
    transaction_id: u8,
) -> [u8; REPORT_LEN] {
    let mut r = [0u8; REPORT_LEN];
    r[0] = 0x07;
    r[1] = 0x00;
    r[2] = transaction_id;
    r[6] = data_size;
    r[7] = cmd_class;
    r[8] = cmd_id;
    for (i, &b) in args.iter().enumerate() {
        r[9 + i] = b;
    }
    r[62] = crc(&r);
    r[63] = 0x00;
    r
}

/// EQ bant dB degerini (-6..+6) cihaz byte'ina cevirir (signed-magnitude).
/// 0dB=0x00, +N=N, -N=0x80|N
pub fn signed_db_to_byte(db: i8) -> u8 {
    let db = db.clamp(-6, 6);
    if db >= 0 {
        db as u8
    } else {
        0x80 | (db.unsigned_abs())
    }
}

/// EQ preset degerleri (command_class=0x18, command_id=0x00).
#[derive(Clone, Copy, Debug)]
pub enum EqPreset {
    Default = 0x00,
    Game = 0x01,
    Movie = 0x02,
    Music = 0x03,
}

impl EqPreset {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default" => Some(Self::Default),
            "game" => Some(Self::Game),
            "movie" => Some(Self::Movie),
            "music" => Some(Self::Music),
            _ => None,
        }
    }
}

// ---- Komut olusturucular ----

pub fn eq_preset(p: EqPreset, tid: u8) -> [u8; REPORT_LEN] {
    // args: 00 01 00 <preset>
    build_report(0x04, 0x18, 0x00, &[0x00, 0x01, 0x00, p as u8], tid)
}

/// 10 bant, sira: 31,63,125,250,500,1k,2k,4k,8k,16k. dB araligi -6..+6.
pub fn eq_custom(bands: &[i8; 10], tid: u8) -> [u8; REPORT_LEN] {
    let mut args = vec![0x00u8, 0x01u8]; // sabit prefix (byte[9..10])
    for &b in bands {
        args.push(signed_db_to_byte(b));
    }
    build_report(0x0C, 0x18, 0x01, &args, tid)
}

/// Sidetone / mic monitoring, 0..15.
pub fn sidetone(level: u8, tid: u8) -> [u8; REPORT_LEN] {
    let level = level.min(15);
    // args: 00 03 <level>
    build_report(0x03, 0x18, 0x03, &[0x00, 0x03, level], tid)
}

/// Mic noise cancellation on/off.
pub fn noise_cancel(on: bool, tid: u8) -> [u8; REPORT_LEN] {
    // args: 00 <v>
    build_report(0x02, 0x18, 0x0F, &[0x00, on as u8], tid)
}

/// Sleep timer, dakika (cihaza big-endian saniye olarak gider).
pub fn sleep_timer(minutes: u16, tid: u8) -> [u8; REPORT_LEN] {
    let secs = minutes.saturating_mul(60);
    let hi = (secs >> 8) as u8;
    let lo = (secs & 0xFF) as u8;
    build_report(0x02, 0x07, 0x03, &[hi, lo], tid)
}

/// Power saving on/off.
pub fn power_save(on: bool, tid: u8) -> [u8; REPORT_LEN] {
    // args: 00 <v>
    build_report(0x02, 0x07, 0x08, &[0x00, on as u8], tid)
}

/// Batarya seviyesi sorgusu (GET). Cevapta byte[9] = yuzde.
pub fn battery_query(tid: u8) -> [u8; REPORT_LEN] {
    build_report(0x02, 0x07, 0x80, &[0x00], tid)
}

/// Batarya cevabindan yuzdeyi cikarir.
pub fn parse_battery(resp: &[u8]) -> Option<u8> {
    // Cevap yapisi: .. 02 07 80 00 VV -> report[9] = VV
    resp.get(9).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(r: &[u8]) -> String {
        r.iter().map(|b| format!("{:02x}", b)).collect()
    }

    // Asagidaki beklenen degerler Synapse'ten yakalanan gercek paketlerdir.

    #[test]
    fn test_eq_game() {
        let r = eq_preset(EqPreset::Game, 0x00);
        assert_eq!(
            hex(&r),
            "07000000000004180000010001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001c00"
        );
    }

    #[test]
    fn test_sidetone_15() {
        let r = sidetone(15, 0x0a);
        assert_eq!(
            hex(&r),
            "07000a00000003180300030f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001400"
        );
    }

    #[test]
    fn test_noise_on() {
        let r = noise_cancel(true, 0x0d);
        assert_eq!(
            hex(&r),
            "07000d00000002180f00010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001400"
        );
    }

    #[test]
    fn test_sleep_60() {
        let r = sleep_timer(60, 0x13);
        assert_eq!(
            hex(&r),
            "0700130000000207030e100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001800"
        );
    }

    #[test]
    fn test_power_save_on() {
        let r = power_save(true, 0x0e);
        assert_eq!(
            hex(&r),
            "07000e00000002070800010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c00"
        );
    }

    #[test]
    fn test_battery_query() {
        let r = battery_query(0x03);
        assert_eq!(
            hex(&r),
            "07000300000002078000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008500"
        );
    }

    #[test]
    fn test_eq_custom_ladder() {
        // 31Hz=+1..1k=+6, 2k=-1, 4k=0, 8k=0, 16k=-6
        let bands = [1, 2, 3, 4, 5, 6, -1, 0, 0, -6];
        let r = eq_custom(&bands, 0x13);
        assert_eq!(
            hex(&r),
            "0700130000000c180100010102030405068100008600000000000000000000000000000000000000000000000000000000000000000000000000000000001400"
        );
    }

    #[test]
    fn test_signed_db() {
        assert_eq!(signed_db_to_byte(0), 0x00);
        assert_eq!(signed_db_to_byte(6), 0x06);
        assert_eq!(signed_db_to_byte(-1), 0x81);
        assert_eq!(signed_db_to_byte(-6), 0x86);
    }
}
