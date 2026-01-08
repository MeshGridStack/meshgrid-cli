# MeshGrid CLI

Command-line tool for managing MeshGrid LoRa mesh networking devices over USB serial.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

## What It Does

- Configure and manage MeshGrid devices over USB
- Send/receive messages across the mesh network
- Monitor mesh traffic in real-time
- View network topology and signal quality
- Flash firmware to 80+ supported boards
- Get telemetry (battery, GPS, sensors)

## Installation

```bash
git clone https://github.com/BetterInc/meshgrid-cli
cd meshgrid-cli
cargo build --release
# Binary at: ./target/release/meshgrid-cli
```

Or install globally:
```bash
cargo install --path .
```

## Quick Start

```bash
# List connected devices
meshgrid-cli ports

# Get device info (auto-detects port)
meshgrid-cli info

# Send a message
meshgrid-cli send "Hello mesh!"

# Monitor mesh traffic
meshgrid-cli monitor

# View neighbors
meshgrid-cli neighbors

# Get telemetry
meshgrid-cli telemetry
```

## Common Commands

```bash
meshgrid-cli ports                    # List serial ports
meshgrid-cli info                     # Device information
meshgrid-cli config                   # Show configuration
meshgrid-cli config name "my-node"    # Set device name
meshgrid-cli send "message"           # Broadcast message
meshgrid-cli send --to node "msg"     # Direct message
meshgrid-cli monitor                  # Monitor traffic
meshgrid-cli neighbors                # View neighbor table
meshgrid-cli stats                    # Performance stats
meshgrid-cli telemetry                # Device telemetry
meshgrid-cli flash heltec-v3          # Flash firmware
meshgrid-cli reboot                   # Reboot device
```

Use `meshgrid-cli --help` or `meshgrid-cli <command> --help` for more options.

## Firmware Flashing

Supports 80+ boards including Heltec, LilyGo, RAK, Seeed, and more.

```bash
# Detect boards
meshgrid-cli flash --detect

# Flash firmware
meshgrid-cli flash heltec-v3
meshgrid-cli flash lilygo-t3s3
meshgrid-cli flash rak4631

# See all boards
meshgrid-cli flash --help
```

Requires [PlatformIO](https://platformio.org/): `pip install platformio`

## Documentation

- Full command reference: `meshgrid-cli --help`
- Examples and tutorials: See `docs/` folder
- Protocol specification: `docs/PROTOCOL.md`

## Troubleshooting

**Permission denied (Linux)**:
```bash
sudo usermod -a -G dialout $USER
# Log out and back in
```

**Device not found**:
```bash
meshgrid-cli ports  # List available ports
meshgrid-cli -p /dev/ttyUSB0 info  # Specify port manually
```

## License

MIT

## Related

- [meshgrid-firmware](https://github.com/BetterInc/meshgrid-firmware) - Device firmware
- [meshgrid-core](https://github.com/BetterInc/meshgrid-core) - Core library
