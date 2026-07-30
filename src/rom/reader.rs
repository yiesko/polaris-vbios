//! Little-endian byte reader with bounds checking.
//! Every VBIOS field read goes through here, so an invalid offset
//! becomes a recoverable error instead of a panic.

use anyhow::{Result, bail};

pub struct Reader<'a> {
    pub data: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn check(&self, off: usize, n: usize) -> Result<()> {
        if off + n > self.data.len() {
            bail!(
                "read out of bounds: offset 0x{:X} + {} bytes > file size (0x{:X})",
                off,
                n,
                self.data.len()
            );
        }
        Ok(())
    }

    pub fn u8(&self, off: usize) -> Result<u8> {
        self.check(off, 1)?;
        Ok(self.data[off])
    }

    pub fn u16(&self, off: usize) -> Result<u16> {
        self.check(off, 2)?;
        Ok(u16::from_le_bytes([self.data[off], self.data[off + 1]]))
    }

    pub fn u32(&self, off: usize) -> Result<u32> {
        self.check(off, 4)?;
        Ok(u32::from_le_bytes([
            self.data[off],
            self.data[off + 1],
            self.data[off + 2],
            self.data[off + 3],
        ]))
    }

    /// NUL-terminated (or up to `max_len` bytes) ASCII string.
    pub fn cstr(&self, off: usize, max_len: usize) -> Result<String> {
        self.check(off, max_len)?;
        let raw = &self.data[off..off + max_len];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        Ok(String::from_utf8_lossy(&raw[..end]).trim().to_string())
    }

    pub fn bytes(&self, off: usize, n: usize) -> Result<&'a [u8]> {
        self.check(off, n)?;
        Ok(&self.data[off..off + n])
    }
}
