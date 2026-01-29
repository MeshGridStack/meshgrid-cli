//! Messaging commands

use super::connect_with_auth;
use crate::cli::{ChannelsAction, MessagesAction};
use crate::protocol::Response;
use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose};

/// Send a message (broadcast, direct, or channel)
pub async fn cmd_send(
    port: &str,
    baud: u32,
    pin: Option<&str>,
    to: Option<&str>,
    channel: Option<&str>,
    message: &str,
) -> Result<()> {
    let dev = connect_with_auth(port, baud, pin).await?;
    let mut proto = dev.into_protocol();

    if let Some(ch) = channel {
        // Send to channel
        println!("Sending to channel {ch}: {message}");
        let cmd = format!("CHANNEL SEND {ch} {message}");
        match proto.command(&cmd).await? {
            Response::Ok(_) => {
                println!("Sent!");
            }
            Response::Error(e) => bail!("Device error: {e}"),
            Response::Json(_) => bail!("Unexpected response to CHANNEL SEND"),
        }
    } else if let Some(dest) = to {
        // Send direct message
        println!("Sending to {dest}: {message}");
        let cmd = format!("SEND {dest} {message}");
        match proto.command(&cmd).await? {
            Response::Ok(msg) => {
                if let Some(m) = msg {
                    println!("Sent! ({m})");
                } else {
                    println!("Sent!");
                }
            }
            Response::Error(e) => bail!("Device error: {e}"),
            Response::Json(_) => bail!("Unexpected response to SEND"),
        }
    } else {
        // Broadcast to public channel
        println!("Broadcasting: {message}");
        let cmd = format!("SEND {message}");
        match proto.command(&cmd).await? {
            Response::Ok(_) => {
                println!("Sent!");
            }
            Response::Error(e) => bail!("Device error: {e}"),
            Response::Json(_) => bail!("Unexpected response to SEND"),
        }
    }

    Ok(())
}

/// Manage inbox messages
pub async fn cmd_messages(
    port: &str,
    baud: u32,
    pin: Option<&str>,
    action: Option<MessagesAction>,
) -> Result<()> {
    use chrono::{Local, TimeZone};
    let dev = connect_with_auth(port, baud, pin).await?;
    let mut proto = dev.into_protocol();

    let action = action.unwrap_or(MessagesAction::Show);

    match action {
        MessagesAction::Show => {
            match proto.command("MESSAGES").await? {
                Response::Json(json) => {
                    let total = json
                        .get("total")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);

                    if total == 0 {
                        println!("No messages in inbox");
                    } else if let Some(messages) = json.get("messages").and_then(|m| m.as_array()) {
                        println!("Inbox ({total} messages):\n");

                        for msg in messages {
                            let _from_hash =
                                msg.get("from_hash").and_then(|h| h.as_str()).unwrap_or("?");
                            let from_name =
                                msg.get("from_name").and_then(|n| n.as_str()).unwrap_or("?");
                            let channel =
                                msg.get("channel").and_then(|c| c.as_str()).unwrap_or("?");
                            let protocol =
                                msg.get("protocol").and_then(|p| p.as_str()).unwrap_or("v0");
                            let decrypted = msg
                                .get("decrypted")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(false);
                            let text = msg.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            let timestamp = msg
                                .get("timestamp")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0);

                            let channel_str = match channel {
                                "direct" => "DM".to_string(),
                                "public" => "Public".to_string(),
                                ch => format!("CH:{ch}"),
                            };

                            let lock = if decrypted { " " } else { "🔒" };

                            // Format timestamp as datetime
                            let timestamp_i64 = i64::try_from(timestamp).unwrap_or(0);
                            let datetime =
                                Local.timestamp_opt(timestamp_i64, 0).single().map_or_else(
                                    || format!("invalid-ts:{timestamp}"),
                                    |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                                );

                            println!(
                                "  [{datetime}] {lock} from {from_name} ({channel_str}/{protocol}): {text}"
                            );
                        }
                    }
                }
                Response::Error(e) => bail!("Device error: {e}"),
                Response::Ok(_) => bail!("Unexpected OK response to MESSAGES"),
            }
        }
        MessagesAction::Clear => match proto.command("MESSAGES CLEAR").await? {
            Response::Ok(msg) => {
                println!("{}", msg.unwrap_or_else(|| "Messages cleared".to_string()));
            }
            Response::Error(e) => bail!("Device error: {e}"),
            Response::Json(_) => bail!("Unexpected response to MESSAGES CLEAR"),
        },
    }

    Ok(())
}

/// Manage channels
pub async fn cmd_channels(
    port: &str,
    baud: u32,
    pin: Option<&str>,
    action: Option<ChannelsAction>,
) -> Result<()> {
    let dev = connect_with_auth(port, baud, pin).await?;
    let mut proto = dev.into_protocol();

    let action = action.unwrap_or(ChannelsAction::List);

    match action {
        ChannelsAction::List => match proto.command("CHANNELS").await? {
            Response::Json(json) => {
                let channels = json.get("channels").and_then(|c| c.as_array());
                let total = json
                    .get("total")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);

                println!("Channels ({total}):\n");

                if let Some(channels) = channels {
                    for channel in channels {
                        let name = channel.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                        let hash = channel.get("hash").and_then(|h| h.as_str()).unwrap_or("?");
                        let builtin = channel
                            .get("builtin")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);

                        let tag = if builtin { "[builtin]" } else { "[custom]" };
                        println!("  {hash} - {name} {tag}");
                    }
                }
            }
            Response::Error(e) => bail!("Device error: {e}"),
            Response::Ok(_) => bail!("Unexpected OK response to CHANNELS"),
        },
        ChannelsAction::Add { name, psk } => {
            // Auto-generate PSK for hashtag channels (public channels)
            let psk_to_use = if name.starts_with('#') {
                // Calculate SHA256(channel_name) for public hashtag channels
                let mut hasher = Sha256::new();
                hasher.update(name.as_bytes());
                let hash = hasher.finalize();
                let encoded = general_purpose::STANDARD.encode(hash);
                println!("Auto-generated PSK for public hashtag channel '{}'", name);
                encoded
            } else {
                // For non-hashtag channels, PSK is required
                match psk {
                    Some(p) => p,
                    None => bail!("PSK is required for non-hashtag channels. Use hashtag (#) prefix for public channels."),
                }
            };

            let cmd = format!("CHANNEL JOIN {name} {psk_to_use}");
            match proto.command(&cmd).await? {
                Response::Ok(msg) => {
                    println!("{}", msg.unwrap_or_else(|| "Channel added".to_string()));
                }
                Response::Error(e) => bail!("Device error: {e}"),
                Response::Json(_) => bail!("Unexpected response to CHANNEL JOIN"),
            }
        }
        ChannelsAction::Remove { name } => {
            let cmd = format!("CHANNEL LEAVE {name}");
            match proto.command(&cmd).await? {
                Response::Ok(msg) => {
                    println!("{}", msg.unwrap_or_else(|| "Channel removed".to_string()));
                }
                Response::Error(e) => bail!("Device error: {e}"),
                Response::Json(_) => bail!("Unexpected response to CHANNEL LEAVE"),
            }
        }
    }

    Ok(())
}

/// Rotate device identity (generate new keypair)
pub async fn cmd_rotate_identity(port: &str, baud: u32, pin: Option<&str>) -> Result<()> {
    let dev = connect_with_auth(port, baud, pin).await?;
    let mut proto = dev.into_protocol();

    println!("WARNING: This will generate a new keypair and clear all encrypted data.");
    println!("         Old messages and neighbor secrets will be deleted.");
    println!("         Other nodes will need to re-discover your new identity.\n");

    match proto.command("IDENTITY ROTATE").await? {
        Response::Ok(msg) => {
            println!(
                "{}",
                msg.unwrap_or_else(|| "Identity rotated, device rebooting...".to_string())
            );
            Ok(())
        }
        Response::Error(e) => bail!("Device error: {e}"),
        Response::Json(_) => bail!("Unexpected response to IDENTITY ROTATE"),
    }
}
