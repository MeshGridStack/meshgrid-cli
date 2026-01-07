//! meshgrid-cli - Command line interface for meshgrid mesh networking.
//!
//! Connects to meshgrid/MeshCore devices over USB serial and provides
//! tools for sending messages, monitoring the mesh, and device management.

mod serial;
mod protocol;
mod device;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "meshgrid")]
#[command(author, version, about = "Meshgrid mesh networking CLI", long_about = None)]
struct Cli {
    /// Serial port device (e.g., /dev/ttyUSB0, /dev/ttyACM0)
    #[arg(short, long, global = true)]
    port: Option<String>,

    /// Baud rate
    #[arg(short, long, default_value = "115200", global = true)]
    baud: u32,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available serial ports
    Ports,

    /// Connect to a device and show info
    Info,

    /// Send a text message
    Send {
        /// Destination node (name or hash)
        #[arg(short, long)]
        to: Option<String>,

        /// Message text
        message: String,
    },

    /// Monitor mesh traffic in real-time
    Monitor,

    /// Interactive terminal UI
    Ui,

    /// Get/set device configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show neighbor table
    Neighbors,

    /// Send a trace packet to map routes
    Trace {
        /// Target node (name or hash)
        target: String,
    },

    /// Reboot the device
    Reboot,

    /// Send a raw hex packet (for debugging)
    Raw {
        /// Hex-encoded packet data
        hex: String,
    },

    /// Receive raw packets (for debugging)
    Recv {
        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Get device telemetry (battery, sensors)
    Telemetry {
        /// Watch mode - continuously update
        #[arg(short, long)]
        watch: bool,
    },

    /// Flash firmware to a device
    Flash {
        /// Board type to flash (auto-detect if not specified)
        #[arg(value_enum)]
        board: Option<BoardType>,

        /// Monitor serial output after flashing
        #[arg(short, long)]
        monitor: bool,

        /// Path to local meshgrid-firmware (for development)
        #[arg(long)]
        local: Option<String>,

        /// List detected devices without flashing
        #[arg(long)]
        detect: bool,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum BoardType {
    // =========== Heltec ESP32-S3 ===========
    /// Heltec LoRa32 V3 (ESP32-S3 + SX1262)
    HeltecV3,
    /// Heltec LoRa32 V4 (ESP32-S3 + SX1262)
    HeltecV4,
    /// Heltec Wireless Stick Lite V3 (ESP32-S3 + SX1262)
    HeltecWirelessStickLiteV3,
    /// Heltec Wireless Tracker V1.1 (ESP32-S3 + SX1262 + GPS)
    HeltecWirelessTracker,
    /// Heltec Wireless Paper (ESP32-S3 + SX1262 + E-Ink)
    HeltecWirelessPaper,
    /// Heltec Vision Master T190 (ESP32-S3 + SX1262 + TFT)
    HeltecVisionMasterT190,
    /// Heltec Vision Master E213 (ESP32-S3 + SX1262 + E-Ink)
    HeltecVisionMasterE213,
    /// Heltec Vision Master E290 (ESP32-S3 + SX1262 + E-Ink)
    HeltecVisionMasterE290,
    /// Heltec HT62 (ESP32-C3 + SX1262)
    HeltecHt62,
    /// Heltec Mesh Node T114 (nRF52840 + SX1262 + TFT)
    HeltecMeshNodeT114,
    /// Heltec MeshPocket (nRF52840 + SX1262)
    HeltecMeshPocket,

    // =========== LilyGo ESP32-S3 ===========
    /// LilyGo T-LoRa T3-S3 (ESP32-S3 + SX1262)
    LilygoT3s3,
    /// LilyGo T-LoRa T3-S3 E-Ink (ESP32-S3 + SX1262 + E-Ink)
    LilygoT3s3Eink,
    /// LilyGo T-Beam Supreme (ESP32-S3 + SX1262 + GPS)
    LilygoTbeamSupreme,
    /// LilyGo T-Deck (ESP32-S3 + SX1262 + keyboard)
    LilygoTdeck,
    /// LilyGo T-Deck Pro (ESP32-S3 + LR1121 + keyboard + GPS)
    LilygoTdeckPro,
    /// LilyGo T-LoRa Pager (ESP32-S3 + SX1262 + keyboard)
    LilygoTloraPager,
    /// LilyGo T-Watch S3 (ESP32-S3 + SX1262)
    LilygoTwatchS3,

    // =========== LilyGo ESP32 ===========
    /// LilyGo T-Beam (ESP32 + SX1276 + GPS)
    LilygoTbeam,
    /// LilyGo T-LoRa V2.1-1.6 (ESP32 + SX1276)
    LilygoTloraV2116,
    /// LilyGo T-LoRa V2.1-1.8 2.4GHz (ESP32 + SX1280)
    LilygoTloraV2118,

    // =========== LilyGo nRF52840 ===========
    /// LilyGo T-Echo (nRF52840 + SX1262 + E-Ink + GPS)
    LilygoTecho,

    // =========== RAK nRF52840 ===========
    /// RAK WisBlock 4631 (nRF52840 + SX1262)
    Rak4631,
    /// RAK WisMesh Repeater (nRF52840 + SX1262)
    RakWismeshRepeater,
    /// RAK WisMesh Tap (nRF52840 + SX1262)
    RakWismeshTap,
    /// RAK WisMesh Tag (nRF52840 + SX1262)
    RakWismeshTag,
    /// RAK3401 1W (nRF52840 + SX1262)
    Rak34011w,

    // =========== RAK ESP32 ===========
    /// RAK WisBlock 11200 (ESP32 + SX1262)
    Rak11200,
    /// RAK3312 (ESP32-S3 + SX1262)
    Rak3312,

    // =========== RAK RP2040 ===========
    /// RAK WisBlock 11310 (RP2040 + SX1262)
    Rak11310,

    // =========== Seeed nRF52840 ===========
    /// Seeed Card Tracker T1000-E (nRF52840 + LR1110 + GPS)
    SeeedTrackerT1000e,
    /// Seeed Xiao nRF52840 Kit (nRF52840 + SX1262)
    SeeedXiaoNrf52840,
    /// Seeed SenseCAP Solar Node (nRF52840 + LR1110)
    SeeedSensecapSolar,
    /// Seeed Wio Tracker L1 (nRF52840 + LR1110)
    SeeedWioTrackerL1,
    /// Seeed Wio Tracker L1 E-Ink (nRF52840 + LR1110 + E-Ink)
    SeeedWioTrackerL1Eink,
    /// Seeed Wio WM1110 Tracker (nRF52840 + LR1110)
    SeeedWioWm1110,

    // =========== Seeed ESP32-S3 ===========
    /// Seeed SenseCAP Indicator (ESP32-S3 + SX1262 + TFT)
    SeeedSensecapIndicator,
    /// Seeed Xiao ESP32-S3 (ESP32-S3)
    SeeedXiaoEsp32s3,

    // =========== Elecrow ===========
    /// ThinkNode M1 (nRF52840 + SX1262)
    ThinknodeM1,
    /// ThinkNode M2 (ESP32-S3 + SX1262)
    ThinknodeM2,
    /// ThinkNode M3 (nRF52840 + SX1262)
    ThinknodeM3,
    /// ThinkNode M5 (ESP32-S3 + SX1262)
    ThinknodeM5,
    /// Crowpanel Adv 2.4/2.8 TFT (ESP32-S3 + SX1262)
    Crowpanel24tft,
    /// Crowpanel Adv 3.5 TFT (ESP32-S3 + SX1262)
    Crowpanel35tft,
    /// Crowpanel Adv 4.3/5.0/7.0 TFT (ESP32-S3 + SX1262)
    Crowpanel43tft,

    // =========== B&Q Consulting ===========
    /// Station G2 (ESP32-S3 + SX1262)
    StationG2,
    /// Station G1 (ESP32 + SX1276)
    StationG1,
    /// Nano G1 (ESP32 + SX1276)
    NanoG1,
    /// Nano G1 Explorer (ESP32 + SX1276)
    NanoG1Explorer,
    /// Nano G2 Ultra (nRF52840 + SX1262)
    NanoG2Ultra,

    // =========== M5Stack ===========
    /// M5 Stack (ESP32)
    M5stack,
    /// M5Stack Unit C6L (ESP32-C6 + SX1262)
    M5stackUnitC6l,

    // =========== Other Vendors ===========
    /// muzi BASE (nRF52840 + SX1262)
    MuziBase,
    /// muzi R1 Neo (nRF52840 + SX1262)
    MuziR1Neo,
    /// NomadStar Meteor Pro (nRF52840 + SX1262 + GPS)
    NomadstarMeteorPro,
    /// Canary One (nRF52840 + SX1262)
    CanaryOne,
    /// RadioMaster 900 Bandit Nano (ESP32 + SX1276)
    Radiomaster900Bandit,
    /// EByte EoRa-S3 (ESP32-S3 + SX1262)
    EbyteEoraS3,
    /// TrackSenger small TFT (ESP32-S3 + SX1262)
    TracksengerSmall,
    /// TrackSenger big OLED (ESP32-S3 + SX1262)
    TracksengerBig,
    /// Pi Computer S3 (ESP32-S3 + SX1262)
    PiComputerS3,
    /// unPhone (ESP32-S3)
    Unphone,

    // =========== RP2040 ===========
    /// Waveshare RP2040 LoRa (RP2040 + SX1262)
    Rp2040Lora,
    /// Raspberry Pi Pico (RP2040)
    RpiPico,
    /// Raspberry Pi Pico W (RP2040 + WiFi)
    RpiPicoW,

    // =========== DIY ===========
    /// DIY V1 (ESP32 + SX1276)
    DiyV1,
    /// Hydra (ESP32 + SX1276)
    Hydra,
    /// nRF52 Pro-micro DIY (nRF52840)
    Nrf52PromicroDiy,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set device name
    Name { name: String },
    /// Set LoRa frequency
    Freq { freq_mhz: f32 },
    /// Set transmit power
    Power { dbm: i8 },
    /// Set radio preset (EU, US, US_FAST, LONG_RANGE)
    Preset { preset: String },
}

#[tokio::main]
async fn main() -> Result<()> {
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
            cmd_info(&port, cli.baud).await?;
        }
        Commands::Send { to, message } => {
            let port = require_port(&cli.port)?;
            cmd_send(&port, cli.baud, to.as_deref(), &message).await?;
        }
        Commands::Monitor => {
            let port = require_port(&cli.port)?;
            cmd_monitor(&port, cli.baud).await?;
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
            cmd_neighbors(&port, cli.baud).await?;
        }
        Commands::Trace { target } => {
            let port = require_port(&cli.port)?;
            cmd_trace(&port, cli.baud, &target).await?;
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
        Commands::Flash { board, monitor, local, detect } => {
            let port = cli.port.clone();
            cmd_flash(board, port.as_deref(), monitor, local.as_deref(), detect).await?;
        }
    }

    Ok(())
}

fn require_port(port: &Option<String>) -> Result<String> {
    if let Some(p) = port.clone() {
        return Ok(p);
    }

    // Try auto-detection
    if let Some(detected) = serial::detect_device()? {
        println!("Auto-detected device: {}", detected);
        return Ok(detected);
    }

    anyhow::bail!(
        "No port specified and no device auto-detected.\nUse -p /dev/ttyUSB0 or run 'meshgrid ports' to list available ports"
    )
}

fn cmd_list_ports() -> Result<()> {
    println!("Available serial ports:\n");

    let ports = serialport::available_ports()?;

    if ports.is_empty() {
        println!("  No serial ports found.");
        println!("\n  Make sure your device is connected via USB.");
        return Ok(());
    }

    for port in ports {
        print!("  {}", port.port_name);

        match port.port_type {
            serialport::SerialPortType::UsbPort(info) => {
                if let Some(manufacturer) = &info.manufacturer {
                    print!("  [{}]", manufacturer);
                }
                if let Some(product) = &info.product {
                    print!("  {}", product);
                }
                println!(
                    "  (VID:{:04x} PID:{:04x})",
                    info.vid, info.pid
                );

                // Identify known devices
                match (info.vid, info.pid) {
                    (0x303a, _) => println!("       ^ ESP32-S3 (T3S3, Heltec V3/V4, Station G2)"),
                    (0x10c4, 0xea60) => println!("       ^ Silicon Labs CP210x (common on ESP32)"),
                    (0x1a86, 0x7523) => println!("       ^ CH340 serial (Heltec, some clones)"),
                    (0x239a, _) => println!("       ^ Seeed/Adafruit device"),
                    _ => {}
                }
            }
            serialport::SerialPortType::PciPort => {
                println!("  (PCI)");
            }
            serialport::SerialPortType::BluetoothPort => {
                println!("  (Bluetooth)");
            }
            serialport::SerialPortType::Unknown => {
                println!();
            }
        }
    }

    println!("\nUsage: meshgrid -p /dev/ttyUSB0 info");

    Ok(())
}

async fn cmd_info(port: &str, baud: u32) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;
    let info = dev.get_info().await?;

    println!("Device Information:");
    println!("  Name:       {}", info.name.unwrap_or_else(|| "<unnamed>".into()));
    println!("  Public Key: {}", hex::encode(&info.public_key[..8]));
    println!("  Node Hash:  0x{:02x}", info.node_hash);
    println!("  Firmware:   {}", info.firmware_version.unwrap_or_else(|| "unknown".into()));
    println!("  Frequency:  {:.2} MHz", info.freq_mhz);
    println!("  TX Power:   {} dBm", info.tx_power_dbm);

    Ok(())
}

async fn cmd_send(port: &str, baud: u32, to: Option<&str>, message: &str) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;

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

async fn cmd_monitor(port: &str, baud: u32) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;

    println!("Monitoring mesh traffic (Ctrl+C to stop)...\n");

    dev.monitor(|event| {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        match event {
            device::MeshEvent::Message { from, to, text, rssi, snr } => {
                let dest = to.as_deref().unwrap_or("broadcast");
                println!("[{}] MSG {} -> {}: \"{}\" (RSSI:{} SNR:{})",
                    timestamp, from, dest, text, rssi, snr);
            }
            device::MeshEvent::Advertisement { name, node_hash, rssi } => {
                let name = name.as_deref().unwrap_or("?");
                println!("[{}] ADV 0x{:02x} \"{}\" (RSSI:{})",
                    timestamp, node_hash, name, rssi);
            }
            device::MeshEvent::Ack { from } => {
                println!("[{}] ACK from {}", timestamp, from);
            }
            device::MeshEvent::Error { message } => {
                eprintln!("[{}] ERR: {}", timestamp, message);
            }
        }
    }).await?;

    Ok(())
}

async fn cmd_ui(port: &str, baud: u32) -> Result<()> {
    ui::run(port, baud).await
}

async fn cmd_config(port: &str, baud: u32, action: ConfigAction) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;

    match action {
        ConfigAction::Show => {
            let config = dev.get_config().await?;
            println!("Device Configuration:");
            println!("  Name:      {}", config.name.unwrap_or_else(|| "<unnamed>".into()));
            println!("  Frequency: {:.2} MHz", config.freq_mhz);
            println!("  TX Power:  {} dBm", config.tx_power_dbm);
            println!("  Bandwidth: {} kHz", config.bandwidth_khz);
            println!("  Spreading: SF{}", config.spreading_factor);
        }
        ConfigAction::Name { name } => {
            dev.set_name(&name).await?;
            println!("Name set to: {}", name);
        }
        ConfigAction::Freq { freq_mhz } => {
            dev.set_frequency(freq_mhz).await?;
            println!("Frequency set to: {:.2} MHz", freq_mhz);
        }
        ConfigAction::Power { dbm } => {
            dev.set_power(dbm).await?;
            println!("TX power set to: {} dBm", dbm);
        }
        ConfigAction::Preset { preset } => {
            dev.set_preset(&preset).await?;
            println!("Preset applied: {}", preset);
        }
    }

    Ok(())
}

async fn cmd_neighbors(port: &str, baud: u32) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;
    let neighbors = dev.get_neighbors().await?;

    if neighbors.is_empty() {
        println!("No neighbors discovered yet.");
        return Ok(());
    }

    println!("Neighbor Table ({} nodes):\n", neighbors.len());
    println!("  {:8} {:16} {:6} {:6} {:8}", "Hash", "Name", "RSSI", "SNR", "Last Seen");
    println!("  {:-<8} {:-<16} {:-<6} {:-<6} {:-<8}", "", "", "", "", "");

    for n in neighbors {
        let name = n.name.unwrap_or_else(|| "?".into());
        println!("  0x{:02x}     {:16} {:6} {:6} {}s ago",
            n.node_hash, name, n.rssi, n.snr, n.last_seen_secs);
    }

    Ok(())
}

async fn cmd_trace(port: &str, baud: u32, target: &str) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;

    println!("Tracing route to {}...\n", target);

    let trace = dev.trace(target).await?;

    println!("Route: {}", trace.path.join(" -> "));
    println!("Hops: {}", trace.hop_count);
    println!("RTT: {} ms", trace.rtt_ms);

    Ok(())
}

async fn cmd_reboot(port: &str, baud: u32) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;
    dev.reboot().await?;
    println!("Device rebooting...");
    Ok(())
}

async fn cmd_raw(port: &str, baud: u32, hex_data: &str) -> Result<()> {
    let mut dev = device::Device::connect(port, baud).await?;

    let packet = hex::decode(hex_data.trim())
        .map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?;

    println!("Sending {} bytes: {}", packet.len(), hex_data);
    dev.send_packet(&packet).await?;
    println!("Sent!");

    Ok(())
}

async fn cmd_recv(port: &str, baud: u32, timeout_secs: u64) -> Result<()> {
    let dev = device::Device::connect(port, baud).await?;

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

async fn cmd_telemetry(port: &str, baud: u32, watch: bool) -> Result<()> {
    let serial_port = serial::SerialPort::open(port, baud).await?;
    let mut proto = protocol::Protocol::new(serial_port);

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

/// USB VID/PID to board type mapping
struct UsbDeviceInfo {
    vid: u16,
    pid: u16,
    board: BoardType,
    name: &'static str,
}

const USB_DEVICE_MAP: &[UsbDeviceInfo] = &[
    // ESP32-S3 native USB (Heltec V3/V4, T3S3, T-Deck, Station G2, etc.)
    UsbDeviceInfo { vid: 0x303a, pid: 0x1001, board: BoardType::HeltecV3, name: "ESP32-S3 (Heltec V3/V4, T3S3, etc.)" },
    UsbDeviceInfo { vid: 0x303a, pid: 0x80d1, board: BoardType::HeltecV3, name: "ESP32-S3 JTAG" },

    // Silicon Labs CP210x (common on many ESP32 boards)
    UsbDeviceInfo { vid: 0x10c4, pid: 0xea60, board: BoardType::LilygoTbeam, name: "CP210x (T-Beam, T-LoRa, etc.)" },

    // CH340/CH341 (Heltec, clones)
    UsbDeviceInfo { vid: 0x1a86, pid: 0x7523, board: BoardType::HeltecV3, name: "CH340 (Heltec, clones)" },
    UsbDeviceInfo { vid: 0x1a86, pid: 0x55d4, board: BoardType::HeltecV3, name: "CH9102 (Heltec V3)" },

    // FTDI
    UsbDeviceInfo { vid: 0x0403, pid: 0x6001, board: BoardType::DiyV1, name: "FTDI FT232" },

    // Nordic/nRF52840 (RAK, T-Echo, etc.)
    UsbDeviceInfo { vid: 0x239a, pid: 0x8029, board: BoardType::Rak4631, name: "RAK4631 (nRF52840)" },
    UsbDeviceInfo { vid: 0x239a, pid: 0x0029, board: BoardType::Rak4631, name: "RAK4631 Bootloader" },
    UsbDeviceInfo { vid: 0x239a, pid: 0x80ab, board: BoardType::LilygoTecho, name: "T-Echo (nRF52840)" },

    // Seeed
    UsbDeviceInfo { vid: 0x2886, pid: 0x802f, board: BoardType::SeeedXiaoNrf52840, name: "Seeed Xiao nRF52840" },
    UsbDeviceInfo { vid: 0x2886, pid: 0x0052, board: BoardType::SeeedTrackerT1000e, name: "Seeed Tracker" },

    // RP2040
    UsbDeviceInfo { vid: 0x2e8a, pid: 0x000a, board: BoardType::RpiPico, name: "Raspberry Pi Pico" },
    UsbDeviceInfo { vid: 0x2e8a, pid: 0xf00a, board: BoardType::RpiPicoW, name: "Raspberry Pi Pico W" },
];

const CP210X_BOARDS: &[BoardType] = &[
    BoardType::HeltecV3,
    BoardType::HeltecV4,
    BoardType::LilygoTbeam,
    BoardType::LilygoTloraV2116,
    BoardType::NanoG1,
    BoardType::StationG1,
];

const CH340_BOARDS: &[BoardType] = &[
    BoardType::HeltecV3,
    BoardType::HeltecV4,
    BoardType::HeltecWirelessStickLiteV3,
];

const ESP32S3_BOARDS: &[BoardType] = &[
    BoardType::HeltecV3,
    BoardType::HeltecV4,
    BoardType::LilygoT3s3,
    BoardType::LilygoTbeamSupreme,
    BoardType::LilygoTdeck,
    BoardType::StationG2,
];

fn detect_boards() -> Vec<(String, Option<BoardType>, String, &'static [BoardType])> {
    let mut detected = Vec::new();

    if let Ok(ports) = serialport::available_ports() {
        for port in ports {
            if let serialport::SerialPortType::UsbPort(info) = port.port_type {
                // Check product string for hints
                let product = info.product.as_deref().unwrap_or("");
                let manufacturer = info.manufacturer.as_deref().unwrap_or("");

                let (chip_name, possible_boards): (&str, &[BoardType]) = match (info.vid, info.pid) {
                    // ESP32-S3 native USB
                    (0x303a, _) => ("ESP32-S3 native USB", ESP32S3_BOARDS),

                    // CP210x - many boards use this
                    (0x10c4, 0xea60) => ("CP210x USB-UART", CP210X_BOARDS),

                    // CH340/CH9102
                    (0x1a86, 0x7523) => ("CH340", CH340_BOARDS),
                    (0x1a86, 0x55d4) => ("CH9102", CH340_BOARDS),

                    // Nordic/nRF52840
                    (0x239a, _) => ("nRF52840", &[BoardType::Rak4631, BoardType::LilygoTecho]),

                    // Seeed
                    (0x2886, _) => ("Seeed", &[BoardType::SeeedXiaoNrf52840, BoardType::SeeedTrackerT1000e]),

                    // RP2040
                    (0x2e8a, _) => ("RP2040", &[BoardType::RpiPico, BoardType::RpiPicoW, BoardType::Rak11310]),

                    // FTDI
                    (0x0403, _) => ("FTDI", &[BoardType::DiyV1]),

                    _ => ("Unknown", &[]),
                };

                // Try to narrow down from product/manufacturer strings
                let specific_board = if manufacturer.to_lowercase().contains("heltec") || product.to_lowercase().contains("heltec") {
                    Some(BoardType::HeltecV3)
                } else if product.to_lowercase().contains("t-beam") || product.to_lowercase().contains("tbeam") {
                    Some(BoardType::LilygoTbeam)
                } else if product.to_lowercase().contains("t-echo") {
                    Some(BoardType::LilygoTecho)
                } else if product.to_lowercase().contains("rak") {
                    Some(BoardType::Rak4631)
                } else if possible_boards.len() == 1 {
                    Some(possible_boards[0])
                } else {
                    None
                };

                detected.push((
                    port.port_name.clone(),
                    specific_board,
                    format!("{} (VID:{:04x} PID:{:04x})", chip_name, info.vid, info.pid),
                    possible_boards,
                ));
            }
        }
    }

    detected
}

async fn cmd_flash(board: Option<BoardType>, port: Option<&str>, monitor: bool, local: Option<&str>, detect: bool) -> Result<()> {
    use std::process::Command;
    use std::io::{self, Write};

    // Detect connected devices
    let detected = detect_boards();

    // If --detect flag, just list devices
    if detect {
        println!("Detected devices:\n");
        if detected.is_empty() {
            println!("  No compatible devices found.");
            println!("\n  Make sure your device is connected via USB.");
        } else {
            for (port, specific, chip_name, possible) in &detected {
                if let Some(board) = specific {
                    println!("  {} - {:?} (confirmed)", port, board);
                } else {
                    println!("  {} - {} (could be one of:)", port, chip_name);
                    for b in *possible {
                        println!("       - {:?}", b);
                    }
                }
                println!();
            }
        }
        return Ok(());
    }

    // Determine board to flash
    let board = if let Some(b) = board {
        b
    } else {
        // Auto-detect
        if detected.is_empty() {
            anyhow::bail!(
                "No device detected. Please specify a board type:\n\
                 meshgrid-cli flash heltec-v3\n\
                 meshgrid-cli flash --help  (for all options)"
            );
        } else if detected.len() == 1 {
            let (ref detected_port, specific, ref chip_name, possible) = &detected[0];

            if let Some(board) = specific {
                println!("Auto-detected: {:?} on {}\n", board, detected_port);
                *board
            } else if possible.is_empty() {
                anyhow::bail!(
                    "Unknown device on {}. Please specify board type:\n\
                     meshgrid-cli flash heltec-v3",
                    detected_port
                );
            } else {
                // Show menu for user to select
                println!("Device detected on {}: {}\n", detected_port, chip_name);
                println!("Which board is this?\n");
                for (i, b) in possible.iter().enumerate() {
                    println!("  [{}] {:?}", i + 1, b);
                }
                println!();
                print!("Enter number (1-{}): ", possible.len());
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let choice: usize = input.trim().parse().unwrap_or(0);

                if choice < 1 || choice > possible.len() {
                    anyhow::bail!("Invalid selection");
                }
                possible[choice - 1]
            }
        } else {
            println!("Multiple devices detected:\n");
            for (i, (port, specific, chip_name, _)) in detected.iter().enumerate() {
                if let Some(board) = specific {
                    println!("  [{}] {} - {:?}", i + 1, port, board);
                } else {
                    println!("  [{}] {} - {}", i + 1, port, chip_name);
                }
            }
            anyhow::bail!(
                "\nMultiple devices found. Please specify port:\n\
                 meshgrid-cli flash heltec-v3 -p /dev/ttyUSB0"
            );
        }
    };

    // Get port (auto-detect if not specified)
    let flash_port = if let Some(p) = port {
        Some(p.to_string())
    } else if detected.len() == 1 {
        Some(detected[0].0.clone())
    } else {
        None
    };

    // Map board type to PlatformIO environment name
    let (env_name, board_name) = match board {
        // Heltec ESP32-S3
        BoardType::HeltecV3 => ("heltec_v3", "Heltec V3"),
        BoardType::HeltecV4 => ("heltec_v4", "Heltec V4"),
        BoardType::HeltecWirelessStickLiteV3 => ("heltec_wireless_stick_lite_v3", "Heltec Wireless Stick Lite V3"),
        BoardType::HeltecWirelessTracker => ("heltec_wireless_tracker", "Heltec Wireless Tracker"),
        BoardType::HeltecWirelessPaper => ("heltec_wireless_paper", "Heltec Wireless Paper"),
        BoardType::HeltecVisionMasterT190 => ("heltec_vision_master_t190", "Heltec Vision Master T190"),
        BoardType::HeltecVisionMasterE213 => ("heltec_vision_master_e213", "Heltec Vision Master E213"),
        BoardType::HeltecVisionMasterE290 => ("heltec_vision_master_e290", "Heltec Vision Master E290"),
        BoardType::HeltecHt62 => ("heltec_ht62", "Heltec HT62"),
        BoardType::HeltecMeshNodeT114 => ("heltec_mesh_node_t114", "Heltec Mesh Node T114"),
        BoardType::HeltecMeshPocket => ("heltec_mesh_pocket", "Heltec MeshPocket"),

        // LilyGo ESP32-S3
        BoardType::LilygoT3s3 => ("lilygo_t3s3", "LilyGo T3S3"),
        BoardType::LilygoT3s3Eink => ("lilygo_t3s3_eink", "LilyGo T3S3 E-Ink"),
        BoardType::LilygoTbeamSupreme => ("lilygo_tbeam_supreme", "LilyGo T-Beam Supreme"),
        BoardType::LilygoTdeck => ("lilygo_tdeck", "LilyGo T-Deck"),
        BoardType::LilygoTdeckPro => ("lilygo_tdeck_pro", "LilyGo T-Deck Pro"),
        BoardType::LilygoTloraPager => ("lilygo_tlora_pager", "LilyGo T-LoRa Pager"),
        BoardType::LilygoTwatchS3 => ("lilygo_twatch_s3", "LilyGo T-Watch S3"),

        // LilyGo ESP32
        BoardType::LilygoTbeam => ("lilygo_tbeam", "LilyGo T-Beam"),
        BoardType::LilygoTloraV2116 => ("lilygo_tlora_v21_16", "LilyGo T-LoRa V2.1-1.6"),
        BoardType::LilygoTloraV2118 => ("lilygo_tlora_v21_18", "LilyGo T-LoRa V2.1-1.8"),

        // LilyGo nRF52840
        BoardType::LilygoTecho => ("lilygo_techo", "LilyGo T-Echo"),

        // RAK nRF52840
        BoardType::Rak4631 => ("rak4631", "RAK4631"),
        BoardType::RakWismeshRepeater => ("rak_wismesh_repeater", "RAK WisMesh Repeater"),
        BoardType::RakWismeshTap => ("rak_wismesh_tap", "RAK WisMesh Tap"),
        BoardType::RakWismeshTag => ("rak_wismesh_tag", "RAK WisMesh Tag"),
        BoardType::Rak34011w => ("rak3401_1w", "RAK3401 1W"),

        // RAK ESP32/S3
        BoardType::Rak11200 => ("rak11200", "RAK11200"),
        BoardType::Rak3312 => ("rak3312", "RAK3312"),

        // RAK RP2040
        BoardType::Rak11310 => ("rak11310", "RAK11310"),

        // Seeed nRF52840
        BoardType::SeeedTrackerT1000e => ("seeed_tracker_t1000e", "Seeed Tracker T1000-E"),
        BoardType::SeeedXiaoNrf52840 => ("seeed_xiao_nrf52840", "Seeed Xiao nRF52840"),
        BoardType::SeeedSensecapSolar => ("seeed_sensecap_solar", "Seeed SenseCAP Solar"),
        BoardType::SeeedWioTrackerL1 => ("seeed_wio_tracker_l1", "Seeed Wio Tracker L1"),
        BoardType::SeeedWioTrackerL1Eink => ("seeed_wio_tracker_l1_eink", "Seeed Wio Tracker L1 E-Ink"),
        BoardType::SeeedWioWm1110 => ("seeed_wio_wm1110", "Seeed Wio WM1110"),

        // Seeed ESP32-S3
        BoardType::SeeedSensecapIndicator => ("seeed_sensecap_indicator", "Seeed SenseCAP Indicator"),
        BoardType::SeeedXiaoEsp32s3 => ("seeed_xiao_esp32s3", "Seeed Xiao ESP32-S3"),

        // Elecrow
        BoardType::ThinknodeM1 => ("thinknode_m1", "ThinkNode M1"),
        BoardType::ThinknodeM2 => ("thinknode_m2", "ThinkNode M2"),
        BoardType::ThinknodeM3 => ("thinknode_m3", "ThinkNode M3"),
        BoardType::ThinknodeM5 => ("thinknode_m5", "ThinkNode M5"),
        BoardType::Crowpanel24tft => ("crowpanel_24tft", "Crowpanel 2.4/2.8 TFT"),
        BoardType::Crowpanel35tft => ("crowpanel_35tft", "Crowpanel 3.5 TFT"),
        BoardType::Crowpanel43tft => ("crowpanel_43tft", "Crowpanel 4.3/5.0/7.0 TFT"),

        // B&Q Consulting
        BoardType::StationG2 => ("station_g2", "Station G2"),
        BoardType::StationG1 => ("station_g1", "Station G1"),
        BoardType::NanoG1 => ("nano_g1", "Nano G1"),
        BoardType::NanoG1Explorer => ("nano_g1_explorer", "Nano G1 Explorer"),
        BoardType::NanoG2Ultra => ("nano_g2_ultra", "Nano G2 Ultra"),

        // M5Stack
        BoardType::M5stack => ("m5stack", "M5 Stack"),
        BoardType::M5stackUnitC6l => ("m5stack_unit_c6l", "M5Stack Unit C6L"),

        // Other Vendors
        BoardType::MuziBase => ("muzi_base", "muzi BASE"),
        BoardType::MuziR1Neo => ("muzi_r1_neo", "muzi R1 Neo"),
        BoardType::NomadstarMeteorPro => ("nomadstar_meteor_pro", "NomadStar Meteor Pro"),
        BoardType::CanaryOne => ("canary_one", "Canary One"),
        BoardType::Radiomaster900Bandit => ("radiomaster_900_bandit", "RadioMaster 900 Bandit"),
        BoardType::EbyteEoraS3 => ("ebyte_eora_s3", "EByte EoRa-S3"),
        BoardType::TracksengerSmall => ("tracksenger_small", "TrackSenger Small"),
        BoardType::TracksengerBig => ("tracksenger_big", "TrackSenger Big"),
        BoardType::PiComputerS3 => ("pi_computer_s3", "Pi Computer S3"),
        BoardType::Unphone => ("unphone", "unPhone"),

        // RP2040
        BoardType::Rp2040Lora => ("rp2040_lora", "RP2040 LoRa"),
        BoardType::RpiPico => ("rpi_pico", "Raspberry Pi Pico"),
        BoardType::RpiPicoW => ("rpi_pico_w", "Raspberry Pi Pico W"),

        // DIY
        BoardType::DiyV1 => ("diy_v1", "DIY V1"),
        BoardType::Hydra => ("hydra", "Hydra"),
        BoardType::Nrf52PromicroDiy => ("nrf52_promicro_diy", "nRF52 Pro-micro DIY"),
    };

    // Find firmware directory
    let firmware_dir = if let Some(path) = local {
        std::path::PathBuf::from(path)
    } else {
        // Look for meshgrid-firmware as sibling directory
        std::env::current_exe()?
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("meshgrid-firmware"))
            .filter(|p| p.exists())
            .or_else(|| {
                let cwd = std::env::current_dir().ok()?;
                let fw = cwd.join("../meshgrid-firmware");
                if fw.exists() { Some(fw) } else { None }
            })
            .ok_or_else(|| anyhow::anyhow!(
                "Could not find meshgrid-firmware directory.\n\
                 Use --local <path> or clone https://github.com/BetterInc/meshgrid-firmware"
            ))?
    };

    // Check for platformio.ini
    if !firmware_dir.join("platformio.ini").exists() {
        anyhow::bail!("No platformio.ini found in {:?}", firmware_dir);
    }

    println!("Flashing {} firmware...\n", board_name);

    // Build PlatformIO command
    let mut pio_args = vec!["run", "-e", env_name, "-t", "upload"];

    if monitor {
        pio_args.push("-t");
        pio_args.push("monitor");
    }

    if let Some(ref p) = flash_port {
        pio_args.push("--upload-port");
        pio_args.push(p);
    }

    let status = Command::new("pio")
        .args(&pio_args)
        .current_dir(&firmware_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("PlatformIO flash failed. Make sure PlatformIO is installed: pip install platformio");
    }

    println!("\nFlash complete!");

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
