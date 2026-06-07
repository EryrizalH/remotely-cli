use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::credentials::{Device, ConnectionType};
use crate::error::TelepromptError;

pub fn execute_command(
    device: &Device,
    command: &str,
    timeout_secs: u64,
) -> Result<(i32, Vec<u8>, Vec<u8>), TelepromptError> {
    if device.connection_type != ConnectionType::Ssh {
        return Err(TelepromptError::Other("Device is not configured for SSH".to_string()));
    }

    // Connect to host
    let addr = format!("{}:{}", device.host, device.port);
    let stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| TelepromptError::ConnectionFailed(addr.clone(), format!("{}", e)))?,
        Duration::from_secs(timeout_secs),
    ).map_err(|e| TelepromptError::ConnectionFailed(addr.clone(), e.to_string()))?;

    let mut sess = Session::new()
        .map_err(|e| TelepromptError::ConnectionFailed(addr.clone(), e.to_string()))?;
    sess.set_tcp_stream(stream);
    sess.set_timeout(timeout_secs as u32 * 1000);
    sess.handshake()
        .map_err(|e| TelepromptError::ConnectionFailed(addr.clone(), e.to_string()))?;

    // Authenticate
    authenticate_session(&mut sess, device, &addr)?;

    let mut channel = sess.channel_session()
        .map_err(|e| TelepromptError::Other(format!("Failed to open channel: {}", e)))?;

    let is_sudo = command.trim().starts_with("sudo ") || command.trim() == "sudo";
    
    if is_sudo {
        // Sudo requires a PTY to receive the password
        channel.request_pty("vanilla", None, None)
            .map_err(|e| TelepromptError::Other(format!("Failed to request PTY for sudo: {}", e)))?;
    }

    channel.exec(command)
        .map_err(|e| TelepromptError::Other(format!("Failed to execute command: {}", e)))?;

    sess.set_blocking(false);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut stdout_closed = false;
    let mut stderr_closed = false;

    let start_time = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    enum SudoState {
        CheckingPrompt,
        PasswordSent,
        Disabled,
    }

    let mut sudo_state = if is_sudo && device.sudo_password_required {
        SudoState::CheckingPrompt
    } else {
        SudoState::Disabled
    };

    let mut initial_prompts = 0;

    while !stdout_closed || !stderr_closed {
        if start_time.elapsed() > timeout {
            return Err(TelepromptError::Timeout(timeout_secs));
        }

        // Read stdout
        if !stdout_closed {
            let mut buf = [0u8; 1024];
            match channel.read(&mut buf) {
                Ok(0) => stdout_closed = true,
                Ok(n) => {
                    stdout.extend_from_slice(&buf[..n]);
                    
                    // Handle sudo prompt detection
                    if let SudoState::CheckingPrompt = sudo_state {
                        let stdout_str = String::from_utf8_lossy(&stdout);
                        if contains_sudo_prompt(&stdout_str) {
                            if let Some(ref pwd) = device.password {
                                initial_prompts = count_sudo_prompts(&stdout_str);
                                // Write password to stdin
                                sess.set_blocking(true);
                                if let Err(e) = channel.write_all(format!("{}\n", pwd).as_bytes()) {
                                    return Err(TelepromptError::SudoFailed(e.to_string()));
                                }
                                let _ = channel.flush();
                                sess.set_blocking(false);
                                sudo_state = SudoState::PasswordSent;
                            } else {
                                return Err(TelepromptError::SudoFailed("Sudo requires a password, but none is saved".to_string()));
                            }
                        }
                    } else if let SudoState::PasswordSent = sudo_state {
                        // Check if password prompt appears again (implies incorrect password)
                        let stdout_str = String::from_utf8_lossy(&stdout);
                        // Find the prompt after the first password sent
                        if count_sudo_prompts(&stdout_str) > initial_prompts {
                            return Err(TelepromptError::SudoFailed("Incorrect password".to_string()));
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(TelepromptError::Io(e)),
            }
        }

        // Read stderr
        if !stderr_closed {
            let mut buf = [0u8; 1024];
            match channel.stderr().read(&mut buf) {
                Ok(0) => stderr_closed = true,
                Ok(n) => stderr.extend_from_slice(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(TelepromptError::Io(e)),
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    sess.set_blocking(true);
    let _ = channel.wait_eof();
    let _ = channel.close();
    let _ = channel.wait_close();

    let exit_status = channel.exit_status()
        .map_err(|e| TelepromptError::Other(format!("Failed to retrieve exit status: {}", e)))?;

    // If PTY was used, remove the password prompt and the echo of the password from stdout to keep it clean
    if is_sudo {
        stdout = clean_sudo_output(stdout, device.password.as_deref().unwrap_or(""));
    }

    Ok((exit_status, stdout, stderr))
}

pub fn test_connection(device: &Device, timeout_secs: u64) -> Result<(), TelepromptError> {
    // We execute a simple echo command to test connectivity
    let (status, _, _) = execute_command(device, "echo 'teleprompt_ok'", timeout_secs)?;
    if status == 0 {
        Ok(())
    } else {
        Err(TelepromptError::Other(format!("Test command exited with code {}", status)))
    }
}

pub fn detect_sudo_capability(device: &mut Device, timeout_secs: u64) -> Result<(), TelepromptError> {
    // 1. Check if sudo is installed and if we can run without password
    // We run `sudo -n true`
    let (status_no_pwd, _, _) = execute_command(device, "sudo -n true", timeout_secs)?;
    if status_no_pwd == 0 {
        device.sudo_capable = true;
        device.sudo_password_required = false;
        return Ok(());
    }

    // 2. Check if we can run with password
    // We try to run `sudo -S true` (this will prompt for password)
    device.sudo_capable = true;
    device.sudo_password_required = true;
    let (status_pwd, _, _) = execute_command(device, "sudo -S true", timeout_secs)?;
    if status_pwd == 0 {
        // Sudo works with password!
        Ok(())
    } else {
        // Sudo failed
        device.sudo_capable = false;
        device.sudo_password_required = false;
        Ok(())
    }
}

fn authenticate_session(
    sess: &mut Session,
    device: &Device,
    addr: &str,
) -> Result<(), TelepromptError> {
    // 1. Try public key if key_path is specified
    if let Some(ref key_path) = device.key_path {
        let path = Path::new(key_path);
        if path.exists() {
            if let Err(_e) = sess.userauth_pubkey_file(
                &device.username,
                None,
                path,
                None, // We can expand to passphrases later
            ) {
                // If pubkey failed and no password is saved, return auth error
                if device.password.is_none() {
                    return Err(TelepromptError::AuthFailed(device.username.clone(), addr.to_string()));
                }
            } else {
                return Ok(()); // Key auth succeeded
            }
        } else if device.password.is_none() {
            return Err(TelepromptError::Other(format!("SSH key file not found at: {}", key_path)));
        }
    }

    // 2. Try password auth
    if let Some(ref password) = device.password {
        sess.userauth_password(&device.username, password)
            .map_err(|_| TelepromptError::AuthFailed(device.username.clone(), addr.to_string()))?;
        Ok(())
    } else {
        Err(TelepromptError::Other("No credentials (password or valid key) provided for SSH auth".to_string()))
    }
}

fn contains_sudo_prompt(stdout: &str) -> bool {
    let lower = stdout.to_lowercase();
    lower.contains("password for") || lower.contains("[sudo]") || lower.contains("password:")
}

fn count_sudo_prompts(stdout: &str) -> usize {
    let lower = stdout.to_lowercase();
    let mut count = 0;
    for pattern in &["password for", "[sudo]", "password:"] {
        count += lower.matches(pattern).count();
    }
    count
}

fn clean_sudo_output(stdout: Vec<u8>, password: &str) -> Vec<u8> {
    let stdout_str = String::from_utf8_lossy(&stdout);
    let lines = stdout_str.lines().collect::<Vec<&str>>();
    
    // Sudo with PTY will echo the prompt and the password typed (or stars, or just newline)
    // E.g.:
    // [sudo] password for user:
    // (password typed)
    // <actual command output>
    
    // We filter out lines that contain sudo prompt or match the password
    let mut filtered_lines = Vec::new();
    let mut prompt_passed = false;
    
    for line in lines {
        let lower = line.to_lowercase();
        let is_prompt = lower.contains("[sudo]") || lower.contains("password for") || lower.contains("password:");
        let is_password = !password.is_empty() && (line.trim() == password.trim() || line.trim() == "");
        
        if is_prompt {
            continue;
        }
        if !prompt_passed && is_password {
            // Skip the first empty line/password echo right after prompt
            continue;
        }
        
        // Once we hit non-prompt/non-password-echo, everything else is output
        prompt_passed = true;
        filtered_lines.push(line);
    }
    
    filtered_lines.join("\n").into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_sudo_prompt() {
        assert!(contains_sudo_prompt("[sudo] password for user:"));
        assert!(contains_sudo_prompt("password:"));
        assert!(contains_sudo_prompt("Password for root:"));
        assert!(!contains_sudo_prompt("access granted"));
        assert!(!contains_sudo_prompt(""));
    }

    #[test]
    fn test_count_sudo_prompts() {
        assert_eq!(count_sudo_prompts("[sudo] password for user:"), 2); // Matches [sudo] and password for
        assert_eq!(count_sudo_prompts("password:"), 1);
        assert_eq!(count_sudo_prompts("[sudo]\npassword:"), 2);
        assert_eq!(count_sudo_prompts("normal command output"), 0);
    }

    #[test]
    fn test_clean_sudo_output() {
        let password = "my_password";
        
        // Output from command with sudo prompting
        let raw_output = b"[sudo] password for admin:\n\nmy_password\nHello World\nSuccess".to_vec();
        let cleaned = clean_sudo_output(raw_output, password);
        assert_eq!(String::from_utf8_lossy(&cleaned), "Hello World\nSuccess");

        // Without password prompting (no [sudo] in prompt or password matches)
        let raw_output_normal = b"Hello World\nSuccess".to_vec();
        let cleaned_normal = clean_sudo_output(raw_output_normal, password);
        assert_eq!(String::from_utf8_lossy(&cleaned_normal), "Hello World\nSuccess");

        // Only sudo prompt and password echo, empty actual output
        let raw_output_empty = b"[sudo] password for admin:\nmy_password\n".to_vec();
        let cleaned_empty = clean_sudo_output(raw_output_empty, password);
        assert_eq!(String::from_utf8_lossy(&cleaned_empty), "");
    }
}

