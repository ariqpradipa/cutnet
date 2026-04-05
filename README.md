# CutNet

**Network Administration Tool for Device Discovery and Control**

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/encore/cutnet)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-orange.svg)](https://github.com/encore/cutnet/releases)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)

CutNet is a modern network administration tool inspired by NetCut. It provides a graphical interface for discovering devices on your network, managing connectivity through ARP-based controls, and protecting against ARP spoofing attacks.

## About

CutNet addresses a specific problem: network administrators need a clear view of every device on their network and the ability to manage those connections without relying on complex command-line tools.

The tool scans your local network using ARP and ping techniques, presents discovered devices in a clean interface, and lets you block or allow specific devices with a single click. It also includes ARP spoofing protection, functioning as a modern replacement for NetCut Defender.

This is a desktop application built with Tauri 2, combining a lightweight Rust backend for raw network operations with a responsive React frontend.

## Features

### Device Discovery

- **ARP Scanning**: Discover all devices on your local network via ARP tables
- **Ping Sweeping**: Verify device responsiveness with ICMP ping
- **Real-time Updates**: Monitor device status changes as they happen
- **MAC Address Lookup**: Identify device manufacturers from MAC addresses

### Device Management

- **Block/Unblock Devices**: Control network access with ARP poisoning techniques
- **Custom Naming**: Assign friendly names to devices for easy identification
- **Whitelist Management**: Create protected device lists that cannot be blocked
- **Session History**: Log all device connections and block actions with timestamps

### Security

- **ARP Spoof Protection**: Detect and block ARP spoofing attacks on your network
- **Protection Mode**: Enable automatic defense against malicious ARP injections
- **Whitelist Shield**: Protected devices are immune to accidental or malicious blocks

### User Interface

- **Dark/Light Mode**: Toggle between themes or follow system preferences
- **Responsive Design**: Native-feeling application with platform-appropriate styling
- **Real-time Status**: Live device status indicators and connection monitoring
- **Search and Filter**: Quickly locate devices in large network environments

## Screenshots

> Screenshots coming soon. The application features a clean, modern interface with device cards showing IP address, MAC address, manufacturer, and connection status.

## How It Works

CutNet operates at the data link layer (Layer 2) of the network stack, working directly with MAC addresses and ARP (Address Resolution Protocol) tables.

### ARP Basics

Every device on a local network has two addresses: an IP address (Layer 3) and a MAC address (Layer 2). When devices communicate, they need to map IP addresses to MAC addresses. This mapping is stored in the ARP table.

ARP works like a phone book:
1. Device A wants to send data to Device B (known by IP address)
2. Device A broadcasts an ARP request: "Who has IP 192.168.1.100?"
3. Device B responds with its MAC address
4. Device A now knows where to send data

### ARP Poisoning

ARP poisoning (also called ARP spoofing) exploits the trust devices place in ARP responses. When you block a device with CutNet:

1. CutNet sends a spoofed ARP response to the gateway claiming the target device's IP maps to a fake MAC address
2. The gateway updates its ARP table with this false mapping
3. Traffic intended for the target device goes nowhere, effectively blocking its network access

This technique works because ARP has no built-in authentication mechanism. Devices accept ARP responses without verification.

### Network Protection

CutNet's protection mode works in reverse:

1. CutNet monitors incoming ARP responses on the network
2. If it detects an ARP entry that conflicts with your whitelist, it blocks the spoofed response
3. Your device's ARP table remains correct, preventing man-in-the-middle attacks

## Installation

### Pre-built Releases

Download the latest release for your platform from the [Releases page](https://github.com/encore/cutnet/releases).

| Platform | Installer | Notes |
|----------|-----------|-------|
| Windows | `.exe` installer, `.msi` | Requires Npcap driver |
| macOS | `.dmg` | Native BSD sockets, no extra drivers |
| Linux | `.deb`, `.rpm`, `.AppImage` | Requires raw socket permissions |

### macOS

1. Download `CutNet-x.x.x.dmg` from the releases page
2. Open the DMG file
3. Drag CutNet to your Applications folder
4. On first launch, right-click and select "Open" to bypass Gatekeeper restrictions
5. Grant administrative privileges when prompted (required for network operations)

### Windows

1. Download `CutNet-x.x.x.exe` or `CutNet-x.x.x.msi`
2. Run the installer
3. Install Npcap when prompted (required for raw socket access)
   - Download from [npcap.com](https://npcap.com/) if not included
   - Ensure "WinPcap API-compatible Mode" is enabled during installation
4. Launch CutNet with administrator privileges

### Linux

#### AppImage (Recommended)

```bash
chmod +x CutNet-x.x.x.AppImage
./CutNet-x.x.x.AppImage
```

#### Debian/RPM

```bash
# Debian/Ubuntu
sudo dpkg -i cutnet-x.x.x.deb
sudo apt-get install -f  # Install dependencies if needed

# Fedora/RHEL
sudo rpm -i cutnet-x.x.x.rpm
```

#### Post-Installation Setup

On Linux, you need to grant raw socket capabilities:

```bash
sudo setcap 'cap_net_raw,cap_net_admin=eip' /usr/bin/cutnet
```

Or run as root (not recommended for daily use):

```bash
sudo cutnet
```

## Development

### Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| Node.js | 18+ | LTS recommended |
| Bun | 1.0+ | Preferred package manager |
| Rust | 1.70+ | Required for Tauri backend |
| Cargo | latest | Comes with Rust |

Platform-specific requirements:

- **macOS**: Xcode Command Line Tools
- **Windows**: Visual Studio Build Tools, Npcap SDK
- **Linux**: `libssl-dev`, `pkg-config`, `build-essential`

### Setup

1. Clone the repository:

```bash
git clone https://github.com/encore/cutnet.git
cd cutnet
```

2. Install frontend dependencies:

```bash
bun install
```

3. Verify the development environment:

```bash
bun run tauri dev
```

This starts the Vite development server and compiles the Rust backend. The application window opens automatically when ready.

### Available Commands

```bash
# Development
bun run dev          # Start Vite dev server only
bun run tauri dev    # Start full Tauri development environment

# Build
bun run build        # Build frontend only
bun run tauri build  # Build complete application for distribution

# Frontend Tools
bun run tauri lint   # Lint frontend code
npx tsc --noEmit     # Type check TypeScript
```

### Project Structure

```
cutnet/
├── src/                    # React frontend source
│   ├── components/         # React components
│   ├── hooks/              # Custom React hooks
│   ├── lib/                # Utilities and helpers
│   ├── stores/             # Zustand state stores
│   └── styles/             # Global styles
├── src-tauri/              # Rust backend source
│   ├── src/
│   │   ├── main.rs         # Application entry point
│   │   ├── lib.rs          # Library exports
│   │   └── commands/       # Tauri commands
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
├── package.json
└── vite.config.ts
```

## Platform-Specific Notes

### macOS

CutNet uses native BSD sockets on macOS, eliminating the need for Npcap or WinPcap. The application requires administrator privileges to open raw sockets for network scanning and ARP manipulation.

On macOS 10.15 (Catalina) and later, the application is sandboxed by default. Grant full disk access in System Preferences > Security & Privacy > Privacy > Full Disk Access if scanning fails to discover all devices.

### Windows

Windows requires the Npcap driver for raw socket operations. During installation, ensure the Npcap installer runs and configures WinPcap API compatibility.

If you encounter permission errors:

1. Right-click CutNet in the Start menu
2. Select "Run as administrator"
3. Confirm the UAC prompt

Npcap must be installed separately if not bundled with the installer.

### Linux

Linux raw socket access requires either:

1. **setcap** (Recommended):
   ```bash
   sudo setcap 'cap_net_raw,cap_net_admin=eip' /path/to/cutnet
   ```

2. **sudo** (Not recommended for regular use):
   ```bash
   sudo /path/to/cutnet
   ```

Firewall tools like `ufw` may interfere with ARP operations. Consider adding rules to allow traffic on the relevant interfaces.

## Usage Guide

### First Launch

On first launch, CutNet requests administrator privileges. This is required for raw socket operations. Grant access to proceed.

The main window shows your network overview and begins scanning automatically.

### Scanning for Devices

1. Click the "Scan" button or press `Ctrl+R` / `Cmd+R`
2. Wait for the scan to complete (typically 5-30 seconds depending on network size)
3. View discovered devices in the device list

Devices are sorted by IP address by default. Click column headers to sort by name, MAC address, or status.

### Blocking a Device

1. Select the device from the list
2. Click the "Block" button
3. Confirm the action when prompted
4. The device status changes to "Blocked"

Blocked devices cannot access the internet or local network resources through the gateway. This effect persists until you unblock the device or restart the network service.

### Unblocking a Device

1. Select the blocked device (filter by "Blocked" status if needed)
2. Click the "Unblock" button
3. The device regains network access immediately

### Managing the Whitelist

1. Go to Settings > Whitelist
2. Click "Add Device"
3. Select a device from your discovered list or enter MAC/IP manually
4. Enable "Protection Mode" to auto-block ARP spoofing attempts

Whitelisted devices display a shield icon and cannot be blocked accidentally.

### Custom Device Names

1. Click the edit icon next to any device name
2. Enter a friendly name (e.g., "Living Room TV")
3. Press Enter or click outside to save

Custom names persist across sessions.

### Enabling Protection Mode

1. Go to Settings > Security
2. Toggle "Protection Mode" on
3. Select devices to protect from your whitelist
4. CutNet monitors ARP traffic and blocks spoofing attempts automatically

## Troubleshooting

### "No devices found" or empty device list

1. Verify you have administrator privileges
2. Check that your network connection is active
3. Disable VPN if enabled (VPNs can prevent local network scanning)
4. On Windows, ensure Npcap is installed and running

### Scan completes but some devices missing

1. Some devices ignore ARP requests by default (notably some IoT devices)
2. Try disabling the device's firewall temporarily
3. Ensure the device is on the same subnet as your machine

### "Permission denied" errors

- **macOS**: Grant Full Disk Access in System Preferences
- **Windows**: Run as administrator
- **Linux**: Run `sudo setcap` as described above

### Application crashes on launch

1. Check the log files in `~/.cutnet/logs/` (Linux/macOS) or `%APPDATA%\CutNet\logs\` (Windows)
2. Report the issue on GitHub with log contents
3. Try reinstalling the application

### High CPU usage during scans

Scanning is CPU-intensive by design, especially on large networks. Consider reducing scan frequency in Settings.

### Blocked device still has network access

Some devices may have alternative routing paths. CutNet specifically blocks traffic through the default gateway, but devices with static routes or peer-to-peer connections may remain reachable.

## Security and Privacy

### Data Collection

CutNet operates entirely locally:

- No data is sent to external servers
- No usage analytics or telemetry
- Network scan results are stored locally in your user directory
- Session logs remain on your machine

### Local Storage

Data is stored in platform-specific directories:

- **macOS**: `~/Library/Application Support/CutNet/`
- **Windows**: `%APPDATA%\CutNet\`
- **Linux**: `~/.config/cutnet/`

Stored data includes:

- Device discovery cache
- Custom device names
- Whitelist entries
- Session history logs

### Network Operations

CutNet only manipulates ARP tables on your local network segment. It cannot:

- Access devices outside your local network
- Intercept or read network traffic content
- Exfiltrate data from your machines
- Be used as a remote access trojan

The tool is designed for legitimate network administration on networks you own or manage.

## Contributing

Contributions are welcome. Before submitting pull requests, review these guidelines.

### Code Style

- **TypeScript**: Follow the existing formatting. Run `npx tsc --noEmit` before committing.
- **Rust**: Run `cargo fmt` before committing. Follow Rust idioms and ownership patterns.
- **CSS**: Use TailwindCSS utility classes. Avoid inline styles.

### Commit Messages

Use conventional commit format:

```
feat: add device sorting by name
fix: resolve scan timeout on large networks
docs: update installation instructions for Linux
```

### Pull Request Process

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes with clear, focused commits
4. Ensure all checks pass (type checking, linting, build)
5. Submit a pull request with a clear description of changes

### Reporting Issues

Include the following in bug reports:

- Platform and OS version
- Steps to reproduce
- Expected vs actual behavior
- Log files (if applicable)

Feature requests are welcome. Open an issue to discuss before implementing major changes.

## Legal Disclaimer

**CutNet is intended for legitimate network administration only.**

Using this tool on networks you do not own or have explicit written permission to manage may violate:

- Local, state, national, or international laws
- Computer fraud and abuse laws
- Terms of service agreements with your ISP or network provider

The authors of CutNet are not responsible for misuse of this software. You are solely responsible for ensuring your use complies with all applicable laws and regulations.

Network administration tools of this type are commonly used by:

- IT administrators managing corporate networks
- Security professionals conducting authorized assessments
- Home users managing their own home networks
- Penetration testers with explicit written authorization

If you are uncertain whether your intended use is legal, consult a legal professional before proceeding.

## License

CutNet is released under the [MIT License](LICENSE).

Copyright (c) 2024 Encore

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Acknowledgments

CutNet draws inspiration from:

- **NetCut** (by Amanda Make) for pioneering accessible ARP-based network management
- **NetCut Defender** for demonstrating personal network protection

This project stands on the shoulders of open-source giants:

- **Tauri** for the efficient desktop framework
- **React** and the React team for the UI foundation
- **Rust** and the Rust team for safe systems programming
- **TailwindCSS** for utility-first CSS
- **shadcn/ui** for accessible component patterns
- All contributors to the dependencies that make this project possible

---

Built with [Tauri 2](https://tauri.app/) | [React 19](https://react.dev/) | [Rust](https://www.rust-lang.org/)
