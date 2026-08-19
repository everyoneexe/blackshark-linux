//! blackshark - CLI istemci. Daemon'a (net.blackshark1) D-Bus ile baglanir.
//!
//! Daemon HID'i tekelde tutar; CLI dogrudan cihaza dokunmaz. Boylece
//! tray/gui/cli ayni anda cakismadan calisir.

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use zbus::Connection;

#[derive(Parser)]
#[command(
    name = "blackshark",
    version,
    about = "Razer BlackShark V3 X HyperSpeed (Xbox) CLI (daemon uzerinden)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Durum: batarya + baglanti (--json ile makine-okunur)
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Batarya seviyesini oku
    Battery,
    /// EQ preset: default | game | movie | music
    EqPreset { name: String },
    /// Custom EQ: 10 bant dB (-6..6). 31..16k
    Eq {
        #[arg(num_args = 10, value_name = "dB", allow_negative_numbers = true)]
        bands: Vec<i32>,
    },
    /// Sidetone 0..15
    Sidetone { level: u32 },
    /// Noise cancellation: on | off
    NoiseCancel { state: String },
    /// Uyku zamanlayicisi (dakika)
    Sleep { minutes: u32 },
    /// Guc tasarrufu: on | off
    PowerSave { state: String },
    /// Kayitli config'i cihaza yeniden uygula
    Apply,
}

fn parse_on_off(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "on" | "1" | "true" | "ac" | "acik" => Ok(true),
        "off" | "0" | "false" | "kapat" | "kapali" => Ok(false),
        _ => Err(anyhow!("gecersiz deger '{}': on/off bekleniyor", s)),
    }
}

/// Daemon'a D-Bus proxy.
struct DaemonProxy {
    conn: Connection,
}

impl DaemonProxy {
    async fn connect() -> Result<Self> {
        let conn = Connection::session()
            .await
            .map_err(|e| anyhow!("D-Bus baglantisi kurulamadi: {e}. Daemon calisiyor mu? (systemctl --user status blacksharkd)"))?;
        Ok(Self { conn })
    }

    async fn call<A: serde::Serialize + zbus::zvariant::DynamicType>(
        &self,
        method: &str,
        args: &A,
    ) -> Result<()> {
        self.conn
            .call_method(
                Some("net.blackshark1"),
                "/net/blackshark1/Headset",
                Some("net.blackshark1.Headset"),
                method,
                args,
            )
            .await
            .map_err(|e| anyhow!("daemon cagrisi basarisiz ({method}): {e}"))?;
        Ok(())
    }

    async fn get_property<T>(&self, name: &str) -> Result<T>
    where
        T: for<'a> zbus::zvariant::Type + serde::de::DeserializeOwned + TryFrom<zbus::zvariant::OwnedValue>,
        <T as TryFrom<zbus::zvariant::OwnedValue>>::Error: std::fmt::Display,
    {
        let proxy = zbus::Proxy::new(
            &self.conn,
            "net.blackshark1",
            "/net/blackshark1/Headset",
            "net.blackshark1.Headset",
        )
        .await?;
        let v = proxy
            .get_property::<T>(name)
            .await
            .map_err(|e| anyhow!("property okunamadi ({name}): {e}"))?;
        Ok(v)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let daemon = DaemonProxy::connect().await?;

    match cli.command {
        Command::Status { json } => {
            let battery: i32 = daemon.get_property("Battery").await.unwrap_or(-1);
            let connected: bool = daemon.get_property("Connected").await.unwrap_or(false);
            if json {
                println!(
                    "{{\"connected\":{},\"battery\":{}}}",
                    connected, battery
                );
            } else {
                println!("Bagli   : {}", if connected { "evet" } else { "hayir" });
                if battery >= 0 {
                    println!("Batarya : %{battery}");
                } else {
                    println!("Batarya : bilinmiyor");
                }
            }
        }
        Command::Battery => {
            let battery: i32 = daemon.get_property("Battery").await.unwrap_or(-1);
            if battery >= 0 {
                println!("Batarya: %{battery}");
            } else {
                println!("Batarya: bilinmiyor");
            }
        }
        Command::EqPreset { name } => {
            daemon.call("SetEqPreset", &(name.clone(),)).await?;
            println!("EQ preset -> {name}");
        }
        Command::Eq { bands } => {
            if bands.len() != 10 {
                return Err(anyhow!("10 bant gerekli, {} verildi", bands.len()));
            }
            daemon.call("SetEq", &(bands.clone(),)).await?;
            println!("Custom EQ -> {:?}", bands);
        }
        Command::Sidetone { level } => {
            daemon.call("SetSidetone", &(level,)).await?;
            println!("Sidetone -> {}/15", level.min(15));
        }
        Command::NoiseCancel { state } => {
            let on = parse_on_off(&state)?;
            daemon.call("SetNoiseCancel", &(on,)).await?;
            println!("Noise Cancellation -> {}", if on { "ON" } else { "OFF" });
        }
        Command::Sleep { minutes } => {
            daemon.call("SetSleep", &(minutes,)).await?;
            println!("Sleep timer -> {minutes} dk");
        }
        Command::PowerSave { state } => {
            let on = parse_on_off(&state)?;
            daemon.call("SetPowerSave", &(on,)).await?;
            println!("Power Saving -> {}", if on { "ON" } else { "OFF" });
        }
        Command::Apply => {
            daemon.call("Apply", &()).await?;
            println!("Config uygulandi");
        }
    }

    Ok(())
}
