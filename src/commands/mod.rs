//! Command implementations

pub mod info;
pub mod messaging;
pub mod config;
pub mod network;
pub mod system;
pub mod util;

// Re-export command functions
pub use info::*;
pub use messaging::*;
pub use config::*;
pub use network::*;
pub use system::*;
pub use util::*;

use anyhow::Result;
use crate::device::Device;

/// Connect to device and authenticate if PIN provided
pub async fn connect_with_auth(port: &str, baud: u32, pin: Option<&str>) -> Result<Device> {
    let mut dev = Device::connect(port, baud).await?;

    // Authenticate if PIN provided
    if let Some(pin_str) = pin {
        dev.authenticate(pin_str).await?;
    }

    Ok(dev)
}
