# Examples

Common usage patterns for MeshGrid CLI.

## First Time Setup

```bash
# Check what's connected
meshgrid-cli ports

# Get device info
meshgrid-cli info

# Give it a name
meshgrid-cli config name "garage-node"

# Apply regional preset
meshgrid-cli config preset EU
```

## Send Messages

```bash
# Broadcast to everyone
meshgrid-cli send "Temperature: 22.5C"

# Send to specific node
meshgrid-cli send --to weather-station "get reading"
```

## Monitor Network Activity

```bash
# Watch all traffic
meshgrid-cli monitor

# Check who's nearby
meshgrid-cli neighbors

# Trace route to a node
meshgrid-cli trace remote-node
```

## Check Device Health

```bash
# Quick health check
meshgrid-cli stats | grep -E "Battery|Packets|Neighbors"

# Watch telemetry
meshgrid-cli telemetry --watch

# Check signal quality
meshgrid-cli neighbors | awk '{print $1, $4}'
```

## Configure Radio Settings

```bash
# Longer range, slower speed
meshgrid-cli config sf 12
meshgrid-cli config bw 62.5

# Faster, shorter range
meshgrid-cli config sf 7
meshgrid-cli config bw 250
```

## Scripting

```bash
# Log battery level every hour
while true; do
  meshgrid-cli telemetry | grep Battery >> battery.log
  sleep 3600
done

# Alert on specific messages
meshgrid-cli monitor | grep -i "alert" | while read line; do
  echo "ALERT: $line" | mail -s "Mesh Alert" admin@example.com
done
```

## Multiple Devices

```bash
# Configure three nodes
for port in /dev/ttyUSB{0,1,2}; do
  meshgrid-cli -p $port config name "node-$port"
  meshgrid-cli -p $port config preset EU
done
```
