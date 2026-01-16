//! MeshCore serial protocol implementation.
//!
//! This module implements the command protocol used by MeshCore firmware
//! for USB serial communication. Commands are text-based for simplicity.
//!
//! ## Command Format
//!
//! Commands are sent as text lines:
//! ```text
//! CMD [args...]\n
//! ```
//!
//! Responses are JSON or simple text:
//! ```text
//! OK [data]\n
//! ERR [message]\n
//! {"json": "response"}\n
//! ```
//!
//! ## Binary Packet Format
//!
//! For raw packet send/receive, binary format is used:
//! ```text
//! PKT <len>\n
//! <binary data>
//! ```

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::serial::SerialPort;

/// Device telemetry data.
#[derive(Debug, Clone, Default)]
pub struct DeviceTelemetry {
    pub battery_percent: u8,
    pub voltage_mv: u16,
    pub charging: bool,
    pub usb_power: bool,
    pub uptime_secs: u32,
    pub free_heap: u32,
    pub cpu_temp_deci_c: i16,
}

impl DeviceTelemetry {
    pub fn new() -> Self { Self::default() }
    pub fn cpu_temp_celsius(&self) -> f32 { self.cpu_temp_deci_c as f32 / 10.0 }
    pub fn voltage(&self) -> f32 { self.voltage_mv as f32 / 1000.0 }
}

/// Environment telemetry data.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentTelemetry {
    temp_deci_c: i16,
    humidity_deci_pct: u16,
    pressure_deci_hpa: u32,
    pub air_quality: u16,
}

impl EnvironmentTelemetry {
    pub fn new() -> Self { Self::default() }
    pub fn with_temperature(mut self, t: f32) -> Self { self.temp_deci_c = (t * 10.0) as i16; self }
    pub fn with_humidity(mut self, h: f32) -> Self { self.humidity_deci_pct = (h * 10.0) as u16; self }
    pub fn with_pressure_hpa(mut self, p: f32) -> Self { self.pressure_deci_hpa = (p * 10.0) as u32; self }
    pub fn temperature_celsius(&self) -> f32 { self.temp_deci_c as f32 / 10.0 }
    pub fn humidity_percent(&self) -> f32 { self.humidity_deci_pct as f32 / 10.0 }
    pub fn pressure_hpa(&self) -> f32 { self.pressure_deci_hpa as f32 / 10.0 }
}

/// Location telemetry data.
#[derive(Debug, Clone, Default)]
pub struct LocationTelemetry {
    lat_micro: i32,
    lon_micro: i32,
    alt_cm: i32,
    speed_cm_s: u16,
    heading_deci: u16,
    pub satellites: u8,
    pub fix_type: u8,
}

impl LocationTelemetry {
    pub fn new() -> Self { Self::default() }
    pub fn with_latitude(mut self, lat: f64) -> Self { self.lat_micro = (lat * 1_000_000.0) as i32; self }
    pub fn with_longitude(mut self, lon: f64) -> Self { self.lon_micro = (lon * 1_000_000.0) as i32; self }
    pub fn with_altitude(mut self, alt: f32) -> Self { self.alt_cm = (alt * 100.0) as i32; self }
    pub fn with_speed(mut self, spd: f32) -> Self { self.speed_cm_s = (spd * 100.0) as u16; self }
    pub fn with_heading(mut self, hdg: f32) -> Self { self.heading_deci = (hdg * 10.0) as u16; self }
    pub fn has_fix(&self) -> bool { self.fix_type > 0 }
    pub fn latitude(&self) -> f64 { self.lat_micro as f64 / 1_000_000.0 }
    pub fn longitude(&self) -> f64 { self.lon_micro as f64 / 1_000_000.0 }
    pub fn altitude_meters(&self) -> f32 { self.alt_cm as f32 / 100.0 }
    pub fn speed_m_s(&self) -> f32 { self.speed_cm_s as f32 / 100.0 }
    pub fn heading_degrees(&self) -> f32 { self.heading_deci as f32 / 10.0 }
}

/// Combined telemetry.
#[derive(Debug, Clone, Default)]
pub struct Telemetry {
    pub device: Option<DeviceTelemetry>,
    pub environment: Option<EnvironmentTelemetry>,
    pub location: Option<LocationTelemetry>,
}

impl Telemetry {
    pub fn new() -> Self { Self::default() }
    pub fn with_device(mut self, d: DeviceTelemetry) -> Self { self.device = Some(d); self }
    pub fn with_environment(mut self, e: EnvironmentTelemetry) -> Self { self.environment = Some(e); self }
    pub fn with_location(mut self, l: LocationTelemetry) -> Self { self.location = Some(l); self }
}

/// Command timeout.
const CMD_TIMEOUT: Duration = Duration::from_secs(5);

/// Response from device.
#[derive(Debug, Clone)]
pub enum Response {
    /// Command succeeded, optionally with message
    Ok(Option<String>),
    /// Command failed with error message
    Error(String),
    /// JSON data response
    Json(serde_json::Value),
}

/// Device info response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: Option<String>,
    pub public_key: [u8; 32],
    pub node_hash: u8,
    pub firmware_version: Option<String>,
    pub mode: Option<String>,
    pub freq_mhz: f32,
    pub tx_power_dbm: i8,
}

/// Device configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub name: Option<String>,
    pub freq_mhz: f32,
    pub tx_power_dbm: i8,
    pub bandwidth_khz: u32,
    pub spreading_factor: u8,
    pub coding_rate: u8,
    pub preamble_len: u16,
}

/// Neighbor entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborInfo {
    pub node_hash: u8,
    pub name: Option<String>,
    pub public_key: Option<[u8; 32]>,
    pub rssi: i16,
    pub snr: i8,
    pub last_seen_secs: u32,
    pub firmware: Option<String>,
}

/// Trace result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResult {
    pub path: Vec<String>,
    pub hop_count: u8,
    pub rtt_ms: u32,
}

/// MeshCore protocol handler.
pub struct Protocol {
    port: SerialPort,
}

impl Protocol {
    /// Create a new protocol handler.
    pub fn new(port: SerialPort) -> Self {
        Self { port }
    }

    /// Send a command and wait for response.
    pub async fn command(&mut self, cmd: &str) -> Result<Response> {
        // Clear any pending data/responses
        self.port.clear().await?;

        // Send command as COBS frame
        self.port.write_cobs_frame(cmd.as_bytes()).await?;

        // Wait for response
        self.read_response().await
    }

    /// Read a response from the device.
    async fn read_response(&mut self) -> Result<Response> {
        // Loop to skip debug frames and wait for command response
        // Limit iterations to prevent infinite loops on stuck devices
        const MAX_SKIP_FRAMES: usize = 50;
        let mut skip_count = 0;

        loop {
            if skip_count >= MAX_SKIP_FRAMES {
                bail!("Too many unrecognized frames - device may be in a crash loop");
            }

            // Read COBS frame
            let frame = match self.port.read_cobs_frame_timeout(CMD_TIMEOUT).await? {
                Some(frame) => frame,
                None => bail!("Command timeout"),
            };

            // Convert to string
            let line = String::from_utf8_lossy(&frame).to_string();
            tracing::debug!("Raw response: {:?}", line);

            // Check if it's a debug frame - skip it
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                if json.get("type").and_then(|v| v.as_str()) == Some("debug") {
                    // This is a debug frame - skip it and continue
                    tracing::debug!("Skipping debug frame: {:?}", json);
                    skip_count += 1;
                    continue;
                }
            }

            // Parse response
            if line.starts_with("OK") {
                let data = line.strip_prefix("OK").map(|s| s.trim().to_string());
                let data = if data.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    None
                } else {
                    data
                };
                return Ok(Response::Ok(data));
            } else if line.starts_with("ERR") {
                let msg = line.strip_prefix("ERR").unwrap_or(&line).trim().to_string();
                return Ok(Response::Error(msg));
            } else if line.starts_with('{') || line.starts_with('[') {
                // JSON object or array (including empty arrays)
                let json: serde_json::Value = serde_json::from_str(&line)?;
                return Ok(Response::Json(json));
            } else if line.starts_with("PKT") {
                // Binary packet - treat as OK (actual packet reading done via recv_packet)
                return Ok(Response::Ok(Some(line)));
            } else if line.starts_with("PONG") {
                // PING response
                return Ok(Response::Ok(Some(line)));
            } else {
                // Skip unrecognized frames
                tracing::debug!("Skipping unrecognized frame: {:?}", line);
                skip_count += 1;
                continue;
            }
        }
    }

    /// Get device info.
    pub async fn get_info(&mut self) -> Result<DeviceInfo> {
        match self.command("INFO").await? {
            Response::Json(json) => {
                let info: DeviceInfo = serde_json::from_value(json)?;
                Ok(info)
            }
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to INFO"),
        }
    }

    /// Get device configuration.
    pub async fn get_config(&mut self) -> Result<DeviceConfig> {
        match self.command("CONFIG").await? {
            Response::Json(json) => {
                let config: DeviceConfig = serde_json::from_value(json)?;
                Ok(config)
            }
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to CONFIG"),
        }
    }

    /// Set device name.
    pub async fn set_name(&mut self, name: &str) -> Result<()> {
        let cmd = format!("SET NAME {}", name);
        match self.command(&cmd).await? {
            Response::Ok(_) => Ok(()),
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to SET NAME"),
        }
    }

    /// Set LoRa frequency.
    pub async fn set_frequency(&mut self, freq_mhz: f32) -> Result<()> {
        let cmd = format!("SET FREQ {:.2}", freq_mhz);
        match self.command(&cmd).await? {
            Response::Ok(_) => Ok(()),
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to SET FREQ"),
        }
    }

    /// Set TX power.
    pub async fn set_power(&mut self, dbm: i8) -> Result<()> {
        let cmd = format!("SET POWER {}", dbm);
        match self.command(&cmd).await? {
            Response::Ok(_) => Ok(()),
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to SET POWER"),
        }
    }

    /// Get neighbor table.
    pub async fn get_neighbors(&mut self) -> Result<Vec<NeighborInfo>> {
        match self.command("NEIGHBORS").await? {
            Response::Json(json) => {
                let neighbors: Vec<NeighborInfo> = serde_json::from_value(json)?;
                Ok(neighbors)
            }
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to NEIGHBORS"),
        }
    }

    /// Send a broadcast message.
    pub async fn send_broadcast(&mut self, message: &str) -> Result<()> {
        let cmd = format!("SEND {}", message);
        match self.command(&cmd).await? {
            Response::Ok(_) => Ok(()),
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to SEND"),
        }
    }

    /// Send a trace packet.
    pub async fn trace(&mut self, target: &str) -> Result<TraceResult> {
        let cmd = format!("TRACE {}", target);

        // Send command and get initial response (status="sent")
        match self.command(&cmd).await? {
            Response::Json(_) => {
                // Initial "sent" response - now wait for trace_response
            }
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to TRACE"),
        }

        // Wait for trace_response with timeout (max 10 seconds)
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                bail!("Trace timeout - no response from target");
            }

            // Read a line
            match self.port.read_line_timeout(Duration::from_millis(500)).await? {
                Some(line) => {
                    // Try to parse as JSON
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        // Check if it's a trace_response
                        if json.get("type").and_then(|v| v.as_str()) == Some("trace_response") {
                            // Extract path
                            let path = json.get("path")
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect())
                                .unwrap_or_default();

                            // Extract hop count
                            let hop_count = json.get("hops")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u8;

                            // Extract RTT if available
                            let rtt_ms = json.get("rtt_ms")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;

                            return Ok(TraceResult {
                                path,
                                hop_count,
                                rtt_ms,
                            });
                        }
                    }
                }
                None => {
                    // No data yet, continue waiting
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Reboot the device.
    pub async fn reboot(&mut self) -> Result<()> {
        match self.command("REBOOT").await? {
            Response::Ok(_) => Ok(()),
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to REBOOT"),
        }
    }

    /// Enter monitor mode - returns an async stream of events.
    pub async fn enter_monitor_mode(&mut self) -> Result<()> {
        match self.command("MONITOR").await? {
            Response::Ok(_) => Ok(()),
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to MONITOR"),
        }
    }

    /// Read next event in monitor mode.
    pub async fn read_event(&mut self) -> Result<Option<MonitorEvent>> {
        let line = match self.port.read_line_timeout(Duration::from_millis(100)).await? {
            Some(line) => line,
            None => return Ok(None),
        };

        // Parse event
        if line.starts_with("MSG ") {
            // Format: MSG <from> <to> <rssi> <snr> <text>
            let parts: Vec<&str> = line.splitn(6, ' ').collect();
            if parts.len() >= 6 {
                return Ok(Some(MonitorEvent::Message {
                    from: parts[1].to_string(),
                    to: if parts[2] == "*" { None } else { Some(parts[2].to_string()) },
                    rssi: parts[3].parse().unwrap_or(0),
                    // snr: parts[4] - ignored
                    text: parts[5].to_string(),
                }));
            }
        } else if line.starts_with("ADV ") {
            // Format: ADV <hash> <rssi> <name>
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if parts.len() >= 3 {
                let hash = u8::from_str_radix(parts[1].trim_start_matches("0x"), 16).unwrap_or(0);
                return Ok(Some(MonitorEvent::Advertisement {
                    node_hash: hash,
                    rssi: parts[2].parse().unwrap_or(0),
                    name: parts.get(3).map(|s| s.to_string()),
                }));
            }
        } else if line.starts_with("ACK ") {
            // Format: ACK <from>
            let from = line.strip_prefix("ACK ").unwrap_or("?").to_string();
            return Ok(Some(MonitorEvent::Ack { from }));
        } else if line.starts_with("ERR ") {
            let msg = line.strip_prefix("ERR ").unwrap_or(&line).to_string();
            return Ok(Some(MonitorEvent::Error { message: msg }));
        }

        Ok(None)
    }

    /// Send a raw packet.
    pub async fn send_packet(&mut self, packet: &[u8]) -> Result<()> {
        let header = format!("PKT {}\n", packet.len());
        self.port.write(header.as_bytes()).await?;
        self.port.write(packet).await?;

        match self.read_response().await? {
            Response::Ok(msg) => {
                if let Some(m) = msg {
                    tracing::debug!("PKT response: {}", m);
                }
                Ok(())
            }
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to PKT"),
        }
    }

    /// Get device telemetry.
    pub async fn get_telemetry(&mut self) -> Result<Telemetry> {
        match self.command("TELEMETRY").await? {
            Response::Json(json) => {
                // Parse JSON telemetry response
                let mut telem = Telemetry::new();

                // Device telemetry
                if let Some(dev) = json.get("device") {
                    let mut dt = DeviceTelemetry::new();
                    if let Some(b) = dev.get("battery").and_then(|v| v.as_u64()) {
                        dt.battery_percent = b as u8;
                    }
                    if let Some(v) = dev.get("voltage").and_then(|v| v.as_f64()) {
                        dt.voltage_mv = (v * 1000.0) as u16;
                    }
                    if let Some(c) = dev.get("charging").and_then(|v| v.as_bool()) {
                        dt.charging = c;
                    }
                    if let Some(u) = dev.get("usb").and_then(|v| v.as_bool()) {
                        dt.usb_power = u;
                    }
                    if let Some(up) = dev.get("uptime").and_then(|v| v.as_u64()) {
                        dt.uptime_secs = up as u32;
                    }
                    if let Some(heap) = dev.get("heap").and_then(|v| v.as_u64()) {
                        dt.free_heap = heap as u32;
                    }
                    if let Some(temp) = dev.get("cpu_temp").and_then(|v| v.as_f64()) {
                        dt.cpu_temp_deci_c = (temp * 10.0) as i16;
                    }
                    telem = telem.with_device(dt);
                }

                // Environment telemetry
                if let Some(env) = json.get("environment") {
                    let mut et = EnvironmentTelemetry::new();
                    if let Some(t) = env.get("temperature").and_then(|v| v.as_f64()) {
                        et = et.with_temperature(t as f32);
                    }
                    if let Some(h) = env.get("humidity").and_then(|v| v.as_f64()) {
                        et = et.with_humidity(h as f32);
                    }
                    if let Some(p) = env.get("pressure").and_then(|v| v.as_f64()) {
                        et = et.with_pressure_hpa(p as f32);
                    }
                    if let Some(aq) = env.get("air_quality").and_then(|v| v.as_u64()) {
                        et.air_quality = aq as u16;
                    }
                    telem = telem.with_environment(et);
                }

                // Location telemetry
                if let Some(loc) = json.get("location") {
                    let mut lt = LocationTelemetry::new();
                    if let Some(lat) = loc.get("latitude").and_then(|v| v.as_f64()) {
                        lt = lt.with_latitude(lat);
                    }
                    if let Some(lon) = loc.get("longitude").and_then(|v| v.as_f64()) {
                        lt = lt.with_longitude(lon);
                    }
                    if let Some(alt) = loc.get("altitude").and_then(|v| v.as_f64()) {
                        lt = lt.with_altitude(alt as f32);
                    }
                    if let Some(spd) = loc.get("speed").and_then(|v| v.as_f64()) {
                        lt = lt.with_speed(spd as f32);
                    }
                    if let Some(hdg) = loc.get("heading").and_then(|v| v.as_f64()) {
                        lt = lt.with_heading(hdg as f32);
                    }
                    if let Some(sat) = loc.get("satellites").and_then(|v| v.as_u64()) {
                        lt.satellites = sat as u8;
                    }
                    if let Some(fix) = loc.get("fix").and_then(|v| v.as_u64()) {
                        lt.fix_type = fix as u8;
                    }
                    telem = telem.with_location(lt);
                }

                Ok(telem)
            }
            Response::Error(e) => bail!("Device error: {}", e),
            _ => bail!("Unexpected response to TELEMETRY"),
        }
    }

    /// Receive a raw packet (waits for incoming packet).
    pub async fn recv_packet(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        // Use read_response with custom timeout
        let line = match self.port.read_line_timeout(timeout).await? {
            Some(line) => line,
            None => return Ok(None),
        };

        // Check if it's a packet
        if line.starts_with("PKT") {
            let len_str = line.strip_prefix("PKT").unwrap_or("0").trim();
            let len: usize = len_str.parse()?;

            let mut buf = vec![0u8; len];
            let mut read = 0;
            while read < len {
                if let Some(n) = self.port.read_timeout(&mut buf[read..], CMD_TIMEOUT).await? {
                    read += n;
                } else {
                    bail!("Timeout reading packet data");
                }
            }
            Ok(Some(buf))
        } else {
            // Not a packet line, ignore
            Ok(None)
        }
    }

}

/// Monitor event types.
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    Message {
        from: String,
        to: Option<String>,
        rssi: i16,
        text: String,
    },
    Advertisement {
        node_hash: u8,
        rssi: i16,
        name: Option<String>,
    },
    Ack {
        from: String,
    },
    Error {
        message: String,
    },
}
