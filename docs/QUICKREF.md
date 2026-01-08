# Quick Reference

## Radio Settings

### Spreading Factor vs Range

| SF | Range | Speed | Battery |
|----|-------|-------|---------|
| 7 | Short | Fast | Low |
| 8-9 | Medium | Medium | Medium |
| 10-11 | Long | Slow | High |
| 12 | Maximum | Slowest | Highest |

### Bandwidth

| BW (kHz) | Range | Sensitivity |
|----------|-------|-------------|
| 500 | Short | Low |
| 250 | Medium | Medium |
| 125 | Long | Good |
| 62.5 | Longer | Better |

### Signal Quality

| RSSI (dBm) | Quality |
|------------|---------|
| > -80 | Excellent |
| -80 to -90 | Good |
| -90 to -100 | Fair |
| -100 to -110 | Poor |
| < -110 | Very Poor |

## TX Power Guidelines

| Power (dBm) | Use Case |
|-------------|----------|
| 2-5 | Indoor, very close range |
| 10-14 | General use |
| 17-20 | Long range outdoors |
| 22+ | Maximum range (check local regulations) |

## Common Port Names

**Linux**: `/dev/ttyUSB0`, `/dev/ttyACM0`
**macOS**: `/dev/cu.usbserial-*`, `/dev/cu.usbmodem*`
**Windows**: `COM3`, `COM4`, etc.

## Device Modes

- **client** - Standard node (sends and receives)
- **repeater** - Relay only (doesn't generate traffic)
- **room** - Location/room node

## Regional Presets

- **EU** - 868 MHz (Europe)
- **US** - 915 MHz (North America)
- **US_FAST** - 915 MHz, optimized for speed
- **LONG_RANGE** - Maximum range configuration

## Exit Codes

- `0` - Success
- `1` - Error
- `130` - Interrupted (Ctrl+C)
