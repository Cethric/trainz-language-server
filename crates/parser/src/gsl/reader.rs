use crate::gsl::GslLibrary;
use crate::gsl::error::GslError;
use std::convert::TryInto;

pub struct GslReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> GslReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn parse(&mut self) -> Result<GslLibrary, GslError> {
        if self.data.len() < 8 {
            return Err(GslError::UnexpectedEof(0));
        }

        // Skip total size
        self.offset = 4;

        let mut name = String::new();
        let mut symbols = Vec::new();

        // The first section contains the library name at offset 0x14
        if self.offset + 20 <= self.data.len() {
            let magic = self.read_u32()?;
            if magic == 0xF762575D {
                // Skip next 12 bytes of metadata (0x08 to 0x14)
                self.offset += 12;
                let name_len = self.read_u32()? as usize;
                if self.offset + name_len <= self.data.len() {
                    let name_bytes = &self.data[self.offset..self.offset + name_len];
                    name = String::from_utf8_lossy(name_bytes)
                        .trim_matches('\0')
                        .to_string();
                    self.offset += name_len;
                }
            } else {
                return Err(GslError::InvalidMagic(magic));
            }
        }

        // Scan the rest of the file for strings that look like symbols
        // Symbols in GSL often start with '$' or are class/method names.
        // We'll collect all null-terminated strings for now and can filter later if needed.
        let mut i = self.offset;
        while i < self.data.len() {
            // Find start of a printable string
            if self.data[i].is_ascii_graphic() || self.data[i] == b' ' {
                let start = i;
                while i < self.data.len() && self.data[i] != 0 {
                    if !self.data[i].is_ascii_graphic() && self.data[i] != b' ' {
                        // Not a valid string char, break
                        break;
                    }
                    i += 1;
                }

                if i < self.data.len() && self.data[i] == 0 {
                    let s = String::from_utf8_lossy(&self.data[start..i]).to_string();
                    if s.len() > 1 {
                        // Avoid single-character junk
                        if !symbols.contains(&s)
                            && s != name
                            && !s.ends_with(".gs")
                            && s != "$Symbol"
                            && s != "lgbd"
                        {
                            symbols.push(s);
                        }
                    }
                }
            }
            i += 1;
        }

        Ok(GslLibrary::new(name, symbols))
    }

    fn read_u32(&mut self) -> Result<u32, GslError> {
        if self.offset + 4 > self.data.len() {
            return Err(GslError::UnexpectedEof(self.offset));
        }
        let val = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let mut data = Vec::new();
        data.extend_from_slice(&28u32.to_le_bytes()); // total size
        data.extend_from_slice(&0xF762575Du32.to_le_bytes()); // magic
        data.extend_from_slice(&[0u8; 12]); // metadata
        data.extend_from_slice(&5u32.to_le_bytes()); // name len
        data.extend_from_slice(b"test\0"); // name

        let mut reader = GslReader::new(&data);
        let lib = reader.parse().unwrap();
        assert_eq!(lib.name, "test");
    }
}
