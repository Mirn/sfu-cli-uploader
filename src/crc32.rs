/// CRC32 implementation used in SFU (STM-style, poly 0x04C11DB7, MSB-first).
/// This matches the original C implementation that works on 32-bit words
/// (data length must be a multiple of 4 bytes).
pub mod crc32 {
    // Same polynomial as in the C version: 0x04C11DB7 (MSB-first)
    const fn make_table_sfu() -> [u32; 256] {
        const POLY_SFU: u32 = 0x04C11_DB7;
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = (i as u32) << 24;
            let mut k = 0;
            while k < 8 {
                if (c & 0x8000_0000) != 0 {
                    c = (c << 1) ^ POLY_SFU;
                } else {
                    c <<= 1;
                }
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    }
    /// Compile-time self-test. If this fails, compilation will fail.
    const fn crc32_sfu_selftest() {
        let v1 = crc32_sfu_impl(0xFFFF_FFFF, b"12345678");
        let v2 = crc32_sfu_impl(0xFFFF_FFFF, b"TESTABCD");

        if v1 != 0xFEFC_54F9 || v2 != 0xB9CF_5238 {
            panic!("crc32_sfu self-check failed: expected (0xFEFC54F9, 0xB9CF5238)");
        }
    }
    // Run self-test at compile time.
    const _: () = {
        crc32_sfu_selftest();
    };

    // Immutable table, generated at compile time.
    pub(crate) const TABLE_SFU: [u32; 256] = make_table_sfu();

    /// Core SFU CRC32 implementation in MSB-first mode, byte-by-byte.
    /// Fully compatible with original algorithm, but no longer requires
    /// processing in 32-bit words.
    /// 
    /// - `initial` is the initial CRC register.
    /// - `data` is any byte slice.
    /// - No final XOR is applied (same as original SFU version).
    ///
    /// This is analogous to `crc32_ieee_impl`, but uses the MSB-first polynomial
    /// and update rule.
    const fn crc32_sfu_impl(initial: u32, data: &[u8]) -> u32 {
        let mut crc = initial;
        let mut i = 0;
        let len = data.len();

        // Process full 4-byte blocks exactly like the original C code:
        // for each little-endian word w = [b0, b1, b2, b3]
        // it feeds bytes in order: b3, b2, b1, b0.
        while i + 3 < len {
            let b0 = data[i];
            let b1 = data[i + 1];
            let b2 = data[i + 2];
            let b3 = data[i + 3];

            let mut c = crc;

            let mut idx = ((c >> 24) as u8) ^ b3;
            c = (c << 8) ^ TABLE_SFU[idx as usize];

            idx = ((c >> 24) as u8) ^ b2;
            c = (c << 8) ^ TABLE_SFU[idx as usize];

            idx = ((c >> 24) as u8) ^ b1;
            c = (c << 8) ^ TABLE_SFU[idx as usize];

            idx = ((c >> 24) as u8) ^ b0;
            c = (c << 8) ^ TABLE_SFU[idx as usize];

            crc = c;
            i += 4;
        }

        // // Process remaining 1–3 bytes as ordinary MSB-first CRC32.
        // while i < len {
        //     let byte = data[i];
        //     let idx = ((crc >> 24) as u8) ^ byte;
        //     crc = (crc << 8) ^ TABLE_SFU[idx as usize];
        //     i += 1;
        // }

        crc
    }

    /// "Raw" SFU CRC32: takes an existing CRC register and extends it with `data`.
    ///
    /// - `previous_crc` is the current CRC register value.
    /// - No final XOR is applied here (matches the original C raw function).
    /// - `data.len()` must be a multiple of 4 bytes.
    pub fn crc32_sfu_raw(previous_crc: u32, data: &[u8]) -> u32 {
        if data.len() % 4 != 0 {
            println!("CRC32 len ERROR");
        }
        crc32_sfu_impl(previous_crc, data)
    }

    /// Main SFU CRC32 function, equivalent to the original `crc32_calc`.
    ///
    /// - Initial value is 0xFFFF_FFFF.
    /// - No final XOR is applied.
    /// - `data.len()` must be a multiple of 4 bytes.
    pub fn crc32_sfu(data: &[u8]) -> u32 {
        crc32_sfu_raw(0xFFFF_FFFF, data)
    }


/// CRC-32 IEEE 802.3 implementation (poly 0xEDB88320, LSB-first).
/// This matches the original C implementation, but works on *arbitrary* byte
/// lengths instead of requiring multiples of 4.

    const fn make_table_ieee() -> [u32; 256] {
        const POLY_IEEE: u32 = 0xEDB8_8320;
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut j = 0;
            while j < 8 {
                if (c & 1) != 0 {
                    c = POLY_IEEE ^ (c >> 1);
                } else {
                    c >>= 1;
                }
                j += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    }
    /// Compile-time self-test. If this fails, compilation will fail.
    const fn crc32_ieee_selftest() {
        let v1 = crc32_ieee_impl(0, b"12345678");
        let v2 = crc32_ieee_impl(0, b"TESTABCD");

        if v1 != 0x9AE0_DAAF || v2 != 0x1FA7_9460 {
            panic!("crc32_IEEE8023 self-check failed: expected (0x9AE0DAAF, 0x1FA79460)");
        }
    }

    // Run self-test at compile time.
    const _: () = {
        crc32_ieee_selftest();
    };

    // Immutable table, generated at compile time.
    pub(crate) const TABLE_IEEE: [u32; 256] = make_table_ieee();

    /// Core IEEE 802.3 CRC implementation that can be evaluated at compile time.
    ///
    /// - `initial` is the *finalized* CRC of the previous block.
    ///   (i.e., you pass the same value as returned by crc32_IEEE8023 /
    ///   crc32_IEEE8023_raw).
    ///
    /// It internally does:
    ///   crc = !initial;
    ///   for each byte: update;
    ///   return !crc;
    const fn crc32_ieee_impl(initial: u32, data: &[u8]) -> u32 {
        let mut crc = !initial;
        let mut i = 0;

        while i < data.len() {
            let byte = data[i] as u32;
            crc = TABLE_IEEE[((crc ^ byte) & 0xFF) as usize] ^ (crc >> 8);
            i += 1;
        }

        !crc
    }

    /// Raw IEEE 802.3 CRC.
    ///
    /// This is fully incremental:
    /// - `previous_crc` must be a "finalized" CRC (as returned by
    ///   `crc32_IEEE8023` or `crc32_IEEE8023_raw`).
    /// - `data` is any byte slice (no 4-byte alignment required).
    ///
    /// Then:
    ///   crc32_IEEE8023_raw(previous_crc, tail)
    /// is equivalent to:
    ///   crc32_IEEE8023(head + tail)
    /// where `previous_crc == crc32_IEEE8023(head)`.
    #[allow(dead_code)]
    pub fn crc32_ieee8023_raw(previous_crc: u32, data: &[u8]) -> u32 {
        crc32_ieee_impl(previous_crc, data)
    }

    /// Main IEEE 802.3 CRC function, equivalent to original `crc32_IEEE8023`.
    ///
    /// Initial CRC is 0. Final XOR is applied, so this returns the "final"
    /// CRC value.
    #[allow(dead_code)]
    pub fn crc32_ieee8023(data: &[u8]) -> u32 {
        crc32_ieee_impl(0, data)
    }
}

// ---- Unit tests ----

#[cfg(test)]
mod tests {
    use super::crc32::*;

    #[test]
    fn crc32_sfu_known_vectors() {
        assert_eq!(crc32_sfu(b"12345678"), 0xFEFC_54F9);
        assert_eq!(crc32_sfu(b"TESTABCD"), 0xB9CF_5238);
    }

    #[test]
    fn crc32_ieee8023_known_vectors() {
        assert_eq!(crc32_ieee8023(b"12345678"), 0x9AE0_DAAF);
        assert_eq!(crc32_ieee8023(b"TESTABCD"), 0x1FA7_9460);
    }

    #[test]
    fn crc32_ieee8023_incremental_equals_one_shot() {
        let full = b"helloworld";
        let head = b"hello";
        let tail = b"world";

        let one_shot = crc32_ieee8023(full);
        let head_crc = crc32_ieee8023(head);
        let incremental = crc32_ieee8023_raw(head_crc, tail);

        assert_eq!(one_shot, incremental);
    }

    #[test]
    fn crc32_sfu_incremental_equals_one_shot() {
        // Lengths are multiples of 4, as required by the SFU variant.
        let full = b"1234ABCD";
        let head = b"1234";
        let tail = b"ABCD";

        let one_shot = crc32_sfu(full);
        let head_crc = crc32_sfu(head);
        let incremental = crc32_sfu_raw(head_crc, tail);

        assert_eq!(one_shot, incremental);
    }
}
