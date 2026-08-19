//! blackshark - CLI client that connects to the daemon over D-Bus.
//!
//! The daemon exclusively owns the HID device; the CLI never touches it directly.
//! This lets the tray, GUI, and CLI run at the same time without conflicts.

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use zbus::Connection;

#[derive(Parser)]
#[command(
    name = "blackshark",
    version,
    about = "Razer BlackShark V3 X HyperSpeed (Xbox) CLI client"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show battery and connection status; use --json for machine-readable output.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Read the battery level.
    Battery,
    /// EQ preset: default | game | movie | music
    EqPreset { name: String },
    /// Custom EQ: 10 dB bands (-6..6), from 31 Hz to 16 kHz.
    Eq {
        #[arg(num_args = 10, value_name = "dB", allow_negative_numbers = true)]
        bands: Vec<i32>,
    },
    /// Sidetone 0..15
    Sidetone { level: u32 },
    /// Noise cancellation: on | off
    NoiseCancel { state: String },
    /// Sleep timer in minutes.
    Sleep { minutes: u32 },
    /// Power saving: on | off.
    PowerSave { state: String },
    /// Reapply the saved configuration to the device.
    Apply,
}

fn parse_on_off(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "on" | "1" | "true" => Ok(true),
        "off" | "0" | "false" => Ok(false),
        _ => Err(anyhow!("invalid value '{}': expected on or off", s)),
    }
}

/// D-Bus proxy for the daemon.
struct DaemonProxy {
    conn: Connection,
}

impl DaemonProxy {
    async fn connect() -> Result<Self> {
        let conn = Connection::session()
            .await
            .map_err(|e| anyhow!("could not connect to D-Bus: {e}. Is the daemon running? (systemctl --user status blacksharkd)"))?;
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
            .map_err(|e| anyhow!("daemon call failed ({method}): {e}"))?;
        Ok(())
    }

    async fn get_property<T>(&self, name: &str) -> Result<T>
    where
        T: for<'a> zbus::zvariant::Type
            + serde::de::DeserializeOwned
            + TryFrom<zbus::zvariant::OwnedValue>,
        <T as TryFrom<zbus::zvariant::OwnedValue>>::Error: Into<zbus::Error>,
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
            .map_err(|e| anyhow!("could not read property ({name}): {e}"))?;
        Ok(v)
    }

    async fn refresh_battery(&self) -> Result<i32> {
        let proxy = zbus::Proxy::new(
            &self.conn,
            "net.blackshark1",
            "/net/blackshark1/Headset",
            "net.blackshark1.Headset",
        )
        .await?;
        proxy
            .call("RefreshBattery", &())
            .await
            .map_err(|e| anyhow!("could not refresh battery: {e}"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let daemon = DaemonProxy::connect().await?;

    match cli.command {
        Command::Status { json } => {
            let battery = daemon.refresh_battery().await.unwrap_or(-1);
            let connected: bool = daemon.get_property("Connected").await.unwrap_or(false);
            if json {
                println!("{{\"connected\":{},\"battery\":{}}}", connected, battery);
            } else {
                println!("Connected : {}", if connected { "yes" } else { "no" });
                if battery >= 0 {
                    println!("Battery   : {battery}%");
                } else {
                    println!("Battery   : unknown");
                }
            }
        }
        Command::Battery => {
            let battery = daemon.refresh_battery().await.unwrap_or(-1);
            if battery >= 0 {
                println!("Battery: {battery}%");
            } else {
                println!("Battery: unknown");
            }
        }
        Command::EqPreset { name } => {
            daemon.call("SetEqPreset", &(name.clone(),)).await?;
            println!("EQ preset -> {name}");
        }
        Command::Eq { bands } => {
            if bands.len() != 10 {
                return Err(anyhow!("10 bands are required; received {}", bands.len()));
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
            println!("Sleep timer -> {minutes} min");
        }
        Command::PowerSave { state } => {
            let on = parse_on_off(&state)?;
            daemon.call("SetPowerSave", &(on,)).await?;
            println!("Power Saving -> {}", if on { "ON" } else { "OFF" });
        }
        Command::Apply => {
            daemon.call("Apply", &()).await?;
            println!("Configuration applied");
        }
    }

    Ok(())
}
