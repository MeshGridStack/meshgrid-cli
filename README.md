# MeshGrid CLI

Command-line tool for managing MeshGrid LoRa mesh networking devices over USB serial.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

## What It Does

- **Configure and manage** MeshGrid devices over USB
- **Send/receive messages** across the mesh network
- **Monitor mesh traffic** in real-time with signal quality
- **View network topology** and neighbor tables
- **Flash firmware** to 70+ supported boards (Heltec, LilyGo, RAK, Seeed, and more)
- **Get telemetry** data (battery, GPS, sensors, temperature)
- **Manage channels** and message inbox
- **Debug and troubleshoot** with raw packet inspection

## Installation

### From Source

```bash
git clone https://github.com/BetterInc/meshgrid-cli
cd meshgrid-cli
cargo build --release
# Binary at: ./target/release/meshgrid-cli
```

### Install Globally

```bash
cargo install --path .
```

## Quick Start

```bash
# List connected devices
meshgrid-cli ports

# Get device info (auto-detects port)
meshgrid-cli info

# Send a broadcast message
meshgrid-cli send "Hello mesh!"

# Send a direct message
meshgrid-cli send --to "Alice" "Private message"

# Monitor mesh traffic in real-time
meshgrid-cli monitor

# View neighbor table
meshgrid-cli neighbors

# Get device statistics
meshgrid-cli stats

# View telemetry data
meshgrid-cli telemetry

# Watch telemetry updates
meshgrid-cli telemetry --watch
```

## Authentication

For PIN-protected devices:

```bash
# Provide PIN for commands that need authentication
meshgrid-cli --pin 123456 info
meshgrid-cli --pin 123456 send "message"
meshgrid-cli --pin 123456 config name "my-node"

# Or set globally
export MESHGRID_PIN=123456
meshgrid-cli info
```

## Command Reference

### Device Information

```bash
meshgrid-cli ports                    # List serial ports
meshgrid-cli info                     # Device information and radio config
meshgrid-cli stats                    # Performance statistics
meshgrid-cli neighbors                # Neighbor table with RSSI/SNR
meshgrid-cli telemetry                # Device telemetry (battery, GPS, sensors)
meshgrid-cli telemetry --watch        # Continuous telemetry updates
```

### Configuration

```bash
meshgrid-cli config                           # Show current config
meshgrid-cli config name "my-node"            # Set device name
meshgrid-cli config preset EU                 # Set radio preset (EU, US)
meshgrid-cli config frequency 915.0           # Set frequency (MHz)
meshgrid-cli config power 20                  # Set TX power (dBm)
meshgrid-cli config bandwidth 125.0           # Set bandwidth (kHz)
meshgrid-cli config spreading-factor 7        # Set spreading factor
```

### Messaging

```bash
meshgrid-cli send "Hello mesh!"               # Broadcast message
meshgrid-cli send --to "Alice" "Hi Alice"     # Direct message
meshgrid-cli monitor                          # Monitor all mesh traffic
meshgrid-cli messages                         # Show inbox
meshgrid-cli messages clear                   # Clear inbox
meshgrid-cli channels                         # List channels
```

### Network Tools

```bash
meshgrid-cli trace "Alice"                    # Trace route to node
meshgrid-cli advert                           # Send advertisement (both types)
meshgrid-cli advert --local                   # Send local advertisement only
meshgrid-cli advert --flood                   # Send flood advertisement only
meshgrid-cli raw 01020304                     # Send raw packet (hex)
meshgrid-cli recv --timeout 30                # Receive raw packets
```

### System Management

```bash
meshgrid-cli reboot                           # Reboot device
meshgrid-cli mode client                      # Set device mode
meshgrid-cli mode repeater                    # Set as repeater
meshgrid-cli time                             # Sync device time
meshgrid-cli time set "2026-01-12 15:30:00"   # Set specific time
meshgrid-cli log                              # Show event log
meshgrid-cli log enable                       # Enable logging
meshgrid-cli log disable                      # Disable logging
meshgrid-cli log clear                        # Clear log
meshgrid-cli rotate-identity                  # Generate new keypair
```

### Firmware Flashing

Flash firmware to 70+ supported boards:

```bash
# Auto-detect connected board
meshgrid-cli flash --detect

# Flash specific boards
meshgrid-cli flash heltec-v3
meshgrid-cli flash heltec-v4
meshgrid-cli flash lilygo-t3s3
meshgrid-cli flash lilygo-tbeam
meshgrid-cli flash rak4631
meshgrid-cli flash seeed-xiao-nrf52840

# Flash with monitoring
meshgrid-cli flash heltec-v3 --monitor

# Flash from local firmware directory
meshgrid-cli flash heltec-v3 --local ../meshgrid-firmware

# Specify port
meshgrid-cli flash heltec-v3 -p /dev/ttyUSB0

# See all supported boards
meshgrid-cli flash --help
```

**Supported board families:**
- **Heltec**: V3, V4, Wireless Stick, Vision Master, Mesh Node, etc.
- **LilyGo**: T3-S3, T-Beam, T-Deck, T-Echo, T-LoRa, T-Watch, etc.
- **RAK**: 4631, WisMesh series, 11200, 11310, 3401, etc.
- **Seeed**: Xiao nRF52840, Tracker T1000-E, SenseCAP, Wio Tracker, etc.
- **Other**: M5Stack, Elecrow, Station G1/G2, Nano G1/G2, DIY boards, and more

Requires [PlatformIO](https://platformio.org/): `pip install platformio`

### Debugging

```bash
# Stream debug output to stdout
meshgrid-cli debug

# Save debug output to file (recommended)
meshgrid-cli debug -o debug.log

# Custom timeout (0 = infinite)
meshgrid-cli debug -o debug.log --timeout 0

# Interactive terminal UI
meshgrid-cli ui
```

**Important:** The `debug` command keeps the serial port open continuously. You have two options:

**Option 1: Use two devices**
```bash
# Terminal 1: Capture debug from device 1
meshgrid-cli -p /dev/ttyUSB0 debug -o heltec.log

# Terminal 2: Send commands to device 2
meshgrid-cli -p /dev/ttyACM0 advert
meshgrid-cli -p /dev/ttyACM0 send "test"
```

**Option 2: Use separate terminals (same device)**
```bash
# Terminal 1: Capture debug output
meshgrid-cli -p /dev/ttyUSB0 debug -o debug.log

# Terminal 2: Stop debug (Ctrl+C in Terminal 1), then run commands
meshgrid-cli -p /dev/ttyUSB0 advert
```

**Debug output includes:**
- TX/RX packet details
- Protocol version detection (v0/v1)
- Encryption/decryption status
- Advertisement processing
- Error messages with detailed codes

### Port Selection

```bash
# Auto-detect (default)
meshgrid-cli info

# Specify port
meshgrid-cli -p /dev/ttyUSB0 info
meshgrid-cli -p /dev/ttyACM0 info

# Custom baud rate
meshgrid-cli -b 921600 info
```

## Use Cases

### Development & Testing

```bash
# Monitor all traffic while testing
meshgrid-cli monitor

# Check signal quality
meshgrid-cli neighbors

# Verify message delivery
meshgrid-cli send "test" && meshgrid-cli messages

# Debug radio issues
meshgrid-cli stats

# Capture debug logs during testing (separate terminal)
meshgrid-cli debug -o test-session.log
```

### Network Diagnostics

```bash
# Check network health
meshgrid-cli neighbors
meshgrid-cli stats

# Test connectivity
meshgrid-cli trace "remote-node"

# Monitor for interference
meshgrid-cli monitor

# Check device status
meshgrid-cli telemetry
```

### Configuration Management

```bash
# Configure multiple devices
for port in /dev/ttyUSB*; do
  meshgrid-cli -p $port config preset EU
  meshgrid-cli -p $port config power 20
done

# Backup configuration
meshgrid-cli info > config-backup.txt
meshgrid-cli config >> config-backup.txt
```

## Troubleshooting

### Permission Denied (Linux)

Add your user to the `dialout` group:

```bash
sudo usermod -a -G dialout $USER
# Log out and back in
```

### Device Not Found

List available ports and specify manually:

```bash
meshgrid-cli ports
meshgrid-cli -p /dev/ttyUSB0 info
```

### Command Timeout

Increase timeout or check device connection:

```bash
# Check if device responds
meshgrid-cli debug --timeout 10

# Try different baud rate
meshgrid-cli -b 115200 info
```

### Serial Port Busy

Only one process can access a serial port at a time:

```bash
# Error: Device or resource busy

# Solution 1: Stop other programs using the port
pkill -f ttyUSB0

# Solution 2: Use a different device port
meshgrid-cli -p /dev/ttyACM0 info

# Solution 3: Stop debug capture (Ctrl+C) before running commands
```

### PIN Authentication Failed

Verify PIN is correct:

```bash
meshgrid-cli --pin YOUR_PIN info
```

### Firmware Flash Issues

Ensure PlatformIO is installed and device is in bootloader mode:

```bash
pip install platformio
meshgrid-cli flash --detect
```

## Project Structure

The codebase is organized into clean, maintainable modules:

```
src/
├── main.rs              # Entry point + command dispatch (144 lines)
├── cli.rs               # CLI argument definitions (clap structs)
├── commands/            # Command implementations
│   ├── mod.rs           # Module exports + connect_with_auth helper
│   ├── info.rs          # info, stats, neighbors, telemetry
│   ├── messaging.rs     # send, monitor, messages, channels, rotate_identity
│   ├── config.rs        # config command implementations
│   ├── network.rs       # advert, trace, raw, recv
│   ├── system.rs        # reboot, log, flash, ui, mode, time, debug
│   └── util.rs          # ports, require_port
├── device.rs            # Device abstraction layer
├── protocol.rs          # Protocol implementation
├── serial.rs            # Serial port handling
└── ui.rs                # Terminal UI
```

## Development

### Building

```bash
cargo build
cargo test
cargo clippy
```

### Contributing

The codebase follows these principles:

- **Modular design**: Each command in its own module
- **Clean separation**: CLI, protocol, device abstraction are separate
- **Type safety**: Strong typing with Rust's type system
- **Error handling**: Comprehensive error messages with `anyhow`
- **Async I/O**: Tokio for efficient async operations

See `REFACTORING.md` for details on the modular architecture.

### Adding New Commands

1. Add command to `src/cli.rs` in the `Commands` enum
2. Implement in appropriate `src/commands/*.rs` module
3. Export from `src/commands/mod.rs`
4. Add dispatch case in `src/main.rs`
5. Update this README

## Documentation

- Full command reference: `meshgrid-cli --help`
- Per-command help: `meshgrid-cli <command> --help`
- Architecture details: `REFACTORING.md`
- Protocol specification: See firmware repository

## Examples

### Automated Network Monitoring

```bash
#!/bin/bash
# Monitor network and log to file
meshgrid-cli monitor | tee -a mesh-traffic-$(date +%Y%m%d).log
```

### Batch Configuration

```bash
#!/bin/bash
# Configure all connected devices
for port in $(meshgrid-cli ports | grep -o '/dev/tty[^ ]*'); do
    echo "Configuring $port..."
    meshgrid-cli -p $port config preset EU
    meshgrid-cli -p $port config power 17
    meshgrid-cli -p $port config name "node-$(basename $port)"
done
```

### Continuous Telemetry Logging

```bash
# Log telemetry data every 10 seconds
while true; do
    meshgrid-cli telemetry | ts '[%Y-%m-%d %H:%M:%S]' >> telemetry.log
    sleep 10
done
```

## License

MIT

## Related Projects

- [meshgrid-firmware](https://github.com/BetterInc/meshgrid-firmware) - MeshGrid device firmware
- [MeshCore](https://github.com/meshtastic/firmware) - Reference implementation

## Support

- Issues: [GitHub Issues](https://github.com/BetterInc/meshgrid-cli/issues)
- Documentation: See `docs/` folder
- Community: [Discord/Forum link]
