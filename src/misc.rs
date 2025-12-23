use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::io::{self};

#[macro_export]
macro_rules! bytes {
    ($($x:expr),* $(,)?) => {{
        let mut v = Vec::new();
        $(
            v.extend_from_slice(&$x);
        )*
        v
    }};
}

/// Serialize u32 into [u8; 4] LE.
#[macro_export]
macro_rules! serialize_u32 {
    ($value:expr) => {{
        ($value as u32).to_le_bytes()
    }};
}

/// Serialize u16 into [u8; 2] LE.
#[macro_export]
macro_rules! serialize_u16 {
    ($value:expr) => {{
        ($value as u16).to_le_bytes()
    }};
}


pub fn tostr(vec:&[u8]) -> String {
    let mut res = String::new();
    for &c in vec {
        if c < 32 || c > 127 {
            res += &format!("<{:02x}>", c);
        } else {
            res.push(c as char)
        }
    }
    return res;
}

pub fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}

pub fn strip_trailing_newline(input: &str) -> &str {
    input
    .strip_suffix("\r\n")
    .or(input.strip_suffix("\n"))
        .unwrap_or(input)
}

#[inline]
pub fn deserialize_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset + 0],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

#[inline]
pub fn deserialize_u16_le(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([
        buf[offset + 0],
        buf[offset + 1],
    ])
}

#[inline]
pub fn deserialize_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&buf[offset..offset + N]);
    out
}