//! meshgrid-cli - Command line interface for meshgrid mesh networking.
//!
//! Connects to meshgrid/MeshCore devices over USB serial and provides
//! tools for sending messages, monitoring the mesh, and device management.

mod serial;
mod protocol;
mod device;
mod ui;
mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import CLI definitions and command functions
use cli::{Cli, Commands};
use commands::{
    // Info commands
    cmd_info, cmd_stats, cmd_neighbors, cmd_telemetry,
    // Messaging commands
    cmd_send, cmd_messages, cmd_channels, cmd_rotate_identity,
    // Config commands
    cmd_config,
    // Network commands
    cmd_trace, cmd_advert, cmd_raw, cmd_recv,
    // System commands
    cmd_reboot, cmd_ui, cmd_mode, cmd_time, cmd_flash, cmd_debug,
    // Utility commands
    cmd_list_ports, require_port,
};

#[tokio::main]
async fn main() -> Result<()> {
    // When running without a TTY (e.g., subprocess, cron, systemd),
    // stdin might block tokio's reactor. Set it to non-blocking mode.
    #[cfg(unix)]
    unsafe {
        use std::os::unix::io::AsRawFd;
        use std::io::IsTerminal;

        // Only modify stdin if it's NOT a terminal
        if !std::io::stdin().is_terminal() {
            let stdin_fd = std::io::stdin().as_raw_fd();
            let flags = libc::fcntl(stdin_fd, libc::F_GETFL, 0);
            if flags >= 0 && (flags & libc::O_NONBLOCK) == 0 {
                // stdin is blocking - make it non-blocking to prevent reactor stalls
                let _ = libc::fcntl(stdin_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }
    }

    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(filter))
        .init();

    match cli.command {
        Commands::Ports => {
            cmd_list_ports()?;
        }
        Commands::Info => {
            let port = require_port(&cli.port)?;
            cmd_info(&port, cli.baud, cli.pin.as_deref()).await?;
        }
        Commands::Send { to, channel, message } => {
            let port = require_port(&cli.port)?;
            cmd_send(&port, cli.baud, cli.pin.as_deref(), to.as_deref(), channel.as_deref(), &message).await?;
        }
        Commands::Ui => {
            let port = require_port(&cli.port)?;
            cmd_ui(&port, cli.baud).await?;
        }
        Commands::Config { action } => {
            let port = require_port(&cli.port)?;
            cmd_config(&port, cli.baud, action).await?;
        }
        Commands::Neighbors => {
            let port = require_port(&cli.port)?;
            cmd_neighbors(&port, cli.baud, cli.pin.as_deref()).await?;
        }
        Commands::Trace { target } => {
            let port = require_port(&cli.port)?;
            cmd_trace(&port, cli.baud, cli.pin.as_deref(), &target).await?;
        }
        Commands::Reboot => {
            let port = require_port(&cli.port)?;
            cmd_reboot(&port, cli.baud).await?;
        }
        Commands::Raw { hex } => {
            let port = require_port(&cli.port)?;
            cmd_raw(&port, cli.baud, &hex).await?;
        }
        Commands::Recv { timeout } => {
            let port = require_port(&cli.port)?;
            cmd_recv(&port, cli.baud, timeout).await?;
        }
        Commands::Telemetry { watch } => {
            let port = require_port(&cli.port)?;
            cmd_telemetry(&port, cli.baud, watch).await?;
        }
        Commands::Stats => {
            let port = require_port(&cli.port)?;
            cmd_stats(&port, cli.baud, cli.pin.as_deref()).await?;
        }
        Commands::Mode { mode } => {
            let port = require_port(&cli.port)?;
            let mode_str = match mode {
                cli::DeviceMode::Client => "client",
                cli::DeviceMode::Repeater => "repeater",
            };
            cmd_mode(&port, cli.baud, cli.pin.as_deref(), mode_str).await?;
        }
        Commands::Time { action } => {
            let port = require_port(&cli.port)?;
            cmd_time(&port, cli.baud, cli.pin.as_deref(), action).await?;
        }
        Commands::Messages { action } => {
            let port = require_port(&cli.port)?;
            cmd_messages(&port, cli.baud, cli.pin.as_deref(), action).await?;
        }
        Commands::Channels { action } => {
            let port = require_port(&cli.port)?;
            cmd_channels(&port, cli.baud, cli.pin.as_deref(), action).await?;
        }
        Commands::Flash { board, monitor, local, detect } => {
            let port = cli.port.clone();
            cmd_flash(board, port.as_deref(), monitor, local.as_deref(), detect).await?;
        }
        Commands::Advert { local, flood } => {
            let port = require_port(&cli.port)?;
            cmd_advert(&port, cli.baud, cli.pin.as_deref(), local, flood).await?;
        }
        Commands::RotateIdentity => {
            let port = require_port(&cli.port)?;
            cmd_rotate_identity(&port, cli.baud, cli.pin.as_deref()).await?;
        }
        Commands::Debug { output, timeout } => {
            let port = require_port(&cli.port)?;
            cmd_debug(&port, cli.baud, output, timeout).await?;
        }
        Commands::Stdin => {
            // TODO: Implement stdin command processing
            eprintln!("Stdin command not yet implemented");
        }
    }

    Ok(())
}
