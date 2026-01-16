//! Serial port transport layer.
//!
//! Handles USB serial communication with meshgrid/MeshCore devices.
//! Supports COBS (Consistent Overhead Byte Stuffing) framing.

use anyhow::{Context, Result};
use std::time::Duration;
use tokio_serial::SerialPortBuilderExt;

/// COBS encode a buffer
/// Returns the encoded data (without the zero terminator)
fn cobs_encode(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(data.len() + (data.len() / 254) + 1);
    let mut code_ptr = 0;
    encoded.push(0); // Placeholder for code byte
    let mut code = 1u8;

    for &byte in data {
        if byte == 0 {
            // Found zero - write code byte
            encoded[code_ptr] = code;
            code_ptr = encoded.len();
            encoded.push(0); // Placeholder for next code byte
            code = 1;
        } else {
            encoded.push(byte);
            code = code.wrapping_add(1);
            if code == 0xFF {
                // Code byte full - write it
                encoded[code_ptr] = code;
                code_ptr = encoded.len();
                encoded.push(0); // Placeholder for next code byte
                code = 1;
            }
        }
    }

    // Write final code byte
    encoded[code_ptr] = code;
    encoded
}

/// COBS decode a buffer
/// Returns the decoded data, or None if invalid
fn cobs_decode(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return Some(Vec::new());
    }

    let mut decoded = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        let code = data[i];
        if code == 0 {
            return None; // Invalid
        }
        i += 1;

        // Copy data bytes
        for _ in 1..code {
            if i >= data.len() {
                break;
            }
            decoded.push(data[i]);
            i += 1;
        }

        // Insert zero if not at end
        if code < 0xFF && i < data.len() {
            decoded.push(0);
        }
    }

    Some(decoded)
}

/// Serial port connection.
pub struct SerialPort {
    port: tokio_serial::SerialStream,
    read_buf: Vec<u8>,
}

impl SerialPort {
    /// Open a serial port connection.
    pub async fn open(port_name: &str, baud_rate: u32) -> Result<Self> {
        use tokio_serial::SerialPort as _;

        let mut port = tokio_serial::new(port_name, baud_rate)
            .data_bits(tokio_serial::DataBits::Eight)
            .stop_bits(tokio_serial::StopBits::One)
            .parity(tokio_serial::Parity::None)
            .flow_control(tokio_serial::FlowControl::None)
            .timeout(Duration::from_millis(100))
            .open_native_async()
            .with_context(|| format!("Failed to open serial port: {}", port_name))?;

        // ESP32-S3 native USB (ttyACM) - DON'T toggle DTR/RTS as it triggers reset!
        // The auto-reset circuit uses DTR+RTS to enter bootloader or reset.
        // Set both HIGH to avoid triggering reset.
        let is_native_usb = port_name.contains("ttyACM") || port_name.contains("cu.usb");

        if is_native_usb {
            // Set DTR and RTS high to avoid reset (low triggers reset on ESP32)
            let _ = port.write_data_terminal_ready(true);
            let _ = port.write_request_to_send(true);
            // ESP32-S3 native USB needs extra time after boot
            // The firmware has a 2s delay + boot messages before it's ready
            tokio::time::sleep(Duration::from_millis(200)).await;
        } else {
            // Small delay for USB CDC to stabilize
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(Self {
            port,
            read_buf: Vec::with_capacity(4096),
        })
    }

    /// Write raw bytes to the serial port.
    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        self.port.write_all(data).await?;
        self.port.flush().await?;
        Ok(())
    }

    /// Read a line from the serial port.
    pub async fn read_line(&mut self) -> Result<String> {
        use tokio::io::AsyncReadExt;

        loop {
            // Check if we have a complete line in buffer
            if let Some(pos) = self.read_buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = self.read_buf.drain(..=pos).collect();
                let s = String::from_utf8_lossy(&line[..line.len()-1]).trim_end().to_string();
                return Ok(s);
            }

            // Read more data
            let mut tmp = [0u8; 256];
            let n = self.port.read(&mut tmp).await?;
            if n == 0 {
                anyhow::bail!("EOF on serial port");
            }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }
    }

    /// Read a line with timeout.
    pub async fn read_line_timeout(&mut self, timeout: Duration) -> Result<Option<String>> {
        match tokio::time::timeout(timeout, self.read_line()).await {
            Ok(Ok(line)) => Ok(Some(line)),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None), // Timeout
        }
    }

    /// Read raw bytes (up to buf size).
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        use tokio::io::AsyncReadExt;
        let n = self.port.read(buf).await?;
        Ok(n)
    }

    /// Read raw bytes with timeout.
    pub async fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<Option<usize>> {
        match tokio::time::timeout(timeout, self.read(buf)).await {
            Ok(Ok(n)) => Ok(Some(n)),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None),
        }
    }

    /// Clear input/output buffers and wait for device to be ready.
    pub async fn clear(&mut self) -> Result<()> {
        // Clear read buffer
        self.read_buf.clear();

        // Drain any pending data (boot messages, etc.)
        // Use longer timeout to catch all buffered output
        let mut buf = [0u8; 1024];
        let start = std::time::Instant::now();
        let max_drain_time = Duration::from_millis(500);

        while start.elapsed() < max_drain_time {
            match self.read_timeout(&mut buf, Duration::from_millis(100)).await {
                Ok(Some(n)) if n > 0 => continue, // More data, keep draining
                _ => break, // Timeout or error, buffer is empty
            }
        }

        Ok(())
    }

    /// Write a COBS-encoded frame (with zero terminator)
    pub async fn write_cobs_frame(&mut self, data: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let encoded = cobs_encode(data);
        self.port.write_all(&encoded).await?;
        self.port.write_all(&[0]).await?; // COBS frame delimiter
        self.port.flush().await?;
        Ok(())
    }

    /// Read a COBS-encoded frame (blocking until zero byte)
    pub async fn read_cobs_frame(&mut self) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        let mut encoded = Vec::new();
        loop {
            // Check if we have a zero byte in buffer
            if let Some(pos) = self.read_buf.iter().position(|&b| b == 0) {
                encoded.extend_from_slice(&self.read_buf[..pos]);
                self.read_buf.drain(..=pos);
                break;
            }

            // Read more data
            let mut tmp = [0u8; 256];
            let n = self.port.read(&mut tmp).await?;
            if n == 0 {
                anyhow::bail!("EOF on serial port");
            }
            self.read_buf.extend_from_slice(&tmp[..n]);
        }

        // Decode COBS
        cobs_decode(&encoded)
            .ok_or_else(|| anyhow::anyhow!("Invalid COBS frame"))
    }

    /// Read a COBS frame with timeout
    pub async fn read_cobs_frame_timeout(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        match tokio::time::timeout(timeout, self.read_cobs_frame()).await {
            Ok(Ok(frame)) => Ok(Some(frame)),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None), // Timeout
        }
    }
}

/// Auto-detect a connected meshgrid/MeshCore device.
pub fn detect_device() -> Result<Option<String>> {
    let ports = serialport::available_ports()?;

    for port in ports {
        if let serialport::SerialPortType::UsbPort(info) = port.port_type {
            // ESP32-S3 native USB (T3S3, Heltec V3/V4, Station G2)
            if info.vid == 0x303a {
                return Ok(Some(port.port_name));
            }
            // Silicon Labs CP210x (common on ESP32 dev boards)
            if info.vid == 0x10c4 && info.pid == 0xea60 {
                return Ok(Some(port.port_name));
            }
            // CH340 (Heltec, some clones)
            if info.vid == 0x1a86 && info.pid == 0x7523 {
                return Ok(Some(port.port_name));
            }
            // Seeed devices
            if info.vid == 0x239a {
                return Ok(Some(port.port_name));
            }
            // Nordic Semiconductor (RAK4631 has nRF52840)
            if info.vid == 0x1915 {
                return Ok(Some(port.port_name));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_no_panic() {
        // Should not panic even if no devices connected
        let _ = detect_device();
    }
}
