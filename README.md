# meshgrid-cli

Command line interface for meshgrid mesh networking.

## Installation

### From Source (Development)

```bash
git clone https://github.com/BetterInc/meshgrid-cli
cd meshgrid-cli
cargo install --path .
```

### From crates.io (Coming Soon)

```bash
cargo install meshgrid-cli
```

## Quick Start

```bash
# List connected devices
meshgrid ports

# Get device info
meshgrid -p /dev/ttyUSB0 info

# Send a message
meshgrid send "Hello mesh!"

# Monitor traffic
meshgrid monitor

# Interactive UI
meshgrid ui
```

## Flashing Firmware

### For Users (Downloads from GitHub)

```bash
# Flash latest firmware
meshgrid flash heltec-v3

# Flash specific version
meshgrid flash heltec-v3 --version 1.0.0

# Flash and monitor
meshgrid flash heltec-v3 --monitor
```

### For Developers (Local Build)

If you have meshgrid-firmware cloned locally:

```bash
# Clone all repos
git clone https://github.com/BetterInc/meshgrid-core
git clone https://github.com/BetterInc/meshgrid-cli
git clone https://github.com/BetterInc/meshgrid-firmware

# Install PlatformIO
pip install platformio

# Flash from local build
meshgrid flash heltec-v3 --local ../meshgrid-firmware

# Or use PlatformIO directly
cd meshgrid-firmware
pio run -e heltec_v3 -t upload -t monitor
```

## Supported Boards

| Board | Command |
|-------|---------|
| Heltec V3 | `meshgrid flash heltec-v3` |
| Heltec V4 | `meshgrid flash heltec-v4` |
| LilyGo T3S3 | `meshgrid flash t3s3` |
| LilyGo T-Beam | `meshgrid flash tbeam` |
| LilyGo T-Echo | `meshgrid flash techo` |
| RAK4631 | `meshgrid flash rak4631` |
| Station G2 | `meshgrid flash station-g2` |

## Commands

```
meshgrid <COMMAND>

Commands:
  ports       List available serial ports
  info        Show device info
  send        Send a message
  monitor     Monitor mesh traffic
  ui          Interactive terminal UI
  flash       Flash firmware to device
  config      Get/set device configuration
  neighbors   Show neighbor table
  trace       Trace route to a node
  reboot      Reboot the device
  telemetry   Show device telemetry

Options:
  -p, --port <PORT>    Serial port (auto-detect if not specified)
  -b, --baud <BAUD>    Baud rate [default: 115200]
  -v, --verbose        Enable verbose logging
```

## Configuration

Config file: `~/.config/meshgrid/config.toml`

```toml
[device]
default_port = "/dev/ttyUSB0"

[lora]
frequency = 868.0
spreading_factor = 9

[firmware]
# For users: download from GitHub releases
source = "github"
repo = "BetterInc/meshgrid-firmware"

# For developers: use local path
# source = "local"
# path = "/home/user/meshgrid-firmware"
```

## Development Workflow

```bash
# 1. Clone all repositories
mkdir meshgrid && cd meshgrid
git clone https://github.com/BetterInc/meshgrid-core
git clone https://github.com/BetterInc/meshgrid-cli
git clone https://github.com/BetterInc/meshgrid-firmware

# 2. Build CLI
cd meshgrid-cli
cargo build --release

# 3. Flash device (local firmware)
./target/release/meshgrid flash heltec-v3 --local ../meshgrid-firmware

# 4. Monitor
./target/release/meshgrid monitor
```

## License

MIT
