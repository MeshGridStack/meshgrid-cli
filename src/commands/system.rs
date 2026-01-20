//! System commands

use crate::cli::{BoardType, TimeAction};
use crate::device::Device;
use crate::protocol::Response;
use anyhow::{bail, Result};

pub async fn cmd_reboot(port: &str, baud: u32) -> Result<()> {
    let mut dev = Device::connect(port, baud).await?;
    dev.reboot().await?;
    println!("Device rebooting...");
    Ok(())
}

pub async fn cmd_ui(port: &str, baud: u32) -> Result<()> {
    crate::ui::run(port, baud).await
}

pub async fn cmd_mode(port: &str, baud: u32, pin: Option<&str>, mode: &str) -> Result<()> {
    let dev = super::connect_with_auth(port, baud, pin).await?;
    let mut proto = dev.into_protocol();

    let mode_lower = mode.to_lowercase();
    let valid_modes = ["client", "repeater", "room"];

    if !valid_modes.contains(&mode_lower.as_str()) {
        bail!("Invalid mode '{mode}'. Valid modes: client, repeater, room");
    }

    let command = format!("/mode {mode_lower}");
    match proto.command(&command).await? {
        Response::Ok(msg) => {
            if let Some(m) = msg {
                println!("{m}");
            } else {
                println!("Mode set to: {}", mode_lower.to_uppercase());
            }
            Ok(())
        }
        Response::Error(e) => bail!("Failed to set mode: {e}"),
        Response::Json(_) => bail!("Unexpected response to mode command"),
    }
}

pub async fn cmd_time(
    port: &str,
    baud: u32,
    pin: Option<&str>,
    action: Option<TimeAction>,
) -> Result<()> {
    use chrono::Local;

    let dev = super::connect_with_auth(port, baud, pin).await?;
    let mut proto = dev.into_protocol();

    let action = action.unwrap_or(TimeAction::Show);

    match action {
        TimeAction::Show => {
            // Query device time
            match proto.command("TIME").await? {
                Response::Ok(msg) => {
                    println!(
                        "{}",
                        msg.unwrap_or_else(|| "Device time not set".to_string())
                    );
                    Ok(())
                }
                Response::Error(e) => bail!("Failed to get time: {e}"),
                Response::Json(_) => bail!("Unexpected response to TIME"),
            }
        }
        TimeAction::Sync => {
            // Sync with computer's current time
            let time_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let command = format!("/time {time_str}");
            match proto.command(&command).await? {
                Response::Ok(msg) => {
                    if let Some(m) = msg {
                        println!("{m}");
                    } else {
                        println!("Time synced: {time_str}");
                    }
                    Ok(())
                }
                Response::Error(e) => bail!("Failed to sync time: {e}"),
                Response::Json(_) => bail!("Unexpected response to time sync"),
            }
        }
        TimeAction::Set { time } => {
            // Set device time to specific value
            let command = format!("/time {time}");
            match proto.command(&command).await? {
                Response::Ok(msg) => {
                    if let Some(m) = msg {
                        println!("{m}");
                    } else {
                        println!("Time set: {time}");
                    }
                    Ok(())
                }
                Response::Error(e) => bail!("Failed to set time: {e}"),
                Response::Json(_) => bail!("Unexpected response to time set"),
            }
        }
    }
}

pub async fn cmd_debug(
    port: &str,
    baud: u32,
    output_file: Option<String>,
    timeout_secs: u64,
) -> Result<()> {
    use crate::serial::SerialPort;
    use std::fs::OpenOptions;
    use std::io::Write;

    let infinite = timeout_secs == 0;

    if let Some(ref file) = output_file {
        println!("Capturing debug output to: {file}");
    } else {
        println!("Streaming debug output to stdout");
    }

    if infinite {
        println!("Running indefinitely (Press Ctrl+C to stop)\n");
    } else {
        println!("Timeout: {timeout_secs} seconds\n");
    }

    // Open output file (unbuffered)
    let mut file_handle = if let Some(ref path) = output_file {
        Some(OpenOptions::new().create(true).append(true).open(path)?)
    } else {
        None
    };

    let mut serial = SerialPort::open(port, baud).await?;
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    loop {
        // Check timeout
        if !infinite && start.elapsed() >= timeout {
            break;
        }

        // Read COBS frame with short timeout
        match serial
            .read_cobs_frame_timeout(std::time::Duration::from_millis(100))
            .await
        {
            Ok(Some(frame)) => {
                // Decode frame to string
                let text = String::from_utf8_lossy(&frame).to_string();

                // Try to parse as JSON to check if it's a debug frame
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if json.get("type").and_then(|v| v.as_str()) == Some("debug") {
                        // It's a debug frame - extract and output
                        let level = json.get("level").and_then(|v| v.as_str()).unwrap_or("INFO");
                        let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or("");

                        let output_line = format!("[{level}] {msg}\n");

                        if let Some(ref mut file) = file_handle {
                            file.write_all(output_line.as_bytes())?;
                            file.flush()?; // Force immediate write
                        } else {
                            print!("{output_line}");
                            std::io::stdout().flush()?;
                        }
                    }
                }
            }
            Ok(None) => {
                // Timeout, continue
            }
            Err(e) => {
                eprintln!("Serial error: {e}");
                break;
            }
        }
    }

    if output_file.is_some() {
        println!("\nDebug capture stopped");
    } else {
        println!("\n--- End of debug output ---");
    }
    Ok(())
}

/// USB VID/PID to board type mapping (prepared for future auto-detection)
#[allow(dead_code)]
struct UsbDeviceInfo {
    vid: u16,
    pid: u16,
    board: BoardType,
    name: &'static str,
}

#[allow(dead_code)]
const USB_DEVICE_MAP: &[UsbDeviceInfo] = &[
    // ESP32-S3 native USB (Heltec V3/V4, T3S3, T-Deck, Station G2, etc.)
    UsbDeviceInfo {
        vid: 0x303a,
        pid: 0x1001,
        board: BoardType::HeltecV3,
        name: "ESP32-S3 (Heltec V3/V4, T3S3, etc.)",
    },
    UsbDeviceInfo {
        vid: 0x303a,
        pid: 0x80d1,
        board: BoardType::HeltecV3,
        name: "ESP32-S3 JTAG",
    },
    // Silicon Labs CP210x (common on many ESP32 boards)
    UsbDeviceInfo {
        vid: 0x10c4,
        pid: 0xea60,
        board: BoardType::LilygoTbeam,
        name: "CP210x (T-Beam, T-LoRa, etc.)",
    },
    // CH340/CH341 (Heltec, clones)
    UsbDeviceInfo {
        vid: 0x1a86,
        pid: 0x7523,
        board: BoardType::HeltecV3,
        name: "CH340 (Heltec, clones)",
    },
    UsbDeviceInfo {
        vid: 0x1a86,
        pid: 0x55d4,
        board: BoardType::HeltecV3,
        name: "CH9102 (Heltec V3)",
    },
    // FTDI
    UsbDeviceInfo {
        vid: 0x0403,
        pid: 0x6001,
        board: BoardType::DiyV1,
        name: "FTDI FT232",
    },
    // Nordic/nRF52840 (RAK, T-Echo, etc.)
    UsbDeviceInfo {
        vid: 0x239a,
        pid: 0x8029,
        board: BoardType::Rak4631,
        name: "RAK4631 (nRF52840)",
    },
    UsbDeviceInfo {
        vid: 0x239a,
        pid: 0x0029,
        board: BoardType::Rak4631,
        name: "RAK4631 Bootloader",
    },
    UsbDeviceInfo {
        vid: 0x239a,
        pid: 0x80ab,
        board: BoardType::LilygoTecho,
        name: "T-Echo (nRF52840)",
    },
    // Seeed
    UsbDeviceInfo {
        vid: 0x2886,
        pid: 0x802f,
        board: BoardType::SeeedXiaoNrf52840,
        name: "Seeed Xiao nRF52840",
    },
    UsbDeviceInfo {
        vid: 0x2886,
        pid: 0x0052,
        board: BoardType::SeeedTrackerT1000e,
        name: "Seeed Tracker",
    },
    // RP2040
    UsbDeviceInfo {
        vid: 0x2e8a,
        pid: 0x000a,
        board: BoardType::RpiPico,
        name: "Raspberry Pi Pico",
    },
    UsbDeviceInfo {
        vid: 0x2e8a,
        pid: 0xf00a,
        board: BoardType::RpiPicoW,
        name: "Raspberry Pi Pico W",
    },
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

                let (chip_name, possible_boards): (&str, &[BoardType]) = match (info.vid, info.pid)
                {
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
                    (0x2886, _) => (
                        "Seeed",
                        &[BoardType::SeeedXiaoNrf52840, BoardType::SeeedTrackerT1000e],
                    ),

                    // RP2040
                    (0x2e8a, _) => (
                        "RP2040",
                        &[BoardType::RpiPico, BoardType::RpiPicoW, BoardType::Rak11310],
                    ),

                    // FTDI
                    (0x0403, _) => ("FTDI", &[BoardType::DiyV1]),

                    _ => ("Unknown", &[]),
                };

                // Try to narrow down from product/manufacturer strings
                let specific_board = if manufacturer.to_lowercase().contains("heltec")
                    || product.to_lowercase().contains("heltec")
                {
                    Some(BoardType::HeltecV3)
                } else if product.to_lowercase().contains("t-beam")
                    || product.to_lowercase().contains("tbeam")
                {
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

#[allow(clippy::too_many_lines)]
pub async fn cmd_flash(
    board: Option<BoardType>,
    port: Option<&str>,
    monitor: bool,
    local: Option<&str>,
    detect: bool,
) -> Result<()> {
    use std::io::{self, Write};
    use std::process::Command;

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
                    println!("  {port} - {board:?} (confirmed)");
                } else {
                    println!("  {port} - {chip_name} (could be one of:)");
                    for b in *possible {
                        println!("       - {b:?}");
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
            bail!(
                "No device detected. Please specify a board type:\n\
                 meshgrid-cli flash heltec-v3\n\
                 meshgrid-cli flash --help  (for all options)"
            );
        } else if detected.len() == 1 {
            let (ref detected_port, specific, ref chip_name, possible) = &detected[0];

            if let Some(board) = specific {
                println!("Auto-detected: {board:?} on {detected_port}\n");
                *board
            } else if possible.is_empty() {
                bail!(
                    "Unknown device on {detected_port}. Please specify board type:\n\
                     meshgrid-cli flash heltec-v3"
                );
            } else {
                // Show menu for user to select
                println!("Device detected on {detected_port}: {chip_name}\n");
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
                    bail!("Invalid selection");
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
            bail!(
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
        BoardType::HeltecWirelessStickLiteV3 => (
            "heltec_wireless_stick_lite_v3",
            "Heltec Wireless Stick Lite V3",
        ),
        BoardType::HeltecWirelessTracker => ("heltec_wireless_tracker", "Heltec Wireless Tracker"),
        BoardType::HeltecWirelessPaper => ("heltec_wireless_paper", "Heltec Wireless Paper"),
        BoardType::HeltecVisionMasterT190 => {
            ("heltec_vision_master_t190", "Heltec Vision Master T190")
        }
        BoardType::HeltecVisionMasterE213 => {
            ("heltec_vision_master_e213", "Heltec Vision Master E213")
        }
        BoardType::HeltecVisionMasterE290 => {
            ("heltec_vision_master_e290", "Heltec Vision Master E290")
        }
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
        BoardType::SeeedWioTrackerL1Eink => {
            ("seeed_wio_tracker_l1_eink", "Seeed Wio Tracker L1 E-Ink")
        }
        BoardType::SeeedWioWm1110 => ("seeed_wio_wm1110", "Seeed Wio WM1110"),

        // Seeed ESP32-S3
        BoardType::SeeedSensecapIndicator => {
            ("seeed_sensecap_indicator", "Seeed SenseCAP Indicator")
        }
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
                if fw.exists() {
                    Some(fw)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find meshgrid-firmware directory.\n\
                 Use --local <path> or clone https://github.com/MeshGridStack/meshgrid-firmware"
                )
            })?
    };

    // Check for platformio.ini
    if !firmware_dir.join("platformio.ini").exists() {
        bail!("No platformio.ini found in {}", firmware_dir.display());
    }

    println!("Flashing {board_name} firmware...\n");

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
        bail!("PlatformIO flash failed. Make sure PlatformIO is installed: pip install platformio");
    }

    println!("\nFlash complete!");

    Ok(())
}
