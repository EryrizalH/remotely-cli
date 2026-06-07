# Teleprompt CLI (`teleprompt`)

<div align="center">
  <p><strong>A secure, agent-friendly remote execution and connection manager for SSH & Telnet.</strong></p>
  <p>
    <a href="https://www.npmjs.com/package/teleprompt-cli"><img src="https://img.shields.io/npm/v/teleprompt-cli.svg?style=flat-square&color=blue" alt="NPM Version" /></a>
    <a href="https://github.com/EryrizalH/teleprompt-cli/actions"><img src="https://img.shields.io/github/actions/workflow/status/EryrizalH/teleprompt-cli/release.yml?branch=main&style=flat-square" alt="Build Status" /></a>
    <a href="https://github.com/EryrizalH/teleprompt-cli/blob/main/LICENSE"><img src="https://img.shields.io/github/license/EryrizalH/teleprompt-cli.svg?style=flat-square" alt="License" /></a>
  </p>
</div>

---

**Teleprompt CLI** enables AI agents (and human developers) to control multiple servers, switches, and remote devices via SSH or Telnet without exposing plain-text credentials, SSH keys, or passwords in terminal histories, shell scripts, or agent prompts. 

By keeping credentials encrypted on-disk and using automatic prompt interception, Teleprompt CLI bridges the gap between powerful automation and strict security.

---

## Key Features

- 🔒 **Zero-Exposure Credentials**: Device passwords and private keys are encrypted locally using AES-256-GCM (Key derivation via Argon2id) in a local SQLite database (`~/.teleprompt/credentials.enc`).
- 🔑 **Bypass Interactive Prompts**: Set the `TELEPROMPT_KEY` environment variable in your agent's context. Once defined, `teleprompt` retrieves credentials and connects without prompting for passwords.
- ⚡ **Auto Sudo Handling**: Automatically detects remote `sudo` password prompts and securely injects the registered sudo password.
- 🌐 **Dual Protocol Support**: Connects to modern servers via **SSH** (supports password & private key authentication) or legacy switches/routers via **Telnet**.
- 🤖 **Optimized for AI Tool-Calling**: Native exit code forwarding and separation of stdout/stderr make it easy for LLMs to parse outcomes and react programmatically.

---

## Installation

### Via NPM (Recommended)
Install globally to make the `teleprompt` command available system-wide:
```bash
npm install -g teleprompt-cli
```
*The installer will automatically download the correct precompiled native binary for your OS and architecture. If a precompiled binary isn't available, it will automatically attempt to compile it from source via cargo.*

### Via Cargo (Rust toolchain)
If you prefer to compile manually from source using Cargo:
```bash
cargo install --git https://github.com/EryrizalH/teleprompt-cli.git
```

---

## Quick Start

### 1. Initialize the Encrypted Store
Create the database and set your Master Password:
```bash
teleprompt init
```
*This creates the database at `~/.teleprompt/credentials.enc`. You will be prompted to enter a Master Password. Remember this password, as it is required to decrypt the store.*

### 2. Set the Master Password Environment Variable
To run commands non-interactively (ideal for shell scripts and AI agents), export your master password:

**Linux / macOS:**
```bash
export TELEPROMPT_KEY="your-master-password"
```

**Windows PowerShell:**
```powershell
$env:TELEPROMPT_KEY="your-master-password"
```

### 3. Add a Remote Device
Register a device interactively. The connection is tested automatically before saving:
```bash
teleprompt add
```
*You will be prompted for device details (name, host, port, username, connection type: ssh/telnet, and credentials).*

### 4. List Registered Devices
Review your configured devices (passwords and sensitive keys are masked):
```bash
teleprompt list
```

### 5. Execute Remote Commands
Execute command(s) on a registered device:
```bash
# Simple single command
teleprompt my-server-name uname -a

# Multi-stage or piped commands (use quotes)
teleprompt my-server-name "cd /var/log && cat syslog | tail -n 20"
```

---

## Command Reference

| Command | Description |
|:---|:---|
| `teleprompt init` | Set up the secure encrypted credential database. |
| `teleprompt add` | Add a new device configuration (SSH or Telnet) interactively. |
| `teleprompt list` | Display a table of all registered devices. |
| `teleprompt remove <name>` | Delete a device from the database. |
| `teleprompt edit <name>` | Interactively edit the configuration of a device. |
| `teleprompt test <name>` | Verify connection settings and credentials for a device. |
| `teleprompt <name> <command...>` | Run a command on the target device and return the output. |

### Global Options

- `--timeout <seconds>`: Override the execution timeout (default: `30` seconds).
- `--db-path <path>`: Specify a custom path to the encrypted database (default: `~/.teleprompt/credentials.enc`).

Example:
```bash
teleprompt --timeout 60 --db-path ./custom.db my-device "df -h"
```

---

## Security & Mechanics

### Auto-Sudo Prompt Interception
When executing remote commands, `teleprompt` monitors the terminal streams. If it detects a password prompt (e.g. `[sudo] password for ...:`), it automatically inputs the stored sudo password (or SSH/Telnet password if a separate sudo password isn't set) and presses Enter. This prevents interactive hangs in automation scripts or AI sessions.

### Encryption Standard
- **Key Derivation**: Argon2id is used to derive a key from your Master Password.
- **Cipher**: AES-256-GCM is used to encrypt and decrypt the credential store on read/write.
- **Memory Safety**: Sensitive keys are wiped from process memory when no longer needed using the `zeroize` crate.

---

## Guidelines for AI Agent Integration

If you are equipping an AI agent (e.g., Claude Code, ChatGPT, Gemini) with this tool, add the following to the agent's instructions:

> [!TIP]
> **AI Agent Instructions for `teleprompt` usage:**
> - To check configured servers, run `teleprompt list`.
> - Ensure the `TELEPROMPT_KEY` environment variable is loaded in your execution context before running commands.
> - When running commands with pipes (`|`), redirects (`>`), or multiple instructions, group them in a single string, e.g., `teleprompt server-name "command1 && command2 | grep foo"`.
> - Check exit codes: a non-zero exit code represents a remote command failure or connection loss.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
