use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::credentials::{Device, ConnectionType};
use crate::error::RemotelyError;

// Telnet Command Codes (RFC 854)
const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;

pub fn execute_command(
    device: &Device,
    command: &str,
    timeout_secs: u64,
) -> Result<(i32, Vec<u8>, Vec<u8>), RemotelyError> {
    if device.connection_type != ConnectionType::Telnet {
        return Err(RemotelyError::Other("Device is not configured for Telnet".to_string()));
    }

    let addr = format!("{}:{}", device.host, device.port);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| RemotelyError::ConnectionFailed(addr.clone(), format!("{}", e)))?,
        Duration::from_secs(timeout_secs),
    ).map_err(|e| RemotelyError::ConnectionFailed(addr.clone(), e.to_string()))?;

    stream.set_read_timeout(Some(Duration::from_secs(timeout_secs)))
        .map_err(|e| RemotelyError::Io(e))?;
    stream.set_write_timeout(Some(Duration::from_secs(timeout_secs)))
        .map_err(|e| RemotelyError::Io(e))?;

    let start_time = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    // 1. Read login prompt
    let mut buffer = Vec::new();
    let username = &device.username;
    let password = device.password.as_deref().unwrap_or("");

    wait_for_prompts(&mut stream, &mut buffer, &["login:", "username:", "user:"], start_time, timeout)?;
    
    // Send username
    stream.write_all(format!("{}\r\n", username).as_bytes())
        .map_err(|e| RemotelyError::Io(e))?;
    stream.flush().map_err(|e| RemotelyError::Io(e))?;

    // 2. Read password prompt
    wait_for_prompts(&mut stream, &mut buffer, &["password:"], start_time, timeout)?;

    // Send password
    stream.write_all(format!("{}\r\n", password).as_bytes())
        .map_err(|e| RemotelyError::Io(e))?;
    stream.flush().map_err(|e| RemotelyError::Io(e))?;

    // 3. Wait for shell prompt to confirm login
    // Common prompt suffixes: "$", "#", ">"
    let prompt_suffixes = &["$", "#", ">"];
    let (matched_prompt, prompt_index) = wait_for_prompts(&mut stream, &mut buffer, prompt_suffixes, start_time, timeout)?;

    // Clear buffer up to the prompt so we only return command output
    buffer.drain(0..prompt_index + matched_prompt.len());

    // 4. Send command
    let is_sudo = command.trim().starts_with("sudo ") || command.trim() == "sudo";
    stream.write_all(format!("{}\r\n", command).as_bytes())
        .map_err(|e| RemotelyError::Io(e))?;
    stream.flush().map_err(|e| RemotelyError::Io(e))?;

    // 5. Read output
    let mut command_output = Vec::new();
    let mut sudo_prompt_handled = false;

    loop {
        if start_time.elapsed() > timeout {
            return Err(RemotelyError::Timeout(timeout_secs));
        }

        let mut temp_buf = [0u8; 1024];
        let bytes_read = match stream.read(&mut temp_buf) {
            Ok(0) => break, // Connection closed
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(RemotelyError::Io(e)),
        };

        // Negotiate telnet options and extract raw text
        let raw_bytes = handle_telnet_options(&mut stream, &temp_buf[..bytes_read])?;
        command_output.extend_from_slice(&raw_bytes);

        let output_str = String::from_utf8_lossy(&command_output);

        // Sudo password prompt detection
        if is_sudo && !sudo_prompt_handled && device.sudo_password_required {
            if contains_sudo_prompt(&output_str) {
                stream.write_all(format!("{}\r\n", password).as_bytes())
                    .map_err(|e| RemotelyError::Io(e))?;
                stream.flush().map_err(|e| RemotelyError::Io(e))?;
                sudo_prompt_handled = true;
                // Clear the output buffer to remove the prompt and password echo
                command_output.clear();
                continue;
            }
        }

        // Wait for prompt to return (indicating command completed)
        if ends_with_any_prompt(&output_str, prompt_suffixes) {
            // Remove the trailing shell prompt from output
            let len = command_output.len();
            let mut prompt_len = 0;
            for suffix in prompt_suffixes {
                if output_str.trim_end().ends_with(suffix) {
                    prompt_len = suffix.len();
                    break;
                }
            }
            if len >= prompt_len {
                command_output.truncate(len - prompt_len);
            }
            break;
        }
    }

    // Clean up carriage returns (\r\n -> \n)
    let cleaned_output = clean_newlines(command_output);

    // Telnet doesn't return exit codes natively, so we default to 0 on success
    Ok((0, cleaned_output, Vec::new()))
}

pub fn test_connection(device: &Device, timeout_secs: u64) -> Result<(), RemotelyError> {
    // A connection test for telnet logs in and waits for the prompt
    let (code, _, _) = execute_command(device, "echo 'remotely_ok'", timeout_secs)?;
    if code == 0 {
        Ok(())
    } else {
        Err(RemotelyError::Other("Failed to execute test command over Telnet".to_string()))
    }
}

fn wait_for_prompts(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
    prompts: &[&str],
    start_time: Instant,
    timeout: Duration,
) -> Result<(String, usize), RemotelyError> {
    loop {
        if start_time.elapsed() > timeout {
            return Err(RemotelyError::Timeout(timeout.as_secs()));
        }

        // Check if we already have one of the prompts in the buffer
        let buffer_str = String::from_utf8_lossy(buffer);
        for prompt in prompts {
            if let Some(idx) = buffer_str.to_lowercase().rfind(&prompt.to_lowercase()) {
                return Ok((prompt.to_string(), idx));
            }
        }

        let mut temp_buf = [0u8; 1024];
        match stream.read(&mut temp_buf) {
            Ok(0) => return Err(RemotelyError::ConnectionFailed("".to_string(), "Connection closed by remote host".to_string())),
            Ok(n) => {
                let raw_bytes = handle_telnet_options(stream, &temp_buf[..n])?;
                buffer.extend_from_slice(&raw_bytes);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(RemotelyError::Io(e)),
        }
    }
}

fn ends_with_any_prompt(s: &str, prompts: &[&str]) -> bool {
    let trimmed = s.trim_end();
    for prompt in prompts {
        if trimmed.ends_with(prompt) {
            return true;
        }
    }
    false
}

fn contains_sudo_prompt(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("password for") || lower.contains("[sudo]") || lower.contains("password:")
}

fn clean_newlines(bytes: Vec<u8>) -> Vec<u8> {
    let mut cleaned = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                cleaned.push(b'\n');
                i += 2;
                continue;
            }
        }
        cleaned.push(bytes[i]);
        i += 1;
    }
    cleaned
}

/// Parses Telnet protocol negotiations (IAC commands) and returns clean text bytes.
/// Responds to the stream for any options we decline (WONT/DONT).
fn handle_telnet_options(stream: &mut TcpStream, data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut clean_data = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if data[i] == IAC {
            if i + 1 >= data.len() {
                break; // Incomplete command
            }
            let command = data[i + 1];
            match command {
                WILL | WONT | DO | DONT => {
                    if i + 2 >= data.len() {
                        break; // Incomplete option negotiation
                    }
                    let option = data[i + 2];
                    
                    // Reply to negotiation
                    let response = match command {
                        WILL => vec![IAC, DONT, option], // We don't want them doing it
                        DO => vec![IAC, WONT, option],   // We won't do it
                        _ => vec![],                     // No reply needed for WONT/DONT
                    };
                    
                    if !response.is_empty() {
                        stream.write_all(&response)?;
                        stream.flush()?;
                    }
                    i += 3;
                }
                SB => {
                    // Subnegotiation: skip until IAC SE
                    i += 2;
                    while i < data.len() {
                        if data[i] == IAC {
                            if i + 1 < data.len() && data[i + 1] == SE {
                                i += 2;
                                break;
                            }
                        }
                        i += 1;
                    }
                }
                _ => {
                    // Other 2-byte commands
                    i += 2;
                }
            }
        } else {
            clean_data.push(data[i]);
            i += 1;
        }
    }

    Ok(clean_data)
}
