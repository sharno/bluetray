# BlueTray

A Windows system tray application for managing Bluetooth device connections.

## Overview

BlueTray allows you to quickly connect to paired Bluetooth devices directly from your system tray, without having to open Windows Bluetooth settings.

## Features

- System tray icon for easy access
- Connect to paired Bluetooth devices with a single click
- Automatic connection management
- Minimal resource usage

## Requirements

- Windows 10 or newer
- Rust compiler (for building from source)

## Building from Source

1. Clone the repository
   ```
   git clone https://github.com/yourusername/bluetray.git
   cd bluetray
   ```

2. Build with Cargo
   ```
   cargo build --release
   ```

3. The compiled binary will be in `target/release/bluetray.exe`

## Usage

Simply run the application. A tray icon will appear in your system tray. Click on the icon to see a list of your paired Bluetooth devices. Click on any device to connect to it.

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 