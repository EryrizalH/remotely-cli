# Skill: remotely-cli usage for AI Agents

Learn how to use the `remotely` CLI tool to run shell commands on remote servers and routers securely without exposing passwords or private keys.

## When to use `remotely`
Use this tool whenever you need to execute shell commands, retrieve system status, or automate operations on a remote device (via SSH or Telnet) registered in the credential store.

## Security Rules
1. **Never Output Passwords**: When using `remotely`, passwords and credentials are kept encrypted on disk. You do not need to read, write, or prompt for device passwords.
2. **Master Key**: To execute commands without interactive prompts, ensure the `REMOTELY_KEY` environment variable is set. If it is not set, commands will fail or wait for human input, which will cause your execution to hang.
3. **Execution exit codes**: The exit code returned by `remotely <device> <command>` matches the remote process's exit code. Always check the exit code to determine if your remote command succeeded.

## CLI Usage Patterns

### 1. List Registered Devices
To see what devices are available:
```bash
remotely list
```
*Output is printed as an ASCII table showing device name, host, port, user, and protocol type.*

### 2. Check Device Connectivity
To check if a specific device is online and credentials are correct:
```bash
remotely test <device_name>
```

### 3. Run Remote Commands
To execute a command on a remote device:
```bash
remotely <device_name> <command...>
```

#### Examples:
- Get interface details:
  ```bash
  remotely server1 ifconfig
  ```
- Run chained shell commands (use quotes):
  ```bash
  remotely server1 "cd /var/log && cat syslog | tail -n 20"
  ```
- Check disk space:
  ```bash
  remotely database-srv df -h
  ```

### 4. Execute Sudo Commands
If a device has sudo privileges configured, you can run sudo commands directly:
```bash
remotely server1 sudo systemctl restart nginx
```
*Note: `remotely` automatically intercepts the sudo password prompt and injects the password securely without exposing it in the terminal.*
