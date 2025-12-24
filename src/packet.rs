use std::collections::VecDeque;
use std::time::{Duration, Instant};
use super::crc32::crc32::crc32_sfu; 

//WARNING: packet SIZE MUST BE multiple of 4 for special CRC32 (reversed byte order inside dword)

pub const PACKET_SIGN_TX: u32 = 0x817E_A345;
pub const PACKET_SIGN_RX: u32 = 0x45A3_7E81;

pub const MAX_PACKET_SIZE: usize = 4096;

const SIGNATURE_BYTES: [u8; 4] = [
    (PACKET_SIGN_RX >> 24) as u8,
    (PACKET_SIGN_RX >> 16) as u8,
    (PACKET_SIGN_RX >> 8) as u8,
    (PACKET_SIGN_RX >> 0) as u8,
];

/// Build SFU-framed packet:
/// [4 bytes sign][code][code^0xFF][len_lo][len_hi][body...][4 bytes CRC (LE)]
///
/// CRC: SFU variant over bytes starting from `code` (offset 4)
pub fn packet_build(code: u8, body: &[u8]) -> Vec<u8> {

    let size = body.len();
    const HEADER_CRC: usize = 4 + 2 + 2 + 4; // 12
    let full_size = size + HEADER_CRC;
    assert!(
        full_size <= MAX_PACKET_SIZE,
        "packet body too large: {} (max {MAX_PACKET_SIZE})", full_size
    );

    // 4 (sign) + 2 (code/code^FF) + 2 (len) + body + 4 (CRC)
    let total_len = 4 + 2 + 2 + size + 4;
    let mut buf = Vec::with_capacity(total_len);

    // Signature: big-endian, как в C: (>>24), (>>16), (>>8), (>>0)
    buf.push((PACKET_SIGN_TX >> 24) as u8);
    buf.push((PACKET_SIGN_TX >> 16) as u8);
    buf.push((PACKET_SIGN_TX >> 8) as u8);
    buf.push((PACKET_SIGN_TX >> 0) as u8);

    // Code / inverted code
    buf.push(code ^ 0x00);
    buf.push(code ^ 0xFF);

    // Size: low byte, then high byte (как в C: (size >> 0), (size >> 8))
    buf.push((size & 0xFF) as u8);
    buf.push(((size >> 8) & 0xFF) as u8);

    // Body
    buf.extend_from_slice(body);

    // CRC over [4..(8+size)) — т.е. code, code^FF, len_lo, len_hi, body...
    let crc_region_start = 4;
    let crc_region_end = 8 + size;
    let crc = crc32_sfu(&buf[crc_region_start..crc_region_end]);

    // CRC is appended little-endian, как в C: (>>0), (>>8), (>>16), (>>24)
    buf.push((crc >> 0) as u8);
    buf.push((crc >> 8) as u8);
    buf.push((crc >> 16) as u8);
    buf.push((crc >> 24) as u8);

    debug_assert_eq!(buf.len(), total_len);
    buf
}

/// Parsed firmware packets storage and statistics.
///
/// `packets[code]` contains the last successfully received body for that command code.
/// Logs are stored as lines of printable ASCII text (split on '\n').
pub struct PacketParser {
    /// Collected log lines (printf text from MCU).
    pub logs: VecDeque<String>,

    /// Per-command last bodies, indexed by command code (0..=255).
    pub packets: [VecDeque<Vec<u8>>; 256],

    /// Successfully received packets (CRC and size OK).
    pub stat_valid_packets: u64,
    /// Packets where CRC32 check failed.
    pub stat_crc_error_packets: u64,
    /// Packets where size or packet length constraints failed.
    pub stat_size_or_code_error_packets: u64,
    /// Any other parser errors (should normally stay 0).
    pub stat_other_error_packets: u64,

    /// Bytes still missing for the last started packet (0 = no unfinished packet).
    pub stat_incomplete_bytes: u64,

    /// Total number of bytes classified as log (non-packet) bytes.
    pub stat_log_bytes: u64,
    /// Total number of completed log lines.
    pub stat_log_lines: u64,

    pub current_log_line: String,

    // --- internal state ---

    state: ParseState,

    current_code: u8,
    expected_size: usize,
    body_buf: Vec<u8>,

    crc_buf: [u8; 4],
    crc_pos: usize,

    /// Remaining bytes (body + CRC) for current packet once size is known.
    remaining_in_packet: usize,
    timeout_log: Instant,
}

enum ParseState {
    /// Not currently inside a packet, scanning for signature or logging bytes.
    Idle,
    /// Matching packet signature; `matched` is number of already matched bytes (1..3).
    WaitSignature { matched: u8 },
    /// Expecting command code after full signature.
    HeaderCode,
    /// Expecting inverted command code.
    HeaderCodeInv,
    /// Expecting low byte of body size.
    HeaderLenLo,
    /// Expecting high byte of body size.
    HeaderLenHi,
    /// Receiving body bytes.
    Body,
    /// Receiving 4-byte CRC.
    Crc,
    /// Skipping a broken packet (size or other hard error); `remaining` bytes to drop.
    Skip { remaining: usize },
}

/// Parser trait for feeding bytes and printing statistics.
pub trait PacketParserExt {
    /// Construct new parser with default state and zeroed statistics.
    fn new() -> Self
    where
        Self: Sized;

    /// Feed single byte from UART/serial stream.
    fn receive_byte(&mut self, x: u8);

    /// Feed a block of bytes from UART/serial stream.
    fn receive_data(&mut self, data: &[u8]);

    /// check timeouts
    fn tick(&mut self);

    /// Reset everything: state, logs, packets and statistics.
    #[allow(dead_code)]
    fn reset(&mut self);

    /// Reset only error-related statistics.
    #[allow(dead_code)]
    fn reset_error_stats(&mut self);

    /// Print a short summary of current statistics.
    fn print_stats(&self);
}

impl PacketParserExt for PacketParser {
    fn new() -> Self {
        PacketParser {
            logs: VecDeque::new(),
            packets: std::array::from_fn(|_| VecDeque::new()),

            stat_valid_packets: 0,
            stat_crc_error_packets: 0,
            stat_size_or_code_error_packets: 0,
            stat_other_error_packets: 0,

            stat_incomplete_bytes: 0,

            stat_log_bytes: 0,
            stat_log_lines: 0,

            state: ParseState::Idle,
            current_log_line: String::new(),

            current_code: 0,
            expected_size: 0,
            body_buf: Vec::new(),

            crc_buf: [0; 4],
            crc_pos: 0,

            remaining_in_packet: 0,
            timeout_log: Instant::now() + Duration::from_millis(500),
        }
    }

    fn receive_byte(&mut self, x: u8) {
        match self.state {
            ParseState::Idle => {
                if x == SIGNATURE_BYTES[0] {
                    // Possible start of signature.
                    self.state = ParseState::WaitSignature { matched: 1 };
                } else {
                    // Plain log byte.
                    self.handle_log_byte(x);
                }
            }

            ParseState::WaitSignature { matched } => {
                // We have already matched `matched` bytes of SIGNATURE_BYTES.
                if matched < 4 && x == SIGNATURE_BYTES[matched as usize] {
                    let new_matched = matched + 1;
                    if new_matched == 4 {
                        // Full signature matched: start packet header parsing.
                        self.start_packet_after_signature();
                    } else {
                        self.state = ParseState::WaitSignature { matched: new_matched };
                    }
                } else {
                    // Signature failed. Previous matched bytes are actually log bytes.
                    for i in 0..matched {
                        self.handle_log_byte(SIGNATURE_BYTES[i as usize]);
                    }
                    // Current byte may start a new signature or be a log byte.
                    if x == SIGNATURE_BYTES[0] {
                        self.state = ParseState::WaitSignature { matched: 1 };
                    } else {
                        self.state = ParseState::Idle;
                        self.handle_log_byte(x);
                    }
                }
            }

            ParseState::HeaderCode => {
                self.current_code = x;
                self.state = ParseState::HeaderCodeInv;
                // We don't treat any code values as invalid here;
                // "size_or_code" errors are currently size-related.
            }

            ParseState::HeaderCodeInv => {
                // Check inverted code: must be code ^ 0xFF.
                if x != (self.current_code ^ 0xFF) {
                    // Hard error: skip rest of minimal possible packet?
                    // Instead, treat as size/code error and skip minimal body+CRC.
                    self.stat_size_or_code_error_packets += 1;
                    //self.abort_current_packet(); // We do not treat these bytes as logs, as they are part of a broken packet.
                    self.expected_size = 0;
                    self.body_buf.clear();
                    self.crc_pos = 0;
                    self.state = ParseState::Skip { remaining: 2 }; // len_lo, len_hi                    

                } else {
                    self.state = ParseState::HeaderLenLo;
                }
            }

            ParseState::HeaderLenLo => {
                self.expected_size = x as usize;
                self.state = ParseState::HeaderLenHi;
            }

            ParseState::HeaderLenHi => {
                self.expected_size |= (x as usize) << 8;

                // Compute full packet size including header and CRC.
                // Packet layout:
                // 4 bytes signature + 2 code + 2 size + body + 4 CRC
                let total_packet_size = 4 + 2 + 2 + self.expected_size + 4;

                if (total_packet_size == 0) || 
                   (total_packet_size > MAX_PACKET_SIZE) ||
                   (total_packet_size & 0x3 != 0) //MUST BE multiple of 4 for special CRC32 (reversed byte order inside dword)
                {
                    self.stat_size_or_code_error_packets += 1;
                    // We know how many bytes remain in this broken packet (body + CRC).
                    let remaining = self.expected_size + 4;
                    if remaining > 0 {
                        self.state = ParseState::Skip { remaining };
                    } else {
                        self.state = ParseState::Idle;
                    }
                    self.stat_incomplete_bytes = 0;
                    self.remaining_in_packet = 0;
                } else {
                    // Valid size: prepare to receive body.
                    self.body_buf.clear();
                    self.body_buf.reserve(self.expected_size);
                    self.crc_pos = 0;
                    self.remaining_in_packet = self.expected_size + 4;
                    self.stat_incomplete_bytes = self.remaining_in_packet as u64;
                    self.state = 
                        if self.expected_size > 0 {
                            ParseState::Body
                        } else {
                            ParseState::Crc};
                }
            }

            ParseState::Body => {
                // Store body byte.
                if self.body_buf.len() < self.expected_size {
                    self.body_buf.push(x);
                    if self.remaining_in_packet > 0 {
                        self.remaining_in_packet -= 1;
                        self.stat_incomplete_bytes = self.remaining_in_packet as u64;
                    }
                }

                // When body is complete, move to CRC collection.
                if self.body_buf.len() == self.expected_size {
                    self.crc_pos = 0;
                    self.state = ParseState::Crc;
                }
            }

            ParseState::Crc => {
                if self.crc_pos < 4 {
                    self.crc_buf[self.crc_pos] = x;
                    self.crc_pos += 1;
                    if self.remaining_in_packet > 0 {
                        self.remaining_in_packet -= 1;
                        self.stat_incomplete_bytes = self.remaining_in_packet as u64;
                    }
                }

                if self.crc_pos == 4 {
                    // We have full body and CRC, validate packet.
                    self.finish_packet();
                }
            }

            ParseState::Skip { remaining } => {
                if remaining > 1 {
                    self.state = ParseState::Skip {
                        remaining: remaining - 1,
                    };
                } else {
                    // Skipped entire broken packet, back to idle.
                    self.state = ParseState::Idle;
                }
                // Skipped bytes are not considered logs.
            }
        }
    }

    fn receive_data(&mut self, data: &[u8]) {
        for &b in data {
            self.receive_byte(b);
        }
        self.tick();
    }

    fn tick(&mut self) {
        if (self.current_log_line.len() > 0) && (Instant::now() > self.timeout_log) {
            self.logs.push_back(std::mem::take(&mut self.current_log_line));
            self.stat_log_lines += 1;
        }
    }

    fn reset(&mut self) {
        self.logs.clear();
        for v in &mut self.packets {
            v.clear();
        }

        self.stat_valid_packets = 0;
        self.stat_crc_error_packets = 0;
        self.stat_size_or_code_error_packets = 0;
        self.stat_other_error_packets = 0;
        self.stat_incomplete_bytes = 0;
        self.stat_log_bytes = 0;
        self.stat_log_lines = 0;

        self.state = ParseState::Idle;
        self.current_log_line.clear();
        self.current_code = 0;
        self.expected_size = 0;
        self.body_buf.clear();
        self.crc_buf = [0; 4];
        self.crc_pos = 0;
        self.remaining_in_packet = 0;
    }

    fn reset_error_stats(&mut self) {
        self.stat_crc_error_packets = 0;
        self.stat_size_or_code_error_packets = 0;
        self.stat_other_error_packets = 0;
        self.stat_incomplete_bytes = 0;
        // Successful statistics (valid packets, logs) are kept.
    }

    fn print_stats(&self) {
        println!("--- PacketParser statistics ---");
        println!("Valid packets:            {}", self.stat_valid_packets);
        println!("CRC error packets:        {}", self.stat_crc_error_packets);
        println!("Size/code error packets:  {}", self.stat_size_or_code_error_packets);
        println!("Other error packets:      {}", self.stat_other_error_packets);
        println!("Incomplete bytes pending: {}", self.stat_incomplete_bytes);
        println!("Log bytes:                {}", self.stat_log_bytes);
        println!("Log lines:                {}", self.stat_log_lines);
    }
}

impl PacketParser {
    /// Called when full signature has been matched.
    fn start_packet_after_signature(&mut self) {
        self.state = ParseState::HeaderCode;
        self.current_code = 0;
        self.expected_size = 0;
        self.body_buf.clear();
        self.crc_pos = 0;
        self.remaining_in_packet = 0;
        self.stat_incomplete_bytes = 0;
    }

    /// Handle a single log byte (non-packet data).
    fn handle_log_byte(&mut self, b: u8) {
        self.stat_log_bytes += 1;
        self.timeout_log = Instant::now() + Duration::from_millis(250);
        match b {
            b'\n' => {
                // End of line.
                self.logs.push_back(std::mem::take(&mut self.current_log_line));
                self.stat_log_lines += 1;
            }
            32..=126 => {
                // Printable ASCII.
                self.current_log_line.push(b as char);
                if self.current_log_line.len() >= 256 {
                    self.logs.push_back(std::mem::take(&mut self.current_log_line));
                    self.stat_log_lines += 1;
                }
            }
            b'\r' => {}
            b'\t' => {
                let mut cnt = 0;
                while (self.current_log_line.len() % 8 != 7) || (cnt == 0) {
                    self.current_log_line.push(' ');
                    cnt += 1;
                }
            }
            _ => {                
                self.current_log_line.push_str(&format!("<{:02X}>", b));
                // Ignore other control characters (e.g. '\r').
            }
        }
    }

    /// Abort current packet parsing due to a hard error.
    fn abort_current_packet(&mut self) {
        self.state = ParseState::Idle;
        self.expected_size = 0;
        self.body_buf.clear();
        self.crc_pos = 0;
        self.remaining_in_packet = 0;
        self.stat_incomplete_bytes = 0;
    }

    /// Finalize current packet: compute CRC and either accept or count as error.
    fn finish_packet(&mut self) {
        if self.body_buf.len() != self.expected_size {
            // Should not happen, but treat as generic error.
            self.stat_other_error_packets += 1;
            self.abort_current_packet();
            return;
        }

        // Build CRC input: [code][code^0xFF][len_lo][len_hi][body...]
        let len_lo = (self.expected_size & 0xFF) as u8;
        let len_hi = ((self.expected_size >> 8) & 0xFF) as u8;

        let mut crc_input = Vec::with_capacity(4 + self.expected_size);
        crc_input.push(self.current_code ^ 0x00);
        crc_input.push(self.current_code ^ 0xFF);
        crc_input.push(len_lo);
        crc_input.push(len_hi);
        crc_input.extend_from_slice(&self.body_buf);

        let crc_calc = crc32_sfu(&crc_input);
        let crc_recv = u32::from_le_bytes(self.crc_buf);

        if crc_calc == crc_recv {
            // Successful packet.
            let idx = self.current_code as usize;
            if idx < self.packets.len() {
                self.packets[idx].push_back(self.body_buf.clone());
            } else {
                // Should not happen as code is u8, but be safe.
                self.stat_other_error_packets += 1;
            }
            self.stat_valid_packets += 1;
        } else {
            println!("CRC32 Broken body: {:02X?}", crc_input);
            println!("CRC32 Broken code: {}", self.current_code);
            // CRC error: ignore this whole packet.
            self.stat_crc_error_packets += 1;
        }

        // In both cases we forget this packet and return to idle.
        self.abort_current_packet();
    }
}
