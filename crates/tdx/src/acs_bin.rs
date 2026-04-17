use crate::error::TdxError;
use crate::value::TdxValue;

pub struct AcsParser<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> AcsParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn consumed(&self) -> usize {
        self.offset
    }

    pub fn parse(&mut self) -> Result<TdxValue, TdxError> {
        let header = self.read_bytes(4)?;
        if header != b"ACS$" {
            let mut h = [0u8; 4];
            h.copy_from_slice(header);
            return Err(TdxError::InvalidHeader(h));
        }

        let _v1 = self.read_u32()?; // 1
        let _v2 = self.read_u32()?; // 0
        let _total_len = self.read_u32()?;

        Ok(TdxValue::Container(self.read_entries()?))
    }

    pub fn read_entries(&mut self) -> Result<Vec<(String, TdxValue)>, TdxError> {
        let mut entries = Vec::new();
        while self.offset < self.data.len() {
            // Skip potential padding zeros at the end
            if self.data[self.offset..].iter().all(|&b| b == 0) {
                self.offset = self.data.len();
                break;
            }

            match self.read_entry() {
                Ok(entry) => {
                    entries.push(entry);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(entries)
    }

    fn read_entry(&mut self) -> Result<(String, TdxValue), TdxError> {
        let entry_len = self.read_u32()? as usize;
        let start_offset = self.offset;

        let tag_len = self.read_u8()? as usize;
        let tag_bytes = self.read_bytes(tag_len)?;
        let tag = String::from_utf8_lossy(tag_bytes)
            .trim_matches('\0')
            .to_string();

        let consumed = self.offset - start_offset;
        if consumed >= entry_len {
            return Ok((tag, TdxValue::Binary(vec![])));
        }

        let value_type = self.read_u8()?;
        let remaining = entry_len.saturating_sub(self.offset - start_offset);

        let value = match value_type {
            0 => {
                // Nested Container
                let sub_data = self.read_bytes(remaining)?;
                let mut sub_parser = AcsParser {
                    data: sub_data,
                    offset: 0,
                };
                TdxValue::Container(sub_parser.read_entries()?)
            }
            1 => {
                // Integer/Boolean?
                if remaining == 4 {
                    TdxValue::Int32(self.read_u32()? as i32)
                } else if remaining == 1 {
                    TdxValue::Bool(self.read_u8()? != 0)
                } else {
                    TdxValue::Binary(self.read_bytes(remaining)?.to_vec())
                }
            }
            2 => {
                // Float or Array of Floats
                if remaining == 4 {
                    TdxValue::Float32(self.read_f32()?)
                } else if remaining.wrapping_rem(4) == 0 {
                    let mut floats = Vec::new();
                    for _ in 0..(remaining / 4) {
                        floats.push(TdxValue::Float32(self.read_f32()?));
                    }
                    TdxValue::Array(floats)
                } else {
                    TdxValue::Binary(self.read_bytes(remaining)?.to_vec())
                }
            }
            3 => {
                // String
                let bytes = self.read_bytes(remaining)?;
                let s = String::from_utf8_lossy(bytes)
                    .trim_matches('\0')
                    .to_string();
                TdxValue::String(s)
            }
            4 => {
                // Potential sub-structures, such as collision-data chunks
                let blob = self.read_bytes(remaining)?;
                if blob.starts_with(b"TMCD") {
                    let mut chunks = Vec::new();
                    if blob.len() >= 26 && &blob[10..14] == b"SEBD" && &blob[26..30] == b"GNRC" {
                        chunks.push(("TMCD".to_string(), TdxValue::Binary(blob[4..10].to_vec())));
                        chunks.push(("SEBD".to_string(), TdxValue::Binary(blob[14..26].to_vec())));
                        chunks.push(("GNRC".to_string(), TdxValue::Binary(blob[30..].to_vec())));
                        TdxValue::Container(chunks)
                    } else {
                        TdxValue::Binary(blob.to_vec())
                    }
                } else {
                    TdxValue::Binary(blob.to_vec())
                }
            }
            13 => {
                // KUID (8 bytes)
                if remaining == 8 {
                    let k1 = self.read_u32()? as i32;
                    let k2 = self.read_u32()? as i32;
                    // For now, let's keep it as two i32s if we want, or pack into i64
                    // Let's use the i64 variant of TdxValue::Kuid if we can,
                    // but I changed TdxValue::Kuid to (i32, i32) in my thought process.
                    // Let me check what I actually did.
                    TdxValue::Kuid(k1 as i64 | ((k2 as i64) << 32))
                } else {
                    TdxValue::Binary(self.read_bytes(remaining)?.to_vec())
                }
            }
            _ => TdxValue::Binary(self.read_bytes(remaining)?.to_vec()),
        };

        // Ensure we consumed entry_len
        self.offset = start_offset + entry_len;

        Ok((tag, value))
    }

    fn read_u8(&mut self) -> Result<u8, TdxError> {
        if self.offset + 1 > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let val = self.data[self.offset];
        self.offset += 1;
        Ok(val)
    }

    fn read_u32(&mut self) -> Result<u32, TdxError> {
        if self.offset + 4 > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let val = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        Ok(val)
    }

    fn read_f32(&mut self) -> Result<f32, TdxError> {
        if self.offset + 4 > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let val = f32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
        self.offset += 4;
        Ok(val)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], TdxError> {
        if self.offset + len > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let val = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(val)
    }
}
