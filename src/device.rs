//! High-level device interface.
//!
//! Wraps the protocol layer with a user-friendly API.

use anyhow::Result;

use crate::protocol::{Protocol, MonitorEvent};
use crate::serial::SerialPort;

/// High-level device interface.
pub struct Device {
    protocol: Protocol,
}

impl Device {
    /// Connect to a device.
    pub async fn connect(port: &str, baud: u32) -> Result<Self> {
        let serial = SerialPort::open(port, baud).await?;
        let protocol = Protocol::new(serial);

        Ok(Self { protocol })
    }

    /// Get device info.
    pub async fn get_info(&mut self) -> Result<DeviceInfo> {
        let info = self.protocol.get_info().await?;

        Ok(DeviceInfo {
            name: info.name,
            public_key: info.public_key,
            node_hash: info.node_hash,
            firmware_version: info.firmware_version,
            mode: info.mode,
            freq_mhz: info.freq_mhz,
            tx_power_dbm: info.tx_power_dbm,
        })
    }

    /// Get device configuration.
    pub async fn get_config(&mut self) -> Result<DeviceConfig> {
        let config = self.protocol.get_config().await?;

        Ok(DeviceConfig {
            name: config.name,
            freq_mhz: config.freq_mhz,
            tx_power_dbm: config.tx_power_dbm,
            bandwidth_khz: config.bandwidth_khz,
            spreading_factor: config.spreading_factor,
            coding_rate: config.coding_rate,
            preamble_len: config.preamble_len,
        })
    }

    /// Set device name.
    pub async fn set_name(&mut self, name: &str) -> Result<()> {
        self.protocol.set_name(name).await
    }

    /// Set LoRa frequency.
    pub async fn set_frequency(&mut self, freq_mhz: f32) -> Result<()> {
        self.protocol.set_frequency(freq_mhz).await
    }

    /// Set TX power.
    pub async fn set_power(&mut self, dbm: i8) -> Result<()> {
        self.protocol.set_power(dbm).await
    }

    /// Set radio preset.
    pub async fn set_preset(&mut self, preset: &str) -> Result<()> {
        let cmd = format!("SET PRESET {}", preset.to_uppercase());
        self.protocol.command(&cmd).await?;
        Ok(())
    }

    /// Set bandwidth.
    pub async fn set_bandwidth(&mut self, bandwidth_khz: f32) -> Result<()> {
        let cmd = format!("SET BW {}", bandwidth_khz);
        self.protocol.command(&cmd).await?;
        Ok(())
    }

    /// Set spreading factor.
    pub async fn set_spreading_factor(&mut self, sf: u8) -> Result<()> {
        let cmd = format!("SET SF {}", sf);
        self.protocol.command(&cmd).await?;
        Ok(())
    }

    /// Get neighbor table.
    pub async fn get_neighbors(&mut self) -> Result<Vec<NeighborInfo>> {
        let neighbors = self.protocol.get_neighbors().await?;

        Ok(neighbors
            .into_iter()
            .map(|n| NeighborInfo {
                node_hash: n.node_hash,
                name: n.name,
                rssi: n.rssi,
                snr: n.snr,
                last_seen_secs: n.last_seen_secs,
            })
            .collect())
    }

    /// Send a broadcast message.
    pub async fn send_broadcast(&mut self, message: &str) -> Result<()> {
        self.protocol.send_broadcast(message).await
    }

    /// Send a direct message.
    pub async fn send_direct(&mut self, dest: &str, message: &str) -> Result<()> {
        // If dest is not a hash (0x...), look it up in neighbors
        let resolved_dest = if !dest.starts_with("0x") {
            // Try to find neighbor by name
            let neighbors = self.get_neighbors().await?;
            if let Some(neighbor) = neighbors.iter().find(|n| {
                n.name.as_ref().map(|name| name.as_str()) == Some(dest)
            }) {
                format!("0x{:x}", neighbor.node_hash)
            } else {
                // Not found in neighbors, try sending anyway (maybe it's a partial match)
                dest.to_string()
            }
        } else {
            dest.to_string()
        };

        self.protocol.send_direct(&resolved_dest, message).await
    }

    /// Trace route to a target.
    pub async fn trace(&mut self, target: &str) -> Result<TraceResult> {
        let result = self.protocol.trace(target).await?;

        Ok(TraceResult {
            path: result.path,
            hop_count: result.hop_count,
            rtt_ms: result.rtt_ms,
        })
    }

    /// Reboot the device.
    pub async fn reboot(&mut self) -> Result<()> {
        self.protocol.reboot().await
    }

    /// Monitor mesh traffic.
    ///
    /// Calls the callback for each event. Returns when Ctrl+C is pressed.
    pub async fn monitor<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(MeshEvent),
    {
        // Enter monitor mode
        self.protocol.enter_monitor_mode().await?;

        // Set up Ctrl+C handler
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let r = running.clone();

        ctrlc_async(move || {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        })?;

        // Read events
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            if let Some(event) = self.protocol.read_event().await? {
                let mesh_event = match event {
                    MonitorEvent::Message { from, to, rssi, snr, text } => {
                        MeshEvent::Message { from, to, text, rssi, snr }
                    }
                    MonitorEvent::Advertisement { node_hash, rssi, name } => {
                        MeshEvent::Advertisement { node_hash, rssi, name }
                    }
                    MonitorEvent::Ack { from } => {
                        MeshEvent::Ack { from }
                    }
                    MonitorEvent::Error { message } => {
                        MeshEvent::Error { message }
                    }
                };
                callback(mesh_event);
            }

            // Small delay to prevent busy-waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Send a local advertisement (ROUTE_DIRECT).
    pub async fn send_advert_local(&mut self) -> Result<()> {
        self.protocol.command("ADVERT LOCAL").await?;
        Ok(())
    }

    /// Send a flood advertisement (ROUTE_FLOOD).
    pub async fn send_advert_flood(&mut self) -> Result<()> {
        self.protocol.command("ADVERT FLOOD").await?;
        Ok(())
    }

    /// Send a raw packet.
    pub async fn send_packet(&mut self, packet: &[u8]) -> Result<()> {
        self.protocol.send_packet(packet).await
    }

    /// Get the underlying protocol for advanced usage (e.g., raw packet receiving).
    pub fn into_protocol(self) -> Protocol {
        self.protocol
    }
}

/// Set up async Ctrl+C handler.
fn ctrlc_async<F>(callback: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    let callback = std::sync::Mutex::new(Some(callback));

    ctrlc::set_handler(move || {
        if let Some(cb) = callback.lock().unwrap().take() {
            cb();
        }
    })?;

    Ok(())
}

/// Device information.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: Option<String>,
    pub public_key: [u8; 32],
    pub node_hash: u8,
    pub firmware_version: Option<String>,
    pub mode: Option<String>,
    #[allow(dead_code)]
    pub freq_mhz: f32,
    #[allow(dead_code)]
    pub tx_power_dbm: i8,
}

/// Device configuration.
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub name: Option<String>,
    pub freq_mhz: f32,
    pub tx_power_dbm: i8,
    pub bandwidth_khz: u32,
    pub spreading_factor: u8,
    pub coding_rate: u8,
    pub preamble_len: u16,
}

/// Neighbor information.
#[derive(Debug, Clone)]
pub struct NeighborInfo {
    pub node_hash: u8,
    pub name: Option<String>,
    pub rssi: i16,
    pub snr: i8,
    pub last_seen_secs: u32,
}

/// Trace result.
#[derive(Debug, Clone)]
pub struct TraceResult {
    pub path: Vec<String>,
    pub hop_count: u8,
    pub rtt_ms: u32,
}

/// Mesh event for monitoring.
#[derive(Debug, Clone)]
pub enum MeshEvent {
    Message {
        from: String,
        to: Option<String>,
        text: String,
        rssi: i16,
        snr: i8,
    },
    Advertisement {
        node_hash: u8,
        name: Option<String>,
        rssi: i16,
    },
    Ack {
        from: String,
    },
    Error {
        message: String,
    },
}
