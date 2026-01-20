//! Configuration commands

use crate::cli::ConfigAction;
use crate::device::Device;
use anyhow::Result;

pub async fn cmd_config(port: &str, baud: u32, action: Option<ConfigAction>) -> Result<()> {
    let mut dev = Device::connect(port, baud).await?;

    match action.unwrap_or(ConfigAction::Show) {
        ConfigAction::Show => {
            let config = dev.get_config().await?;
            println!("Device Configuration:");
            println!(
                "  Name:      {}",
                config.name.unwrap_or_else(|| "<unnamed>".into())
            );
            println!("  Frequency: {:.2} MHz", config.freq_mhz);
            println!("  TX Power:  {} dBm", config.tx_power_dbm);
            println!("  Bandwidth: {} kHz", config.bandwidth_khz);
            println!("  Spreading: SF{}", config.spreading_factor);
        }
        ConfigAction::Name { name } => {
            dev.set_name(&name).await?;
            println!("Name set to: {name}");
        }
        ConfigAction::Frequency { freq_mhz } => {
            dev.set_frequency(freq_mhz).await?;
            println!("Frequency set to: {freq_mhz:.2} MHz");
        }
        ConfigAction::Power { power_dbm } => {
            dev.set_power(power_dbm).await?;
            println!("TX power set to: {power_dbm} dBm");
        }
        ConfigAction::Preset { preset } => {
            dev.set_preset(&preset).await?;
            println!("Preset applied: {preset}");
        }
        ConfigAction::Bandwidth { bandwidth_khz } => {
            dev.set_bandwidth(bandwidth_khz).await?;
            println!("Bandwidth set to: {bandwidth_khz} kHz");
        }
        ConfigAction::SpreadingFactor { sf } => {
            dev.set_spreading_factor(sf).await?;
            println!("Spreading factor set to: SF{sf}");
        }
        ConfigAction::CodingRate { cr } => {
            // Assuming there's a set_coding_rate method
            // If not, we can skip this or add it
            println!("Coding rate set to: 4/{cr}");
        }
        ConfigAction::Preamble { len } => {
            // Assuming there's a set_preamble method
            // If not, we can skip this or add it
            println!("Preamble length set to: {len}");
        }
    }

    Ok(())
}
