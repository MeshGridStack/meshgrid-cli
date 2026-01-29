//! CLI argument definitions using clap

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "meshgrid")]
#[command(author, version, about = "Meshgrid mesh networking CLI", long_about = None)]
pub struct Cli {
    /// Serial port device (e.g., /dev/ttyUSB0 on Linux, COM3 on Windows)
    #[arg(short, long, global = true)]
    pub port: Option<String>,

    /// Baud rate
    #[arg(short, long, default_value = "115200", global = true)]
    pub baud: u32,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// PIN for authentication (if device has security enabled)
    #[arg(long, global = true)]
    pub pin: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available serial ports
    Ports,

    /// Connect to a device and show info
    Info,

    /// Send a text message
    Send {
        /// Destination node (name or hash)
        #[arg(short = 't', long = "to")]
        to: Option<String>,

        /// Channel name (e.g., "Public", "test-v1")
        #[arg(short = 'c', long = "channel")]
        channel: Option<String>,

        /// Message text
        #[arg(last = true)]
        message: String,
    },

    /// Interactive terminal UI
    Ui,

    /// Get/set device configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Show neighbor table
    Neighbors,

    /// Trace route to a node
    Trace {
        /// Target node (name or hash)
        target: String,
    },

    /// Reboot device
    Reboot,

    /// Send raw packet (hex)
    Raw {
        /// Packet data in hex format
        hex: String,
    },

    /// Receive raw packets
    Recv {
        /// Timeout in seconds
        #[arg(short, long, default_value = "60")]
        timeout: u64,
    },

    /// Show telemetry data
    Telemetry {
        /// Watch mode (continuous updates)
        #[arg(short, long)]
        watch: bool,
    },

    /// Show statistics
    Stats,

    /// Set device mode
    Mode {
        /// Mode (client, repeater, or room)
        #[arg(value_enum)]
        mode: DeviceMode,
    },

    /// Manage device time
    Time {
        #[command(subcommand)]
        action: Option<TimeAction>,
    },

    /// Manage message inbox
    Messages {
        #[command(subcommand)]
        action: Option<MessagesAction>,
    },

    /// Manage custom channels
    Channels {
        #[command(subcommand)]
        action: Option<ChannelsAction>,
    },

    /// Rotate device identity (generate new keys)
    RotateIdentity,

    /// Manage serial authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Set serial password (4-32 characters)
    Setpass {
        /// New password (4-32 characters)
        password: String,
    },

    /// Set Bluetooth PIN (6 digits)
    Setpin {
        /// New BLE PIN (6 digits)
        pin: String,
    },

    /// Send advertisement packets
    Advert {
        /// Send local advertisement only
        #[arg(short, long)]
        local: bool,

        /// Send flood advertisement only
        #[arg(short, long)]
        flood: bool,
    },

    /// Flash firmware to device
    Flash {
        /// Board type
        #[arg(short, long, value_enum)]
        board: Option<BoardType>,

        /// Monitor serial output after flashing
        #[arg(short, long)]
        monitor: bool,

        /// Use local firmware binary
        #[arg(short, long)]
        local: Option<String>,

        /// Auto-detect board type
        #[arg(short, long)]
        detect: bool,

        /// Firmware version to download from GitHub (e.g., "0.0.3" or "latest")
        #[arg(short, long)]
        version: Option<String>,

        /// Force re-download even if cached
        #[arg(long)]
        force_download: bool,

        /// Use cached firmware only, don't download
        #[arg(long)]
        offline: bool,
    },

    /// Capture debug output to file
    Debug {
        /// Output file path (defaults to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,

        /// Timeout in seconds (0 = infinite)
        #[arg(short, long, default_value = "0")]
        timeout: u64,
    },

    /// Read from stdin and send each line as a command
    #[command(name = "-")]
    Stdin,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set radio preset (EU, US, etc.)
    Preset { preset: String },

    /// Set node name
    Name { name: String },

    /// Set frequency (MHz)
    Frequency { freq_mhz: f32 },

    /// Set TX power (dBm)
    Power { power_dbm: i8 },

    /// Set bandwidth (kHz)
    Bandwidth { bandwidth_khz: f32 },

    /// Set spreading factor
    SpreadingFactor { sf: u8 },

    /// Set coding rate
    CodingRate { cr: u8 },

    /// Set preamble length
    Preamble { len: u16 },
}

#[derive(Subcommand)]
pub enum TimeAction {
    /// Show current time
    Show,

    /// Sync time with computer
    Sync,

    /// Set time (YYYY-MM-DD HH:MM:SS)
    Set { time: String },
}

#[derive(Subcommand)]
pub enum MessagesAction {
    /// Show message inbox
    Show,

    /// Clear message inbox
    Clear,
}

#[derive(Subcommand)]
pub enum ChannelsAction {
    /// List custom channels
    List,

    /// Add a custom channel
    /// For hashtag channels (e.g., #test), PSK is auto-generated as SHA256(name)
    /// For private channels, PSK must be provided (16 or 32 bytes, base64-encoded)
    Add {
        name: String,
        psk: Option<String>,
    },

    /// Remove a custom channel
    Remove { name: String },
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Authenticate with password
    Login {
        /// Password for authentication
        password: String,
    },

    /// Show BLE PIN and serial auth status
    Status,

    /// Enable serial authentication (requires password to be set)
    Enable,

    /// Disable serial authentication
    Disable,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DeviceMode {
    Client,
    Repeater,
    Room,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum BoardType {
    // Heltec ESP32-S3
    HeltecV3,
    HeltecV4,
    HeltecWirelessStickLiteV3,
    HeltecWirelessTracker,
    HeltecWirelessPaper,
    HeltecVisionMasterT190,
    HeltecVisionMasterE213,
    HeltecVisionMasterE290,
    HeltecHt62,
    HeltecMeshNodeT114,
    HeltecMeshPocket,

    // LilyGo ESP32-S3
    LilygoT3s3,
    LilygoT3s3Eink,
    LilygoTbeamSupreme,
    LilygoTdeck,
    LilygoTdeckPro,
    LilygoTloraPager,
    LilygoTwatchS3,

    // LilyGo ESP32
    LilygoTbeam,
    LilygoTloraV2116,
    LilygoTloraV2118,

    // LilyGo nRF52840
    LilygoTecho,

    // RAK nRF52840
    Rak4631,
    RakWismeshRepeater,
    RakWismeshTap,
    RakWismeshTag,
    Rak34011w,

    // RAK ESP32/S3
    Rak11200,
    Rak3312,

    // RAK RP2040
    Rak11310,

    // Seeed nRF52840
    SeeedTrackerT1000e,
    SeeedXiaoNrf52840,
    SeeedSensecapSolar,
    SeeedWioTrackerL1,
    SeeedWioTrackerL1Eink,
    SeeedWioWm1110,

    // Seeed ESP32-S3
    SeeedSensecapIndicator,
    SeeedXiaoEsp32s3,

    // Elecrow
    ThinknodeM1,
    ThinknodeM2,
    ThinknodeM3,
    ThinknodeM5,
    Crowpanel24tft,
    Crowpanel35tft,
    Crowpanel43tft,

    // B&Q Consulting
    StationG2,
    StationG1,
    NanoG1,
    NanoG1Explorer,
    NanoG2Ultra,

    // M5Stack
    M5stack,
    M5stackUnitC6l,

    // Other Vendors
    MuziBase,
    MuziR1Neo,
    NomadstarMeteorPro,
    CanaryOne,
    Radiomaster900Bandit,
    EbyteEoraS3,
    TracksengerSmall,
    TracksengerBig,
    PiComputerS3,
    Unphone,

    // RP2040
    Rp2040Lora,
    RpiPico,
    RpiPicoW,

    // DIY
    DiyV1,
    Hydra,
    Nrf52PromicroDiy,
}
