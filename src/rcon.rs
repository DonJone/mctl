use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_RESPONSE_VALUE: i32 = 0;

pub struct RconClient {
    stream: TcpStream,
    req_id: i32,
}

impl RconClient {
    pub fn connect(host: &str, port: u16, password: &str, timeout: Duration) -> Result<Self, String> {
        let addr_str = format!("{}:{}", host, port);
        let addrs: Vec<SocketAddr> = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("DNS resolution failed for {}: {}", addr_str, e))?
            .collect();

        if addrs.is_empty() {
            return Err(format!("Could not resolve host: {}", host));
        }

        let stream = TcpStream::connect_timeout(&addrs[0], timeout)
            .map_err(|e| format!("Failed to connect to RCON server at {}: {}", addr_str, e))?;

        stream.set_read_timeout(Some(timeout)).ok();
        stream.set_write_timeout(Some(timeout)).ok();

        let mut client = Self { stream, req_id: 1 };
        client.authenticate(password)?;
        Ok(client)
    }

    fn authenticate(&mut self, password: &str) -> Result<(), String> {
        let id = self.next_id();
        self.send_packet(id, SERVERDATA_AUTH, password.as_bytes())?;

        let (resp_id, resp_type, _) = self.read_packet()?;
        if resp_id == -1 || resp_id != id || resp_type != SERVERDATA_EXECCOMMAND && resp_type != SERVERDATA_RESPONSE_VALUE {
            return Err("RCON authentication failed: invalid password".to_string());
        }
        Ok(())
    }

    pub fn exec(&mut self, command: &str) -> Result<String, String> {
        let id = self.next_id();
        self.send_packet(id, SERVERDATA_EXECCOMMAND, command.as_bytes())?;

        let (_, _, payload) = self.read_packet()?;
        let text = String::from_utf8_lossy(&payload).to_string();
        Ok(strip_minecraft_formatting(&text))
    }

    fn next_id(&mut self) -> i32 {
        self.req_id += 1;
        self.req_id
    }

    fn send_packet(&mut self, id: i32, p_type: i32, payload: &[u8]) -> Result<(), String> {
        let length = (payload.len() + 10) as i32;
        let mut buf = Vec::with_capacity((length + 4) as usize);

        buf.extend_from_slice(&length.to_le_bytes());
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&p_type.to_le_bytes());
        buf.extend_from_slice(payload);
        buf.push(0); // Null terminator for payload
        buf.push(0); // 1-byte padding

        self.stream
            .write_all(&buf)
            .map_err(|e| format!("Failed to send RCON packet: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("Failed to flush RCON stream: {}", e))?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<(i32, i32, Vec<u8>), String> {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .map_err(|e| format!("Failed to read RCON packet length: {}", e))?;
        let length = i32::from_le_bytes(len_buf) as usize;

        if length < 10 || length > 65536 {
            return Err(format!("Invalid RCON packet length: {}", length));
        }

        let mut data = vec![0u8; length];
        self.stream
            .read_exact(&mut data)
            .map_err(|e| format!("Failed to read RCON packet body: {}", e))?;

        let id = i32::from_le_bytes(data[0..4].try_into().unwrap());
        let p_type = i32::from_le_bytes(data[4..8].try_into().unwrap());
        let payload = if length > 10 {
            // Drop null terminators at the end
            let body_end = length - 2;
            data[8..body_end].to_vec()
        } else {
            Vec::new()
        };

        Ok((id, p_type, payload))
    }

    pub fn interactive_shell(&mut self) -> Result<(), String> {
        println!("Connected to Minecraft RCON console.");
        println!("Type commands to send to server. Type 'exit' or 'quit' to leave.");
        println!();

        let stdin = io::stdin();
        let mut line = String::new();

        loop {
            print!("rcon> ");
            io::stdout().flush().ok();
            line.clear();
            if stdin.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                break;
            }

            let cmd = line.trim();
            if cmd.is_empty() {
                continue;
            }
            if cmd.eq_ignore_ascii_case("exit") || cmd.eq_ignore_ascii_case("quit") {
                break;
            }

            match self.exec(cmd) {
                Ok(resp) => {
                    if !resp.is_empty() {
                        println!("{}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}

pub fn strip_minecraft_formatting(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next(); // Skip the color/format code char
        } else {
            out.push(c);
        }
    }
    out
}
