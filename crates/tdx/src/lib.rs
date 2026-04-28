use serde::Serialize;

pub mod acs_bin;
pub mod error;
pub mod reader;
pub mod value;

use shadow_rs::shadow;

shadow!(build);

pub use error::TdxError;
pub use reader::TdxReader;
pub use value::TdxValue;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KuidInfo {
    pub user_id: i64,
    pub content_id: i64,
    pub version: u32,
    pub username: Option<String>,
    pub files: Vec<String>,
}

impl TdxValue {
    pub fn find_kuids(entries: &[(String, TdxValue)]) -> Vec<KuidInfo> {
        let mut kuids = Vec::new();
        Self::find_kuids_recursive(entries, &mut kuids);
        kuids
    }

    pub fn split_kuid(kuid: i64) -> (i64, i64, u32) {
        let val = kuid as u64;
        let user_id = (val & 0x1FFFFFF) as i64;
        // Handle 25-bit signed UserID
        let user_id = if user_id >= 0x1000000 {
            user_id - 0x2000000
        } else {
            user_id
        };
        let content_id = ((val >> 25) & 0xFFFFFFFF) as i64;
        // Handle 32-bit signed ContentID
        let content_id = if content_id >= 0x80000000 {
            content_id - 0x100000000
        } else {
            content_id
        };
        let version = ((val >> 57) & 0x7F) as u32;
        (user_id, content_id, version)
    }

    pub fn make_kuid(user_id: i64, content_id: i64, version: u32) -> i64 {
        let u = (user_id & 0x1FFFFFF) as u64;
        let c = (content_id & 0xFFFFFFFF) as u64;
        let v = version as u64 & 0x7F;
        (u | (c << 25) | (v << 57)) as i64
    }

    fn find_kuids_recursive(entries: &[(String, TdxValue)], kuids: &mut Vec<KuidInfo>) {
        for (_name, value) in entries {
            match value {
                TdxValue::Container(sub_entries) => {
                    // Check if this container is a KUID-like object
                    let mut user_id = None;
                    let mut content_id = None;
                    let mut version = 0;
                    let mut username = None;
                    let mut files = Vec::new();

                    for (sub_name, sub_value) in sub_entries {
                        match (sub_name.as_str(), sub_value) {
                            ("userid", TdxValue::Int32(i)) => user_id = Some(*i as i64),
                            ("userid", TdxValue::Int64(i)) => user_id = Some(*i),
                            ("contentid", TdxValue::Int32(i)) => content_id = Some(*i as i64),
                            ("contentid", TdxValue::Int64(i)) => content_id = Some(*i),
                            ("version", TdxValue::Int32(i)) => version = *i as u32,
                            ("username", TdxValue::String(s)) => username = Some(s.clone()),
                            ("file", TdxValue::String(s)) => files.push(s.clone()),
                            _ => {}
                        }
                    }

                    if let (Some(u), Some(c)) = (user_id, content_id) {
                        kuids.push(KuidInfo {
                            user_id: u,
                            content_id: c,
                            version,
                            username: username.clone(),
                            files: files.clone(),
                        });
                    }

                    // Also check for direct KUID values in sub-entries
                    Self::find_kuids_recursive(sub_entries, kuids);
                }
                TdxValue::Kuid(val) => {
                    let (u, c, v) = Self::split_kuid(*val);
                    kuids.push(KuidInfo {
                        user_id: u,
                        content_id: c,
                        version: v,
                        username: None,
                        files: Vec::new(),
                    });
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::value::TdxValue;

    #[test]
    fn test_find_kuids() {
        let entries = vec![
            (
                "asset".to_string(),
                TdxValue::Container(vec![
                    ("userid".to_string(), TdxValue::Int64(1234567)),
                    ("contentid".to_string(), TdxValue::Int64(9876543)),
                    ("version".to_string(), TdxValue::Int32(2)),
                    (
                        "username".to_string(),
                        TdxValue::String("testuser".to_string()),
                    ),
                    (
                        "file".to_string(),
                        TdxValue::String("file1.txt".to_string()),
                    ),
                    (
                        "file".to_string(),
                        TdxValue::String("file2.txt".to_string()),
                    ),
                ]),
            ),
            (
                "direct_kuid".to_string(),
                TdxValue::Kuid(TdxValue::make_kuid(1, 2, 3)),
            ),
            (
                "direct_kuid2".to_string(),
                TdxValue::Kuid(TdxValue::make_kuid(100, 200, 127)),
            ),
        ];

        let kuids = TdxValue::find_kuids(&entries);
        assert_eq!(kuids.len(), 3);

        let info = &kuids[0];
        assert_eq!(info.user_id, 1234567);
        assert_eq!(info.content_id, 9876543);
        assert_eq!(info.version, 2);
        assert_eq!(info.username.as_deref(), Some("testuser"));
        assert_eq!(info.files, vec!["file1.txt", "file2.txt"]);

        assert_eq!(kuids[1].user_id, 1);
        assert_eq!(kuids[1].content_id, 2);
        assert_eq!(kuids[1].version, 3);

        assert_eq!(kuids[2].user_id, 100);
        assert_eq!(kuids[2].content_id, 200);
        assert_eq!(kuids[2].version, 127);
    }
}
