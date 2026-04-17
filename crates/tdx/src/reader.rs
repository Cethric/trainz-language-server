use crate::acs_bin::AcsParser;
use crate::error::TdxError;
use crate::value::TdxValue;

pub struct TdxReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> TdxReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn is_marker_like(&self, offset: usize) -> bool {
        if offset + 1 > self.data.len() {
            return false;
        }
        let len = self.data[offset] as usize;
        if len == 0 || len > 64 || offset + 1 + len > self.data.len() {
            return false;
        }
        let marker = &self.data[offset + 1..offset + 1 + len];
        if marker.is_empty() {
            return false;
        }
        // Markers usually contain a semicolon and are uppercase
        if marker.contains(&b';') {
            return marker
                .iter()
                .all(|&b| b.is_ascii_uppercase() || b == b';' || b.is_ascii_digit() || b == b'_');
        }
        // Some older markers might not have a semicolon, but let's be strict for now to avoid false positives
        false
    }

    pub fn read_asset_header(&mut self) -> Result<(String, TdxValue), TdxError> {
        if !self.is_marker_like(self.offset) {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let marker_len = self.data[self.offset] as usize;
        self.offset += 1;
        let marker =
            String::from_utf8_lossy(&self.data[self.offset..self.offset + marker_len]).to_string();
        self.offset += marker_len;

        if self.offset + 1 > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let name_len = self.data[self.offset] as usize;
        self.offset += 1;
        if self.offset + name_len > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let name_bytes = &self.data[self.offset..self.offset + name_len];
        let mut name = String::from_utf8_lossy(name_bytes)
            .trim_matches('\0')
            .to_string();
        if name.is_empty() {
            name = format!(":{}", self.offset);
        }
        self.offset += name_len;

        Ok((name.clone(), TdxValue::AssetHeader { marker, name }))
    }

    pub fn read_path_entry(&mut self) -> Result<(String, TdxValue), TdxError> {
        for prefix in [
            b"fld:".as_slice(),
            b"kuid:".as_slice(),
            b"file:".as_slice(),
            b"pkg:".as_slice(),
        ] {
            if self.offset + prefix.len() <= self.data.len()
                && &self.data[self.offset..self.offset + prefix.len()] == prefix
            {
                let mut end = self.offset + prefix.len();
                while end < self.data.len() && self.data[end] != b'|' && end < self.offset + 1024 {
                    end += 1;
                }
                if end < self.data.len() && self.data[end] == b'|' {
                    let path_str =
                        String::from_utf8_lossy(&self.data[self.offset..end + 1]).to_string();
                    let start = self.offset;
                    self.offset = end + 1;
                    return Ok((format!("path_{:x}", start), TdxValue::AssetPath(path_str)));
                }
            }
        }
        Err(TdxError::UnexpectedEof(self.offset))
    }

    pub fn parse_all(&mut self) -> Result<Vec<(String, TdxValue)>, TdxError> {
        // Look for the footer first
        let footer = if self.data.len() >= 4 {
            // Strategy: find the last few strings and see if they match the count.
            // For now, let's use the pattern we found: 4-byte count + Pascal strings.
            // We'll search backwards for a potential count that matches the number of strings following it.

            let mut best_footer = None;

            // Try a few offsets from the end
            for offset in
                (self.data.len().saturating_sub(2048)..self.data.len().saturating_sub(4)).rev()
            {
                if offset + 4 > self.data.len() {
                    continue;
                }
                let count =
                    u32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap()) as usize;
                if count > 0 && count < 1000 {
                    let mut current_pos = offset + 4;
                    let mut temp_strings = Vec::new();
                    let mut valid = true;
                    for _ in 0..count {
                        if current_pos >= self.data.len() {
                            valid = false;
                            break;
                        }
                        let len = self.data[current_pos] as usize;
                        current_pos += 1;
                        if current_pos + len > self.data.len() {
                            valid = false;
                            break;
                        }
                        let s = String::from_utf8_lossy(&self.data[current_pos..current_pos + len])
                            .to_string();
                        // Basic sanity check: mostly printable ASCII
                        if !s
                            .chars()
                            .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace() || c == ':')
                        {
                            valid = false;
                            break;
                        }
                        temp_strings.push(s);
                        current_pos += len;
                    }
                    if valid && current_pos == self.data.len() {
                        best_footer = Some((offset, temp_strings));
                        break;
                    }
                }
            }
            best_footer
        } else {
            None
        };

        let limit = footer
            .as_ref()
            .map(|(off, _)| *off)
            .unwrap_or(self.data.len());

        let mut results = Vec::new();

        // Handle Magic and Header
        if self.data.len() >= 4 && self.data[0..4] == [0xdb, 0xa2, 0xeb, 0xce] {
            results.push((
                "magic".to_string(),
                TdxValue::Binary(self.data[0..4].to_vec()),
            ));
            self.offset = 4;

            // Header: [v: 4] [total_size: 4] [name_len: 1] [name: "standard-31"]
            if self.offset + 9 <= limit {
                let v =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap());
                let total_size = u32::from_le_bytes(
                    self.data[self.offset + 4..self.offset + 8]
                        .try_into()
                        .unwrap(),
                );
                let name_len = self.data[self.offset + 8] as usize;
                if self.offset + 9 + name_len <= limit {
                    let name = String::from_utf8_lossy(
                        &self.data[self.offset + 9..self.offset + 9 + name_len],
                    )
                    .to_string();
                    if name == "standard-31" {
                        results.push(("header_v".to_string(), TdxValue::Int32(v as i32)));
                        results.push((
                            "header_total_size".to_string(),
                            TdxValue::Int32(total_size as i32),
                        ));
                        results.push(("header_name".to_string(), TdxValue::String(name)));
                        self.offset += 9 + name_len;
                    }
                }
            }
        }

        while self.offset < limit {
            let start_offset = self.offset;

            // 0. Try ACS$ block
            if self.offset + 4 <= limit && &self.data[self.offset..self.offset + 4] == b"ACS$" {
                let mut parser = AcsParser::new(&self.data[self.offset..limit]);
                match parser.parse() {
                    Ok(value) => {
                        results.push((format!("acs_{:x}", start_offset), value));
                        self.offset += parser.consumed();
                        continue;
                    }
                    Err(_) => {
                        // Fallback: Skip 16 bytes if possible (typical header size)
                        if self.offset + 16 <= limit {
                            self.offset += 16;
                        } else {
                            self.offset += 4;
                        }
                        continue;
                    }
                }
            }

            // 1. Try AssetHeader (Markers)
            if self.is_marker_like(self.offset) {
                if let Ok((name, value)) = self.read_asset_header() {
                    results.push((name, value));
                    continue;
                } else {
                    self.offset = start_offset;
                }
            }

            // 2. Try Path
            if let Ok((name, value)) = self.read_path_entry() {
                results.push((name, value));
                continue;
            } else {
                self.offset = start_offset;
            }

            // 3. Try TypedProperty (8-byte blocks)
            if self.offset + 8 <= limit {
                let sig = u32::from_le_bytes(
                    self.data[self.offset + 4..self.offset + 8]
                        .try_into()
                        .unwrap(),
                );
                // Common signatures
                let is_sig = match sig & 0xFFFFFF00 {
                    0x0A172100 => true, // 21 17 0a XX
                    0x00020B00 => true, // 0b 02 00 XX
                    0x00DF8E00 => true, // 8e df 00 XX
                    0x021D6F00 => true, // 6f 1d 02 XX
                    0x07B8B200 => true, // b2 b8 07 XX
                    0x02172100 => true, // 21 17 02 XX
                    0x02020B00 => true, // 0b 02 02 XX
                    0x00080A75 => true, // 75 0a 08 XX
                    0x00010A75 => true, // 75 0a 01 XX
                    _ => sig == 0xFFFFFFFF,
                };

                if is_sig {
                    let val = u32::from_le_bytes(
                        self.data[self.offset..self.offset + 4].try_into().unwrap(),
                    );
                    results.push((
                        format!("prop_{:x}", self.offset),
                        TdxValue::TypedProperty {
                            value: val,
                            signature: sig,
                        },
                    ));
                    self.offset += 8;
                    continue;
                }
            }

            // 4. Try Entry
            if self.offset + 4 <= limit {
                match self.read_entry() {
                    Ok((name, value)) => {
                        results.push((name, value));
                        continue;
                    }
                    Err(_) => {
                        self.offset = start_offset;
                    }
                }
            }

            // 5. Fallback: Unknown byte
            let b = self.data[self.offset];
            if let Some((_, TdxValue::Unknown(_, last_bytes))) = results.last_mut() {
                last_bytes.push(b);
            } else {
                results.push((
                    format!("unk_{:x}", self.offset),
                    TdxValue::Unknown(b, Vec::new()),
                ));
            }
            self.offset += 1;
        }

        if let Some((_, strings)) = footer {
            results.push(("footer".to_string(), TdxValue::Footer { strings }));
        }

        Ok(results)
    }

    pub fn read_entry(&mut self) -> Result<(String, TdxValue), TdxError> {
        if self.offset + 4 > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }

        let s1 = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
            as usize;
        let _entry_start = self.offset;

        // Sanity check for s1
        if s1 == 0 || s1 > 100 * 1024 * 1024 {
            return Err(TdxError::UnexpectedEof(self.offset));
        }

        self.offset += 4;

        let mut _outer_size = s1;
        let mut inner_size = s1;

        if self.offset + 4 <= self.data.len() {
            let s2 = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                as usize;
            // Dual size heuristic
            // If s1 == s2 + 4, then s1 is the size including itself, and s2 is the size of the rest.
            if s1 == s2 + 4 || (s1 > s2 + 4 && s2 > 0 && s2 < s1 && s2 < 1024 * 1024) {
                _outer_size = s1;
                inner_size = s2;
                self.offset += 4;
            } else if s1 > 0 && s1 < 1024 * 1024 && s2 > s1 && s2 <= self.data.len() {
                // standard-31 case: s1 is entry size, s2 is total/outer size
                _outer_size = s2;
                inner_size = s1;
                self.offset += 4;
            }
            // If neither matches, we assume s1 is the inner_size and we didn't consume s2.
        }

        if self.offset >= self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let name_len = self.data[self.offset] as usize;
        self.offset += 1;

        if name_len == 0 || name_len > 128 || self.offset + name_len > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }

        let name_bytes = &self.data[self.offset..self.offset + name_len];
        let mut name = String::from_utf8_lossy(name_bytes)
            .trim_matches('\0')
            .to_string();
        if name.is_empty() {
            name = format!(":{}", self.offset);
        }
        self.offset += name_len;

        if self.offset >= self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let kind = self.data[self.offset];
        self.offset += 1;

        // The data_len should be inner_size minus name_len, minus 1 (name_len byte), minus 1 (kind byte).
        if inner_size < 1 + name_len + 1 {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let data_len = inner_size - (1 + name_len + 1);

        if self.offset + data_len > self.data.len() {
            return Err(TdxError::UnexpectedEof(self.offset));
        }
        let data_slice = &self.data[self.offset..self.offset + data_len];
        self.offset += data_len;

        let value = match kind {
            0x00 | 0x07 | 0x41 | 0x54 | 0x65 | 0x6d | 0x72 | 0x61 | 0x63 | 0x6e | 0x76 | 0x0e
            | 0x0f | 0x15 | 0x18 | 0x19 | 0x1a | 0x1b | 0x1c | 0x1d | 0x1e | 0x1f | 0x24 | 0x25
            | 0x3b | 0x3f => {
                let mut container = Vec::new();
                let mut sub_reader = TdxReader::new(data_slice);

                // Keep track of how much we've parsed to decide if it's really a container
                let mut last_pos = 0;
                while sub_reader.offset + 4 <= sub_reader.data.len() {
                    let start = sub_reader.offset;
                    if let Ok((k, v)) = sub_reader.read_entry() {
                        container.push((k, v));
                        last_pos = sub_reader.offset;
                    } else {
                        // If it's not a valid entry, maybe it's an AssetHeader, Path or TypedProperty
                        sub_reader.offset = start;
                        if sub_reader.is_marker_like(sub_reader.offset) {
                            if let Ok((k, v)) = sub_reader.read_asset_header() {
                                container.push((k, v));
                                last_pos = sub_reader.offset;
                                continue;
                            }
                            sub_reader.offset = start;
                        }

                        if let Ok((k, v)) = sub_reader.read_path_entry() {
                            container.push((k, v));
                            last_pos = sub_reader.offset;
                            continue;
                        }
                        sub_reader.offset = start;

                        // TypedProperty heuristic inside container
                        if sub_reader.offset + 8 <= sub_reader.data.len() {
                            let sig = u32::from_le_bytes(
                                sub_reader.data[sub_reader.offset + 4..sub_reader.offset + 8]
                                    .try_into()
                                    .unwrap(),
                            );
                            let is_sig = match sig & 0xFFFFFF00 {
                                0x0A172100 => true,
                                0x00020B00 => true,
                                0x00DF8E00 => true,
                                0x021D6F00 => true,
                                0x07B8B200 => true,
                                0x02172100 => true,
                                0x02020B00 => true,
                                0x00080A75 => true,
                                0x00010A75 => true,
                                0x00000000 => true,
                                _ => sig == 0xFFFFFFFF,
                            };
                            if is_sig {
                                let val = u32::from_le_bytes(
                                    sub_reader.data[sub_reader.offset..sub_reader.offset + 4]
                                        .try_into()
                                        .unwrap(),
                                );
                                if sig != 0 || val != 0 {
                                    container.push((
                                        format!("prop_{:x}", sub_reader.offset),
                                        TdxValue::TypedProperty {
                                            value: val,
                                            signature: sig,
                                        },
                                    ));
                                }
                                sub_reader.offset += 8;
                                last_pos = sub_reader.offset;
                                continue;
                            }
                        }

                        // Just skip one byte and mark as unknown if we're really stuck
                        let b = sub_reader.data[sub_reader.offset];
                        if let Some((_, TdxValue::Unknown(_, last_bytes))) = container.last_mut() {
                            last_bytes.push(b);
                        } else {
                            container.push((
                                format!("unk_{:x}", sub_reader.offset),
                                TdxValue::Unknown(b, Vec::new()),
                            ));
                        }
                        sub_reader.offset += 1;
                        last_pos = sub_reader.offset;
                    }
                }

                // If we didn't parse any valid entries and only have Unknown blocks, it's likely not a container
                let has_valid_entry = container
                    .iter()
                    .any(|(_, v)| !matches!(v, TdxValue::Unknown(_, _)));
                if !has_valid_entry && !data_slice.is_empty() {
                    TdxValue::Binary(data_slice.to_vec())
                } else {
                    // Collect remaining bytes in container if any
                    if last_pos < data_slice.len() {
                        let rem = &data_slice[last_pos..];
                        if let Some((_, TdxValue::Unknown(_, last_bytes))) = container.last_mut() {
                            last_bytes.extend_from_slice(rem);
                        } else {
                            container.push(("rem".to_string(), TdxValue::Binary(rem.to_vec())));
                        }
                    }
                    TdxValue::Container(container)
                }
            }
            0x01 | 0x10 | 0x11 => {
                if data_slice.len() >= 4 {
                    TdxValue::Int32(i32::from_le_bytes(data_slice[0..4].try_into().unwrap()))
                } else {
                    TdxValue::Unknown(kind, data_slice.to_vec())
                }
            }
            0x02 => {
                if data_slice.len() >= 8 {
                    TdxValue::Float64(f64::from_le_bytes(data_slice[0..8].try_into().unwrap()))
                } else {
                    TdxValue::Unknown(kind, data_slice.to_vec())
                }
            }
            0x03 => TdxValue::String(
                String::from_utf8_lossy(data_slice)
                    .trim_matches('\0')
                    .to_string(),
            ),
            0x04 => {
                if !data_slice.is_empty() {
                    TdxValue::Bool(data_slice[0] != 0)
                } else {
                    TdxValue::Unknown(kind, data_slice.to_vec())
                }
            }
            0x05 | 0x0b => {
                if data_slice.len() >= 8 {
                    TdxValue::Kuid(i64::from_le_bytes(data_slice[0..8].try_into().unwrap()))
                } else if data_slice.len() >= 4 {
                    // Possible single 32-bit KUID? Trainz usually uses more.
                    TdxValue::Int32(i32::from_le_bytes(data_slice[0..4].try_into().unwrap()))
                } else {
                    TdxValue::Unknown(kind, data_slice.to_vec())
                }
            }
            0x0c => {
                if data_slice.len() >= 8 {
                    TdxValue::Kuid(i64::from_le_bytes(data_slice[0..8].try_into().unwrap()))
                } else {
                    TdxValue::Unknown(kind, data_slice.to_vec())
                }
            }
            0x0d => {
                if data_slice.len() >= 8 {
                    TdxValue::Int64(i64::from_le_bytes(data_slice[0..8].try_into().unwrap()))
                } else {
                    TdxValue::Unknown(kind, data_slice.to_vec())
                }
            }
            _ => TdxValue::Unknown(kind, data_slice.to_vec()),
        };

        self.offset = _entry_start
            + 4
            + if s1 == inner_size + 4 {
                s1
            } else if s1 == inner_size {
                s1 + 4
            } else {
                inner_size
            };
        Ok((name, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let mut data = Vec::new();
        data.extend_from_slice(&10u32.to_le_bytes());
        data.push(4);
        data.extend_from_slice(b"test");
        data.push(0x01);
        data.extend_from_slice(&42i32.to_le_bytes());

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        assert_eq!(
            results.iter().find(|(k, _)| k == "test").map(|(_, v)| v),
            Some(&TdxValue::Int32(42))
        );
    }

    #[test]
    fn test_parse_container() {
        let mut data = Vec::new();
        let mut inner = Vec::new();
        inner.extend_from_slice(&10u32.to_le_bytes());
        inner.push(4);
        inner.extend_from_slice(b"test");
        inner.push(0x01);
        inner.extend_from_slice(&42i32.to_le_bytes());

        data.extend_from_slice(&21u32.to_le_bytes());
        data.push(5);
        data.extend_from_slice(b"outer");
        data.push(0x00);
        data.extend_from_slice(&inner);

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        let outer = results
            .iter()
            .find(|(k, _)| k == "outer")
            .map(|(_, v)| v)
            .unwrap();
        if let TdxValue::Container(vec) = outer {
            assert_eq!(
                vec.iter().find(|(k, _)| k == "test").map(|(_, v)| v),
                Some(&TdxValue::Int32(42))
            );
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn test_resync_txnev() {
        let mut data = Vec::new();
        // First entry
        let entry1_size: u32 = 1 + 6 + 1 + 4; // name_len + "entry1" + kind + i32
        data.extend_from_slice(&entry1_size.to_le_bytes());
        data.push(6);
        data.extend_from_slice(b"entry1");
        data.push(0x01);
        data.extend_from_slice(&10i32.to_le_bytes());

        // Garbage
        data.extend_from_slice(b"SOME_GARBAGE_DATA_THAT_FAILS_PARSING");

        // 0x06 + TX;NEV
        data.push(0x06);
        data.extend_from_slice(b"TX;NEV");
        data.push(0); // name_len
        data.extend_from_slice(&[0; 8]); // unk
        data.extend_from_slice(&[1, 0, 0, 0]); // path_len
        data.push(0); // path data
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // footer

        // Second entry
        let entry2_size: u32 = 1 + 6 + 1 + 4; // name_len + "entry2" + kind + i32
        data.extend_from_slice(&entry2_size.to_le_bytes());
        data.push(6);
        data.extend_from_slice(b"entry2");
        data.push(0x01);
        data.extend_from_slice(&20i32.to_le_bytes());

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();

        let val1 = results.iter().find(|(k, _)| k == "entry1").map(|(_, v)| v);
        assert_eq!(val1, Some(&TdxValue::Int32(10)));

        let val2 = results.iter().find(|(k, _)| k == "entry2").map(|(_, v)| v);
        assert_eq!(val2, Some(&TdxValue::Int32(20)));
    }

    #[test]
    fn test_parse_n_kind_as_container() {
        let mut data = Vec::new();
        let mut inner = Vec::new();
        // Sub-entry in 'n' kind
        let sub_size: u32 = 1 + 4 + 1 + 4; // name_len + "sub1" + kind + i32
        inner.extend_from_slice(&sub_size.to_le_bytes());
        inner.push(4);
        inner.extend_from_slice(b"sub1");
        inner.push(0x01);
        inner.extend_from_slice(&99i32.to_le_bytes());

        // Outer entry with kind 'n' (0x6e)
        let outer_size: u32 = 1 + 5 + 1 + inner.len() as u32; // name_len + "outer" + kind + data
        data.extend_from_slice(&outer_size.to_le_bytes());
        data.push(5);
        data.extend_from_slice(b"outer");
        data.push(0x6e);
        data.extend_from_slice(&inner);

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        let outer = results
            .iter()
            .find(|(k, _)| k == "outer")
            .map(|(_, v)| v)
            .unwrap();
        if let TdxValue::Container(vec) = outer {
            assert_eq!(
                vec.iter().find(|(k, _)| k == "sub1").map(|(_, v)| v),
                Some(&TdxValue::Int32(99))
            );
        } else {
            panic!("Expected container for kind 0x6e when it has sub-entries");
        }
    }

    #[test]
    fn test_parse_n_kind_as_binary() {
        let mut data = Vec::new();
        let binary_data = b"NOT_A_CONTAINER_STRUCTURE";

        let outer_size: u32 = 1 + 5 + 1 + binary_data.len() as u32;
        data.extend_from_slice(&outer_size.to_le_bytes());
        data.push(5);
        data.extend_from_slice(b"outer");
        data.push(0x6e);
        data.extend_from_slice(binary_data);

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        let outer = results
            .iter()
            .find(|(k, _)| k == "outer")
            .map(|(_, v)| v)
            .unwrap();
        if let TdxValue::Binary(vec) = outer {
            assert_eq!(vec, binary_data);
        } else {
            panic!("Expected binary for kind 0x6e when it has no sub-entries");
        }
    }

    #[test]
    fn test_parse_kuid() {
        let mut data = Vec::new();
        let kuid_val = TdxValue::make_kuid(1, 2, 3);

        // KUID: <kuid2:1:2:3> (8 bytes)
        let name1 = b"kuid1";
        let inner1_size: u32 = 1 + name1.len() as u32 + 1 + 8; // len + name + kind + 8
        data.extend_from_slice(&(inner1_size + 4).to_le_bytes()); // s1
        data.extend_from_slice(&inner1_size.to_le_bytes()); // s2
        data.push(name1.len() as u8);
        data.extend_from_slice(name1);
        data.push(0x05); // KUID kind
        data.extend_from_slice(&kuid_val.to_le_bytes());

        // KUID: <kuid2:1:2:3> (8 bytes) again with kind 0x0b
        let name2 = b"kuid2";
        let inner2_size: u32 = 1 + name2.len() as u32 + 1 + 8;
        data.extend_from_slice(&(inner2_size + 4).to_le_bytes()); // s1
        data.extend_from_slice(&inner2_size.to_le_bytes()); // s2
        data.push(name2.len() as u8);
        data.extend_from_slice(name2);
        data.push(0x0b); // KUID kind
        data.extend_from_slice(&kuid_val.to_le_bytes());

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        assert_eq!(
            results.iter().find(|(k, _)| k == "kuid1").map(|(_, v)| v),
            Some(&TdxValue::Kuid(kuid_val))
        );
        assert_eq!(
            results.iter().find(|(k, _)| k == "kuid2").map(|(_, v)| v),
            Some(&TdxValue::Kuid(kuid_val))
        );
    }

    #[test]
    fn test_parse_asset_descriptor() {
        let mut data = Vec::new();
        // Asset Descriptor 1
        data.push(0x10);
        data.extend_from_slice(b"TX;NEV;TXGRP;CMP");
        data.push(7);
        data.extend_from_slice(b"NameOne");
        data.extend_from_slice(&[0; 8]); // unk
        let path1 = b"fld:path1|";
        data.extend_from_slice(&(path1.len() as u32).to_le_bytes());
        data.extend_from_slice(path1);
        data.extend_from_slice(b"some_params");

        // Asset Descriptor 2 (no separator, just next one)
        data.push(0x10);
        data.extend_from_slice(b"TX;NEV;TXGRP;CMP");
        data.push(7);
        data.extend_from_slice(b"NameTwo");
        data.extend_from_slice(&[0; 8]); // unk
        let path2 = b"fld:path2|";
        data.extend_from_slice(&(path2.len() as u32).to_le_bytes());
        data.extend_from_slice(path2);
        data.extend_from_slice(b"more_params");
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // footer

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();

        // We expect Header, Path, etc.
        assert!(
            results
                .iter()
                .any(|(n, v)| n == "NameOne" && matches!(v, TdxValue::AssetHeader { .. }))
        );
        assert!(
            results
                .iter()
                .any(|(_, v)| matches!(v, TdxValue::AssetPath(p) if p == "fld:path1|"))
        );
        assert!(
            results
                .iter()
                .any(|(n, v)| n == "NameTwo" && matches!(v, TdxValue::AssetHeader { .. }))
        );
        assert!(
            results
                .iter()
                .any(|(_, v)| matches!(v, TdxValue::AssetPath(p) if p == "fld:path2|"))
        );
    }

    #[test]
    fn test_parse_asset_descriptor_v2() {
        let mut data = Vec::new();
        // Asset Descriptor v2 (0x0F + BD;RRD;SCEN;SY;)
        data.push(0x0F);
        data.extend_from_slice(b"BD;RRD;SCEN;SY;");
        let name_val = b"1435mm.BufferOld.v1\0\0\0";
        data.push(name_val.len() as u8);
        data.extend_from_slice(name_val);
        data.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 0]); // unk
        let _path = b"fld:something|"; // path doesn't have length in the user provided hex?
        // Wait, the user hex: 0F 42 44 3B ... 13 31 34 33 35 6D 6D 2E 42 75 66 66 65 72 4F 6C 64 2E 76 31 00 00 00 01 00 00 00 00 01 00 00 00 00
        // BD;RRD;SCEN;SY; is 15 bytes.
        // 0x13 is 19. "1435mm.BufferOld.v1" is 19 bytes.
        // So 0x13 is the name length.
        // Then 8 bytes unk.
        // Then path?
        // Let's re-examine: 00 00 00 01 00 00 00 00 01 00 00 00 00 01 00 00 00 00 01 00 00 00 00 FF FF FF FF ...
        // In my current read_asset_descriptor:
        // marker length (1) -> marker
        // name length (1) -> name
        // unk (8)
        // path length (4) -> path
        // params until next marker

        // Let's look at the hex again:
        // 0F marker_BD;RRD;SCEN;SY; (15)
        // 13 "1435mm.BufferOld.v1" (19)
        // 00 00 00 01 00 00 00 00 (8 bytes unk)
        // 01 00 00 00 (path_len = 1)
        // 00 (path = "") ? or is it "0" ?
        // or maybe the next 4 bytes 01 00 00 00 is NOT path length.

        // Actually, in the user hex:
        // ... 13 31 34 33 35 6D 6D 2E 42 75 66 66 65 72 4F 6C 64 2E 76 31 (Name)
        // 00 00 00 01 00 00 00 00 (8 bytes unk)
        // 01 00 00 00 (4 bytes, looks like path_len=1)
        // 00 (1 byte path?)
        // 01 00 00 00 (4 bytes?)
        // 00 (1 byte?)
        // ...

        // Wait, if path_len is 4 bytes LE, then 01 00 00 00 is indeed 1.
        // If it's 1, then the next byte 00 is the path (empty string after trim?).

        let mut hex = Vec::new();
        // 0F BD;RRD;SCEN;SY;
        hex.push(0x0F);
        hex.extend_from_slice(b"BD;RRD;SCEN;SY;");
        // 13 "1435mm.BufferOld.v1"
        hex.push(0x13);
        hex.extend_from_slice(b"1435mm.BufferOld.v1");
        // 8 bytes unk
        hex.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 0]);
        // 4 bytes path len (1)
        hex.extend_from_slice(&[1, 0, 0, 0]);
        // 1 byte path
        hex.push(0);
        // some params
        hex.extend_from_slice(&[1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
        hex.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

        let mut reader = TdxReader::new(&hex);
        let results = reader.parse_all().unwrap();
        // Skip unknown bytes from start
        let descriptor = results
            .iter()
            .find(|(_, v)| matches!(v, TdxValue::AssetHeader { .. }));
        assert!(descriptor.is_some(), "Expected AssetHeader");
        let (name, val) = descriptor.unwrap();
        assert_eq!(name, "1435mm.BufferOld.v1");
        if let TdxValue::AssetHeader { marker, .. } = val {
            assert_eq!(marker, "BD;RRD;SCEN;SY;");
        } else {
            panic!("Expected AssetHeader");
        }
    }

    #[test]
    fn test_parse_asset_descriptor_v3() {
        let mut data = Vec::new();
        // 0x14 + FRT;HOPP;ROLL;TR;TV;
        data.push(0x14);
        data.extend_from_slice(b"FRT;HOPP;ROLL;TR;TV;");
        // name: "Westrail RCH Grain Wagon V2" (27 bytes = 0x1B)
        data.push(0x1B);
        data.extend_from_slice(b"Westrail RCH Grain Wagon V2");
        // 8 bytes unk
        data.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 0]);
        // 4 bytes path len (1)
        data.extend_from_slice(&[1, 0, 0, 0]);
        // 1 byte path
        data.push(0);
        // params
        data.extend_from_slice(&[1, 0, 0, 0, 1, 0, 0, 0]);

        // Next descriptor: 0x07 + SIN;TO;
        data.push(0x07);
        data.extend_from_slice(b"SIN;TO;");
        let name2 = b"Load - Unload Marker 5KPH LHS"; // 29 bytes = 0x1D
        data.push(0x1D);
        data.extend_from_slice(name2);
        data.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 0]);
        data.extend_from_slice(&[1, 0, 0, 0]);
        data.push(0);
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        let headers: Vec<_> = results
            .iter()
            .filter(|(_, v)| matches!(v, TdxValue::AssetHeader { .. }))
            .collect();
        assert_eq!(headers.len(), 2);

        assert_eq!(headers[0].0, "Westrail RCH Grain Wagon V2");
        if let TdxValue::AssetHeader { marker, .. } = &headers[0].1 {
            assert_eq!(marker, "FRT;HOPP;ROLL;TR;TV;");
        } else {
            panic!("Expected AssetHeader 1");
        }

        assert_eq!(headers[1].0, "Load - Unload Marker 5KPH LHS");
        if let TdxValue::AssetHeader { marker, .. } = &headers[1].1 {
            assert_eq!(marker, "SIN;TO;");
        } else {
            panic!("Expected AssetHeader 2");
        }
    }

    #[test]
    fn test_resync_generic() {
        let mut data = Vec::new();
        // Garbage
        data.extend_from_slice(b"JUNK_DATA");

        // 0x03 + SS;
        data.push(0x03);
        data.extend_from_slice(b"SS;");
        // name
        data.push(4);
        data.extend_from_slice(b"Test");
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&[1, 0, 0, 0]);
        data.push(0);
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

        let mut reader = TdxReader::new(&data);
        let results = reader.parse_all().unwrap();
        let header = results
            .iter()
            .find(|(_, v)| matches!(v, TdxValue::AssetHeader { .. }));
        assert!(header.is_some(), "Expected AssetHeader");
        let (name, val) = header.unwrap();
        assert_eq!(name, "Test");
        if let TdxValue::AssetHeader { marker, .. } = val {
            assert_eq!(marker, "SS;");
        } else {
            panic!("Expected AssetHeader");
        }
    }

    #[test]
    fn test_parse_asset_descriptor_v4() {
        let hex_str = "54 58 3B 4E 45 56 3B 54 58 47 52 50 3B 43 4D 50 15 43 61 6D 65 72 61 20 69 63 6F 6E 20 2D 20 52 6F 61 6D 69 6E 67 02 30 30 05 32 30 31 30 73 00 01 00 00 00 00 2E 00 00 00 66 6C 64 3A 24 28 62 75 69 6C 74 69 6E 29 2F 62 61 73 65 2F 63 6F 6E 74 65 6E 74 2F 6B 75 69 64 20 34 34 37 32 36 34 20 31 31 30 38 7C 00 02 00 00 00 21 00 01 00 00 00 00 FF FF FF FF FF FF FF FF 01 00 00 00 01 00 01 01 01 01 00 01 00 00 00 01 01 01 01 00 01 00 01 01 FF FF FF FF FF FF FF FF 00 01 00 00 00 00 D7 08 00 00 99 99 59 40 FF 9A DB B5 68 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 9B EE 31 78 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 C2 18 00 00 FF FF FF FF 06 43 4D 50 3B 54 58 09 4C 69 67 68 74 6E 69 6E 67 00 00 00 01 00 00 00 00 2A 00 00 00 66 6C 64 3A 24 28 62 75 69 6C 74 69 6E 29 2F 62 61 73 65 2F 63 6F 6E 74 65 6E 74 2F 6B 75 69 64 20 2D 31 20 36 33 33 38 7C 00 02 00 00 00 21 00 01 00 00 00 00 FF FF FF FF FF FF FF FF 01 00 00 00 01 00 01 01 01 01 00 01 00 00 00 01 01 01 01 00 01 00 01 01 FF FF FF FF FF FF FF FF 00 01 00 00 00 00 2A 17 00 00 66 66 A6 3F FF 9A DB B5 68 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 00 00 00 00 00 A3 92 F2 A9 00 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 4C 18 00 00 FF FF FF FF";
        let hex: Vec<u8> = hex_str
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect();

        // The provided hex starts with "TX;NEV;TXGRP;CMP" which is 16 bytes.
        // So the first byte should be 0x10.
        let mut full_hex = vec![0x10];
        full_hex.extend_from_slice(&hex);

        let mut reader = TdxReader::new(&full_hex);
        let results = reader.parse_all().unwrap();

        let val1 = results
            .iter()
            .find(|(k, _)| k == "Camera icon - Roaming")
            .map(|(_, v)| v)
            .expect("Missing Camera icon");
        if let TdxValue::AssetHeader { marker, .. } = val1 {
            assert_eq!(marker, "TX;NEV;TXGRP;CMP");
        } else {
            panic!("Expected AssetHeader 1, got {:?}", val1);
        }

        // The second descriptor is "2010s" with marker "00"
        // Wait, "00" is NOT a marker in our new strict logic because it doesn't contain semicolon.
        // So "2010s" will probably be parsed as a standard entry if it looks like one.
        // In this test, it's just raw bytes.
    }

    #[test]
    fn test_panic_truncated_descriptor_in_container() {
        let mut data = Vec::new();
        let name = b"cont";
        let marker = b"TX";
        let descriptor_data = vec![0x02, marker[0], marker[1], 0x05]; // name_len = 5, but only 0 bytes left

        let inner_size = (1 + name.len() + 1 + descriptor_data.len()) as u32;
        data.extend_from_slice(&(inner_size + 4).to_le_bytes()); // s1
        data.extend_from_slice(&inner_size.to_le_bytes()); // s2
        data.push(name.len() as u8);
        data.extend_from_slice(name);
        data.push(0x00); // Container kind
        data.extend_from_slice(&descriptor_data);

        let mut reader = TdxReader::new(&data);
        let _ = reader.parse_all();
    }
}
