# arkv

A fast, no-frills file archiving tool that copies files to remote servers via SFTP.

## Features

- 🚀 Fast SFTP transfers using SSH key or password authentication
- 📁 Supports both individual files and entire directories
- 📊 Real-time progress indicators with spinners and progress bars
- 🔧 Simple one-time setup
- 💾 Supports multiple remote destinations

## Installation

Build from source:

```bash
cargo build --release
```

The binary will be at `target/release/arkv`. You can copy it to your PATH:

```bash
cp target/release/arkv /usr/local/bin/
```

## Quick Start

First time setup:

```bash
arkv --setup
```

This will guide you through:
1. Locating your SSH key (default: `~/.ssh/id_ed25519`)
2. Adding remote destinations (name, host, username, remote path, optional password)

## Usage

Upload a file:
```bash
arkv cool-picture.png
```

Upload a folder:
```bash
arkv my_files/tuesday/
```

Choose destination interactively (when multiple destinations are configured):
```bash
arkv document.pdf --interactive
```

Re-run setup:
```bash
arkv --setup
```

Show help:
```bash
arkv --help
```

## Configuration

Configuration is stored at `~/.config/arkv/config.toml`

Example config:
```toml
ssh_key_path = "/Users/username/.ssh/id_ed25519"

[[destinations]]
name = "production"
host = "example.com"
username = "deploy"
remote_path = "/var/www/uploads"

[[destinations]]
name = "backup"
host = "192.168.1.100"
username = "user"
remote_path = "/home/user/backups"
password = "optional_password"
```

## How It Works

1. Connects to remote server via SSH (port 22)
2. Uses SFTP protocol for file transfers
3. Automatically creates remote directories if they don't exist
4. Preserves folder structure when uploading directories
5. Shows progress with spinners (single files) or progress bars (folders)

## License

MIT
