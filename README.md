# Remotely CLI (`remotely`)

A secure, agent-friendly remote execution and connection manager for SSH and Telnet, designed specifically to enable AI agents (and human developers) to control multiple servers/devices without having plain-text passwords or keys exposed in scripts, command histories, or agent prompts.

## Features
- **Zero Exposed Credentials**: Passwords and keys are kept in an encrypted credential store on disk (`~/.remotely/credentials.enc`).
- **One-time Master Password**: Provide your master key via the `REMOTELY_KEY` environment variable or via terminal prompts. Once set, commands bypass password prompts.
- **Auto Sudo Handling**: Remotely detects if a device supports `sudo` and automatically inputs the sudo password securely when required.
- **SSH & Telnet Support**: Connects to modern servers via SSH (supporting passwords and private keys) or legacy switches/routers via Telnet.
- **Agent-optimized**: Clean separation of stdout/stderr and correct exit codes, making it perfect for tool calling.

## Installation
Ensure you have Rust/Cargo installed:
```bash
cargo install --path .
```

## Quick Start
1. **Initialize the Credential Store**:
   ```bash
   remotely init
   ```
   *This prompts you for a Master Password to encrypt your storage.*

2. **Set the environment variable** to avoid interactive prompts in scripts/agents:
   ```bash
   # Linux/macOS
   export REMOTELY_KEY="your-master-password"
   
   # Windows PowerShell
   $env:REMOTELY_KEY="your-master-password"
   ```

3. **Register a Device**:
   ```bash
   remotely add
   ```
   Follow the prompts to add details: name, IP, port, username, password/SSH key, etc. The connection will be tested automatically.

4. **List Registered Devices**:
   ```bash
   remotely list
   ```

5. **Run a Command**:
   ```bash
   remotely deviceA ifconfig
   remotely deviceA "cd /var/log && cat auth.log | tail -n 20"
   ```

6. **Run Sudo Command**:
   ```bash
   remotely deviceA sudo service nginx restart
   ```

## Commands
- `remotely init`: Set up the secure credential store.
- `remotely add`: Interactively add a new device configuration.
- `remotely list`: Table view of all registered devices (passwords masked).
- `remotely remove <device_name>`: Remove a device.
- `remotely edit <device_name>`: Modify an existing device.
- `remotely test <device_name>`: Verify connectivity and credentials.
- `remotely <device_name> <command...>`: Execute a remote command.
