//! Messaging commands

use anyhow::{Result, bail};
use super::connect_with_auth;
use crate::serial::SerialPort;
use crate::protocol::{Protocol, Response};
use crate::cli::{MessagesAction, ChannelsAction};

/// Send a message (broadcast or direct)
pub async fn cmd_send(port: &str, baud: u32, pin: Option<&str>, to: Option<&str>, message: &str) -> Result<()> {
    let mut dev = connect_with_auth(port, baud, pin).await?;

    if let Some(dest) = to {
        println!("Sending to {}: {}", dest, message);
        dev.send_direct(dest, message).await?;
    } else {
        println!("Broadcasting: {}", message);
        dev.send_broadcast(message).await?;
    }

    println!("Sent!");
    Ok(())
}

/// Monitor mesh traffic in real-time
pub async fn cmd_monitor(port: &str, baud: u32, pin: Option<&str>) -> Result<()> {
    let mut dev = connect_with_auth(port, baud, pin).await?;

    println!("Monitoring mesh traffic (Ctrl+C to stop)...\n");

    dev.monitor(|event| {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        match event {
            crate::device::MeshEvent::Message { from, to, text, rssi, snr } => {
                let dest = to.as_deref().unwrap_or("broadcast");
                println!("[{}] MSG {} -> {}: \"{}\" (RSSI:{} SNR:{})",
                    timestamp, from, dest, text, rssi, snr);
            }
            crate::device::MeshEvent::Advertisement { name, node_hash, rssi } => {
                let name = name.as_deref().unwrap_or("?");
                println!("[{}] ADV 0x{:02x} \"{}\" (RSSI:{})",
                    timestamp, node_hash, name, rssi);
            }
            crate::device::MeshEvent::Ack { from } => {
                println!("[{}] ACK from {}", timestamp, from);
            }
            crate::device::MeshEvent::Error { message } => {
                eprintln!("[{}] ERR: {}", timestamp, message);
            }
        }
    }).await?;

    Ok(())
}

/// Manage inbox messages
pub async fn cmd_messages(port: &str, baud: u32, action: Option<MessagesAction>) -> Result<()> {
    let serial_port = SerialPort::open(port, baud).await?;
    let mut proto = Protocol::new(serial_port);

    let action = action.unwrap_or(MessagesAction::Show);

    match action {
        MessagesAction::Show => {
            match proto.command("MESSAGES").await? {
                Response::Json(json) => {
                    let total = json.get("total").and_then(|t| t.as_u64()).unwrap_or(0);

                    if total == 0 {
                        println!("No messages in inbox");
                    } else if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
                        println!("Inbox ({} messages):\n", total);

                        for msg in messages {
                            let _from_hash = msg.get("from_hash").and_then(|h| h.as_str()).unwrap_or("?");
                            let from_name = msg.get("from_name").and_then(|n| n.as_str()).unwrap_or("?");
                            let channel = msg.get("channel").and_then(|c| c.as_str()).unwrap_or("?");
                            let decrypted = msg.get("decrypted").and_then(|d| d.as_bool()).unwrap_or(false);
                            let text = msg.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            let timestamp = msg.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0);

                            let channel_str = match channel {
                                "direct" => "DM".to_string(),
                                "public" => "Public".to_string(),
                                ch => format!("CH:{}", ch),
                            };

                            let lock = if decrypted { " " } else { "🔒" };

                            println!("  [{}s] {} from {} ({}): {}",
                                     timestamp / 1000,
                                     lock,
                                     from_name,
                                     channel_str,
                                     text);
                        }
                    }
                }
                Response::Error(e) => bail!("Device error: {}", e),
                _ => bail!("Unexpected response to MESSAGES"),
            }
        }
        MessagesAction::Clear => {
            match proto.command("MESSAGES CLEAR").await? {
                Response::Ok(msg) => {
                    println!("{}", msg.unwrap_or_else(|| "Messages cleared".to_string()));
                }
                Response::Error(e) => bail!("Device error: {}", e),
                _ => bail!("Unexpected response to MESSAGES CLEAR"),
            }
        }
    }

    Ok(())
}

/// Manage channels
pub async fn cmd_channels(port: &str, baud: u32, action: Option<ChannelsAction>) -> Result<()> {
    let serial_port = SerialPort::open(port, baud).await?;
    let mut proto = Protocol::new(serial_port);

    let _action = action.unwrap_or(ChannelsAction::List);

    match proto.command("CHANNELS").await? {
        Response::Json(json) => {
            let channels = json.get("channels").and_then(|c| c.as_array());
            let total = json.get("total").and_then(|t| t.as_u64()).unwrap_or(0);

            println!("Channels ({}):\n", total);

            if let Some(channels) = channels {
                for channel in channels {
                    let name = channel.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let hash = channel.get("hash").and_then(|h| h.as_str()).unwrap_or("?");
                    let builtin = channel.get("builtin").and_then(|b| b.as_bool()).unwrap_or(false);

                    let tag = if builtin { "[builtin]" } else { "[custom]" };
                    println!("  {} - {} {}", hash, name, tag);
                }
            }
        }
        Response::Error(e) => bail!("Device error: {}", e),
        _ => bail!("Unexpected response to CHANNELS"),
    }

    Ok(())
}

/// Rotate device identity (generate new keypair)
pub async fn cmd_rotate_identity(port: &str, baud: u32) -> Result<()> {
    let serial_port = SerialPort::open(port, baud).await?;
    let mut proto = Protocol::new(serial_port);

    println!("WARNING: This will generate a new keypair and clear all encrypted data.");
    println!("         Old messages and neighbor secrets will be deleted.");
    println!("         Other nodes will need to re-discover your new identity.\n");

    match proto.command("IDENTITY ROTATE").await? {
        Response::Ok(msg) => {
            println!("{}", msg.unwrap_or_else(|| "Identity rotated, device rebooting...".to_string()));
            Ok(())
        }
        Response::Error(e) => bail!("Device error: {}", e),
        _ => bail!("Unexpected response to IDENTITY ROTATE"),
    }
}
