//! Utility commands

use anyhow::Result;

/// List available serial ports
pub fn cmd_list_ports() -> Result<()> {
    println!("Available serial ports:\n");

    let ports = serialport::available_ports()?;

    if ports.is_empty() {
        println!("  No serial ports found");
        return Ok(());
    }

    for port in ports {
        print!("  {} ", port.port_name);

        if let serialport::SerialPortType::UsbPort(info) = port.port_type {
            print!("(USB");
            if let Some(manufacturer) = info.manufacturer {
                print!(" - {manufacturer}");
            }
            if let Some(product) = info.product {
                print!(" {product}");
            }
            print!(")");
        }

        println!();
    }

    Ok(())
}

/// Require port or auto-detect
pub fn require_port(port: Option<&String>) -> Result<String> {
    if let Some(p) = port {
        return Ok(p.clone());
    }

    // Try auto-detection
    if let Some(detected) = crate::serial::detect_device()? {
        println!("Auto-detected device: {detected}");
        return Ok(detected);
    }

    anyhow::bail!(
        "No port specified and no device auto-detected.\nUse -p /dev/ttyUSB0 or run 'meshgrid ports' to list available ports"
    )
}
