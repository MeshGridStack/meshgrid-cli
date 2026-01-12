//! Network and radio commands

use anyhow::Result;
use crate::device::Device;

pub async fn cmd_trace(port: &str, baud: u32, target: &str) -> Result<()> {
    let mut dev = Device::connect(port, baud).await?;

    println!("Tracing route to {}...\n", target);

    let trace = dev.trace(target).await?;

    println!("Route: {}", trace.path.join(" -> "));
    println!("Hops: {}", trace.hop_count);
    println!("RTT: {} ms", trace.rtt_ms);

    Ok(())
}

pub async fn cmd_advert(port: &str, baud: u32, local_only: bool, flood_only: bool) -> Result<()> {
    let mut dev = Device::connect(port, baud).await?;

    // Determine which advertisements to send
    let send_local = !flood_only; // Send local unless flood-only is specified
    let send_flood = !local_only; // Send flood unless local-only is specified

    // If neither flag is set, send both (default behavior)
    let send_both = !local_only && !flood_only;

    if send_local || send_both {
        dev.send_advert_local().await?;
        println!("Local advertisement (ROUTE_DIRECT) sent");
    }

    if send_flood || send_both {
        // Small delay between commands to ensure first packet completes
        if send_both {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        dev.send_advert_flood().await?;
        println!("Flood advertisement (ROUTE_FLOOD) sent");
    }

    Ok(())
}

pub async fn cmd_raw(port: &str, baud: u32, hex_data: &str) -> Result<()> {
    let mut dev = Device::connect(port, baud).await?;

    let packet = hex::decode(hex_data.trim())
        .map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?;

    println!("Sending {} bytes: {}", packet.len(), hex_data);
    dev.send_packet(&packet).await?;
    println!("Sent!");

    Ok(())
}

pub async fn cmd_recv(port: &str, baud: u32, timeout_secs: u64) -> Result<()> {
    let dev = Device::connect(port, baud).await?;

    println!("Waiting for packets ({}s timeout, Ctrl+C to stop)...\n", timeout_secs);

    let timeout = std::time::Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    // Get underlying protocol for raw packet access
    let mut proto = dev.into_protocol();

    while start.elapsed() < timeout {
        if let Some(packet) = proto.recv_packet(std::time::Duration::from_millis(100)).await? {
            print_packet(&packet);
        }
    }

    println!("Timeout reached.");
    Ok(())
}

fn print_packet(packet: &[u8]) {
    let timestamp = chrono::Local::now().format("%H:%M:%S");
    println!("[{}] Received {} bytes:", timestamp, packet.len());
    println!("  Hex: {}", hex::encode(packet));

    // Try to decode as text if it looks like ASCII
    if packet.iter().all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace()) {
        if let Ok(text) = std::str::from_utf8(packet) {
            println!("  Text: \"{}\"", text);
        }
    }
    println!();
}
