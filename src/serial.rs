//! Serial port transport layer.
//!
//! Handles USB serial communication with meshgrid/MeshCore devices.

use anyhow::{Context, Result};
use std::time::Duration;
use tokio_serial::SerialPortBuilderExt;

/// Serial port connection.
pub struct SerialPort {
    port: tokio_serial::SerialStream,
    read_buf: Vec<u8>,
}

impl SerialPort {
    /// Open a serial port connection.
    pub async fn open(port_name: &str, baud_rate: u32) -> Result<Self> {
        let port = tokio_serial::new(port_name, baud_rate)
            .data_bits(tokio_serial::DataBits::Eight)
            .stop_bits(tokio_serial::StopBits::One)
            .parity(tokio_serial::Parity::None)
            .flow_control(tokio_serial::FlowControl::None)
            .timeout(Duration::from_millis(100))
            .open_native_async()
            .with_context(|| format!("Failed to open serial port: {}", port_name))?;

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

    /// Write a line (with newline).
    pub async fn write_line(&mut self, line: &str) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        self.port.write_all(line.as_bytes()).await?;
        self.port.write_all(b"\n").await?;
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

        // With ARDUINO_USB_CDC_ON_BOOT=0, device doesn't reset on port open
        // Just drain any pending data
        let mut buf = [0u8; 1024];
        while let Ok(Some(n)) = self.read_timeout(&mut buf, Duration::from_millis(50)).await {
            if n == 0 {
                break;
            }
        }

        Ok(())
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
