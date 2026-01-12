//! Device information commands

use anyhow::{Result, bail};
use super::connect_with_auth;
use crate::serial::SerialPort;
use crate::protocol::{Protocol, Response};

/// Show device information and configuration
pub async fn cmd_info(port: &str, baud: u32, pin: Option<&str>) -> Result<()> {
    let mut dev = connect_with_auth(port, baud, pin).await?;
    let info = dev.get_info().await?;
    let config = dev.get_config().await?;

    println!("Device Information:");
    println!("  Name:       {}", info.name.unwrap_or_else(|| "<unnamed>".into()));
    println!("  Mode:       {}", info.mode.unwrap_or_else(|| "unknown".into()));
    println!("  Public Key: {}", hex::encode(info.public_key));
    println!("  Node Hash:  0x{:02x}", info.node_hash);
    println!("  Firmware:   {}", info.firmware_version.unwrap_or_else(|| "unknown".into()));
    println!();
    println!("Radio Configuration:");
    println!("  Frequency:  {:.3} MHz", config.freq_mhz);
    println!("  TX Power:   {} dBm", config.tx_power_dbm);
    println!("  Bandwidth:  {} kHz", config.bandwidth_khz);
    println!("  SF:         {}", config.spreading_factor);
    println!("  CR:         4/{}", config.coding_rate);
    println!("  Preamble:   {}", config.preamble_len);

    Ok(())
}

/// Show device statistics
pub async fn cmd_stats(port: &str, baud: u32) -> Result<()> {
    let serial_port = SerialPort::open(port, baud).await?;
    let mut proto = Protocol::new(serial_port);

    // Request stats from device
    match proto.command("STATS").await? {
        Response::Json(json) => {
            // Format stats nicely
            println!("╔══════════════════════════════════════════╗");
            println!("║        MESHGRID PERFORMANCE STATS        ║");
            println!("╚══════════════════════════════════════════╝");

            // Hardware
            if let Some(hw) = json.get("hardware") {
                println!("\n📟 Hardware:");
                if let Some(board) = hw.get("board").and_then(|v| v.as_str()) {
                    println!("  Board:  {}", board);
                }
                if let Some(chip) = hw.get("chip").and_then(|v| v.as_str()) {
                    let mhz = hw.get("cpu_mhz").and_then(|v| v.as_u64()).unwrap_or(0);
                    let cores = hw.get("cores").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  CPU:    {} @ {} MHz ({} cores)", chip, mhz, cores);
                }
            }

            // Memory
            if let Some(mem) = json.get("memory") {
                println!("\n💾 Memory:");
                let ram_used = mem.get("ram_used_kb").and_then(|v| v.as_u64()).unwrap_or(0);
                let ram_total = mem.get("ram_total_kb").and_then(|v| v.as_u64()).unwrap_or(0);
                let ram_pct = if ram_total > 0 { (ram_used * 100) / ram_total } else { 0 };
                println!("  RAM:    {} / {} KB ({:.1}%)", ram_used, ram_total, ram_pct as f64);

                if let Some(heap) = mem.get("heap_free_kb").and_then(|v| v.as_u64()) {
                    println!("  Heap:   {} KB free", heap);
                }

                let flash_used = mem.get("flash_used_kb").and_then(|v| v.as_u64()).unwrap_or(0);
                let flash_total = mem.get("flash_total_kb").and_then(|v| v.as_u64()).unwrap_or(0);
                let flash_pct = if flash_total > 0 { (flash_used * 100) / flash_total } else { 0 };
                println!("  Flash:  {} / {} KB ({:.1}%)", flash_used, flash_total, flash_pct as f64);
            }

            // Packets
            if let Some(packets) = json.get("packets") {
                println!("\n📡 Packets:");
                println!("  RX:     {}", packets.get("rx").and_then(|v| v.as_u64()).unwrap_or(0));
                println!("  TX:     {}", packets.get("tx").and_then(|v| v.as_u64()).unwrap_or(0));
                println!("  FWD:    {}", packets.get("fwd").and_then(|v| v.as_u64()).unwrap_or(0));
                println!("  DROP:   {}", packets.get("dropped").and_then(|v| v.as_u64()).unwrap_or(0));
                println!("  DUP:    {}", packets.get("duplicates").and_then(|v| v.as_u64()).unwrap_or(0));
            }

            // Neighbors
            if let Some(neighbors) = json.get("neighbors") {
                let total = neighbors.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                let clients = neighbors.get("clients").and_then(|v| v.as_u64()).unwrap_or(0);
                let repeaters = neighbors.get("repeaters").and_then(|v| v.as_u64()).unwrap_or(0);
                let rooms = neighbors.get("rooms").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("\n🔗 Neighbors: {}", total);
                if total > 0 {
                    println!("  Clients:   {}", clients);
                    println!("  Repeaters: {}", repeaters);
                    println!("  Rooms:     {}", rooms);
                }
            }

            // Radio
            if let Some(radio) = json.get("radio") {
                println!("\n📻 Radio:");
                if let Some(freq) = radio.get("freq_mhz").and_then(|v| v.as_f64()) {
                    println!("  Freq:   {:.2} MHz", freq);
                }
                if let Some(bw) = radio.get("bandwidth_khz").and_then(|v| v.as_f64()) {
                    println!("  BW:     {:.1} kHz", bw);
                }
                if let Some(sf) = radio.get("spreading_factor").and_then(|v| v.as_u64()) {
                    println!("  SF:     {}", sf);
                }
                if let Some(power) = radio.get("tx_power_dbm").and_then(|v| v.as_i64()) {
                    println!("  Power:  {} dBm", power);
                }
            }

            // Power
            if let Some(power) = json.get("power") {
                println!("\n🔋 Power:");
                let pct = power.get("battery_pct").and_then(|v| v.as_u64()).unwrap_or(0);
                let mv = power.get("battery_mv").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("  Battery:  {}% ({:.2}V)", pct, mv as f64 / 1000.0);

                let usb = power.get("usb_power").and_then(|v| v.as_bool()).unwrap_or(false);
                let charging = power.get("charging").and_then(|v| v.as_bool()).unwrap_or(false);
                let sleep = power.get("sleep_enabled").and_then(|v| v.as_bool()).unwrap_or(false);

                println!("  USB:      {}", if usb { "Yes" } else { "No" });
                println!("  Charging: {}", if charging { "Yes" } else { "No" });
                println!("  Sleep:    {}", if sleep { "Enabled" } else { "Disabled" });
            }

            // Features
            if let Some(features) = json.get("features") {
                println!("\n⚡ Optimizations:");
                if features.get("hw_aes").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("  ✓ Hardware AES-128");
                } else {
                    println!("  ✗ Hardware AES-128 (software)");
                }
                if features.get("hw_sha256").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("  ✓ Hardware SHA-256");
                } else {
                    println!("  ✗ Hardware SHA-256 (software)");
                }
                if features.get("priority_scheduling").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("  ✓ Priority Scheduling");
                }
                if features.get("airtime_budget").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("  ✓ Airtime Budget (33%)");
                }
                if let Some(queue_size) = features.get("tx_queue_size").and_then(|v| v.as_u64()) {
                    println!("  ✓ TX Queue ({} slots)", queue_size);
                }
                if features.get("secret_caching").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("  ✓ Shared Secret Caching");
                }
            }

            // Firmware
            if let Some(fw) = json.get("firmware") {
                println!("\n🔧 Firmware:");
                if let Some(ver) = fw.get("version").and_then(|v| v.as_str()) {
                    println!("  Version: {}", ver);
                }
                if let Some(mode) = fw.get("mode").and_then(|v| v.as_str()) {
                    println!("  Mode:    {}", mode);
                }
                if let Some(uptime) = fw.get("uptime_secs").and_then(|v| v.as_u64()) {
                    let hours = uptime / 3600;
                    let mins = (uptime % 3600) / 60;
                    let secs = uptime % 60;
                    if hours > 0 {
                        println!("  Uptime:  {}h {}m {}s", hours, mins, secs);
                    } else if mins > 0 {
                        println!("  Uptime:  {}m {}s", mins, secs);
                    } else {
                        println!("  Uptime:  {}s", secs);
                    }
                }
            }

            // Temperature
            if let Some(temp) = json.get("temperature") {
                if let Some(cpu_temp) = temp.get("cpu_c").and_then(|v| v.as_f64()) {
                    println!("\n🌡️  CPU Temp: {:.1}°C", cpu_temp);
                }
            }

            println!();
        }
        Response::Error(e) => bail!("Device error: {}", e),
        Response::Ok(data) => {
            eprintln!("DEBUG: Got OK response: {:?}", data);
            bail!("Unexpected OK response to STATS (expected JSON)")
        }
    }

    Ok(())
}

/// Show neighbor table
pub async fn cmd_neighbors(port: &str, baud: u32) -> Result<()> {
    let mut dev = connect_with_auth(port, baud, None).await?;
    let neighbors = dev.get_neighbors().await?;

    if neighbors.is_empty() {
        println!("No neighbors discovered yet.");
        return Ok(());
    }

    println!("Neighbor Table ({} nodes):\n", neighbors.len());
    println!("  {:8} {:16} {:6} {:6} {:12} {:8}", "Hash", "Name", "RSSI", "SNR", "Firmware", "Last Seen");
    println!("  {:-<8} {:-<16} {:-<6} {:-<6} {:-<12} {:-<8}", "", "", "", "", "", "");

    for n in neighbors {
        let name = n.name.unwrap_or_else(|| "?".into());
        let firmware = n.firmware.unwrap_or_else(|| "unknown".into());
        println!("  0x{:02x}     {:16} {:6} {:6} {:12} {}s ago",
            n.node_hash, name, n.rssi, n.snr, firmware, n.last_seen_secs);
    }

    Ok(())
}

/// Show telemetry data
pub async fn cmd_telemetry(port: &str, baud: u32, watch: bool) -> Result<()> {
    let serial_port = SerialPort::open(port, baud).await?;
    let mut proto = Protocol::new(serial_port);

    loop {
        // Request telemetry from device
        let telem = proto.get_telemetry().await?;

        // Clear screen in watch mode
        if watch {
            print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen
        }

        println!("Device Telemetry");
        println!("================\n");

        if let Some(dev) = telem.device {
            println!("Battery:     {}% ({:.2}V)", dev.battery_percent, dev.voltage());
            println!("Charging:    {}", if dev.charging { "Yes" } else { "No" });
            println!("USB Power:   {}", if dev.usb_power { "Yes" } else { "No" });
            println!("Uptime:      {}s", dev.uptime_secs);
            println!("Free Heap:   {} bytes", dev.free_heap);
            println!("CPU Temp:    {:.1}°C", dev.cpu_temp_celsius());
            println!();
        }

        if let Some(env) = telem.environment {
            println!("Temperature: {:.1}°C", env.temperature_celsius());
            println!("Humidity:    {:.1}%", env.humidity_percent());
            println!("Pressure:    {:.1} hPa", env.pressure_hpa());
            if env.air_quality > 0 {
                println!("Air Quality: {}", env.air_quality);
            }
            println!();
        }

        if let Some(loc) = telem.location {
            if loc.has_fix() {
                println!("Location:    {:.6}, {:.6}", loc.latitude(), loc.longitude());
                println!("Altitude:    {:.1}m", loc.altitude_meters());
                println!("Speed:       {:.1} m/s", loc.speed_m_s());
                println!("Heading:     {:.0}°", loc.heading_degrees());
                println!("Satellites:  {}", loc.satellites);
            } else {
                println!("GPS:         No fix");
            }
            println!();
        }

        if !watch {
            break;
        }

        // Wait 1 second before next update
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}
