use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerStatus {
    pub host: String,
    pub port: u16,
    pub online: bool,
    pub latency_ms: u64,
    pub version_name: String,
    pub protocol: i32,
    pub players_online: u32,
    pub players_max: u32,
    pub player_samples: Vec<String>,
    pub motd: String,
}

#[derive(Deserialize)]
struct RawSlpResponse {
    version: Option<RawVersion>,
    players: Option<RawPlayers>,
    description: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawVersion {
    name: Option<String>,
    protocol: Option<i32>,
}

#[derive(Deserialize)]
struct RawPlayers {
    max: Option<u32>,
    online: Option<u32>,
    sample: Option<Vec<RawPlayerSample>>,
}

#[derive(Deserialize, Clone)]
struct RawPlayerSample {
    name: Option<String>,
}

pub fn ping(host: &str, port: u16, timeout: Duration) -> Result<ServerStatus, String> {
    let addr_str = format!("{}:{}", host, port);
    let addrs: Vec<SocketAddr> = addr_str
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution error for {}: {}", addr_str, e))?
        .collect();

    if addrs.is_empty() {
        return Err(format!("Could not resolve host: {}", host));
    }

    let start_time = Instant::now();
    let mut stream = TcpStream::connect_timeout(&addrs[0], timeout)
        .map_err(|e| format!("Connection to {}:{} timed out / failed: {}", host, port, e))?;

    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    // 1. Handshake Packet (ID: 0x00, protocol: 767, host, port, nextState: 1)
    let mut hs_body = Vec::new();
    write_varint(&mut hs_body, 0x00); // Packet ID
    write_varint(&mut hs_body, 767);  // Protocol version (1.21.1)
    write_string(&mut hs_body, host); // Server address
    hs_body.extend_from_slice(&port.to_be_bytes()); // Server port
    write_varint(&mut hs_body, 1);    // Next state: 1 (status)

    send_packet(&mut stream, &hs_body)?;

    // 2. Status Request Packet (ID: 0x00)
    let mut req_body = Vec::new();
    write_varint(&mut req_body, 0x00);
    send_packet(&mut stream, &req_body)?;

    // 3. Status Response Packet
    let resp_bytes = read_packet(&mut stream)?;
    let latency_ms = start_time.elapsed().as_millis() as u64;

    let mut cursor = 0;
    let packet_id = read_varint_from(&resp_bytes, &mut cursor)?;
    if packet_id != 0x00 {
        return Err(format!("Unexpected response packet ID: {}", packet_id));
    }

    let json_str = read_string_from(&resp_bytes, &mut cursor)?;
    let raw: RawSlpResponse = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse SLP JSON response: {}", e))?;

    let version_name = raw.version.as_ref().and_then(|v| v.name.clone()).unwrap_or_else(|| "Unknown".to_string());
    let protocol = raw.version.as_ref().and_then(|v| v.protocol).unwrap_or(0);
    let players_online = raw.players.as_ref().and_then(|p| p.online).unwrap_or(0);
    let players_max = raw.players.as_ref().and_then(|p| p.max).unwrap_or(0);
    let player_samples = raw
        .players
        .as_ref()
        .and_then(|p| p.sample.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| s.name)
        .collect();

    let motd = parse_motd(raw.description);

    Ok(ServerStatus {
        host: host.to_string(),
        port,
        online: true,
        latency_ms,
        version_name,
        protocol,
        players_online,
        players_max,
        player_samples,
        motd,
    })
}

fn parse_motd(val: Option<serde_json::Value>) -> String {
    match val {
        Some(serde_json::Value::String(s)) => crate::rcon::strip_minecraft_formatting(&s),
        Some(serde_json::Value::Object(map)) => {
            if let Some(serde_json::Value::String(text)) = map.get("text") {
                crate::rcon::strip_minecraft_formatting(text)
            } else {
                "Minecraft Server".to_string()
            }
        }
        _ => "Minecraft Server".to_string(),
    }
}

fn write_varint(w: &mut Vec<u8>, mut val: i32) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val = ((val as u32) >> 7) as i32;
        if val != 0 {
            byte |= 0x80;
        }
        w.push(byte);
        if val == 0 {
            break;
        }
    }
}

fn write_string(w: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_varint(w, bytes.len() as i32);
    w.extend_from_slice(bytes);
}

fn send_packet(stream: &mut TcpStream, data: &[u8]) -> Result<(), String> {
    let mut header = Vec::new();
    write_varint(&mut header, data.len() as i32);
    stream
        .write_all(&header)
        .map_err(|e| format!("Failed to send packet length: {}", e))?;
    stream
        .write_all(data)
        .map_err(|e| format!("Failed to send packet body: {}", e))?;
    stream.flush().map_err(|e| format!("Flush failed: {}", e))?;
    Ok(())
}

fn read_varint_stream(stream: &mut TcpStream) -> Result<i32, String> {
    let mut res = 0i32;
    for i in 0..5 {
        let mut b = [0u8; 1];
        stream
            .read_exact(&mut b)
            .map_err(|e| format!("VarInt read error: {}", e))?;
        res |= ((b[0] & 0x7F) as i32) << (7 * i);
        if (b[0] & 0x80) == 0 {
            return Ok(res);
        }
    }
    Err("VarInt too large".to_string())
}

fn read_packet(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let length = read_varint_stream(stream)?;
    if length < 0 || length > 262144 {
        return Err(format!("Invalid incoming packet length: {}", length));
    }
    let mut buf = vec![0u8; length as usize];
    stream
        .read_exact(&mut buf)
        .map_err(|e| format!("Failed to read packet data: {}", e))?;
    Ok(buf)
}

fn read_varint_from(buf: &[u8], cursor: &mut usize) -> Result<i32, String> {
    let mut res = 0i32;
    for i in 0..5 {
        if *cursor >= buf.len() {
            return Err("Buffer underflow reading VarInt".to_string());
        }
        let b = buf[*cursor];
        *cursor += 1;
        res |= ((b & 0x7F) as i32) << (7 * i);
        if (b & 0x80) == 0 {
            return Ok(res);
        }
    }
    Err("VarInt too large".to_string())
}

fn read_string_from(buf: &[u8], cursor: &mut usize) -> Result<String, String> {
    let len = read_varint_from(buf, cursor)? as usize;
    if *cursor + len > buf.len() {
        return Err("Buffer underflow reading String".to_string());
    }
    let s = String::from_utf8_lossy(&buf[*cursor..*cursor + len]).to_string();
    *cursor += len;
    Ok(s)
}
