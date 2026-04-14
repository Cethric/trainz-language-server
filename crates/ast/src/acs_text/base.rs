use crate::acs_text::Kuid;
use crate::acs_text::Value;
use crate::acs_text::key_value_pair::KeyValuePair;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcsText {
    pub key_value_pairs: Vec<KeyValuePair>,
    pub range: crate::Range,
    pub src: String,
}

impl AcsText {
    pub fn find_all_kuids(&self) -> Vec<Kuid> {
        let mut kuids = Vec::new();
        for kvp in &self.key_value_pairs {
            self.find_kuids_in_kvp(kvp, &mut kuids);
        }
        kuids
    }

    pub fn find_kuid_at(&self, pos: crate::Position) -> Option<Kuid> {
        for kvp in &self.key_value_pairs {
            if let Some(kuid) = self.find_kuid_at_kvp(kvp, pos) {
                return Some(kuid);
            }
        }
        None
    }

    fn find_kuid_at_kvp(&self, kvp: &KeyValuePair, pos: crate::Position) -> Option<Kuid> {
        if let Some(value) = &kvp.value {
            return self.find_kuid_at_value(value, pos);
        }
        None
    }

    fn find_kuid_at_value(&self, value: &Value, pos: crate::Position) -> Option<Kuid> {
        match value {
            Value::Kuid(kuid, range) => {
                if crate::find::position_in_range(pos, *range) {
                    return Some(kuid.clone());
                }
            }
            Value::Container(kvps, _, _) => {
                for kvp in kvps {
                    if let Some(kuid) = self.find_kuid_at_kvp(kvp, pos) {
                        return Some(kuid);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn find_kuids_in_kvp(&self, kvp: &KeyValuePair, kuids: &mut Vec<Kuid>) {
        if let Some(value) = &kvp.value {
            self.find_kuids_in_value(value, kuids);
        }
    }

    fn find_kuids_in_value(&self, value: &Value, kuids: &mut Vec<Kuid>) {
        match value {
            Value::Kuid(kuid, _) => kuids.push(kuid.clone()),
            Value::Container(kvps, _, _) => {
                for kvp in kvps {
                    self.find_kuids_in_kvp(kvp, kuids);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;

    #[test]
    fn test_kuid_version_logic() {
        let mut kuid = Kuid {
            user_id: 123,
            content_id: 456,
            version: None,
            range: crate::Range::default(),
        };

        assert!(kuid.can_increment());
        assert!(!kuid.can_decrement());
        assert_eq!(kuid.to_string(), "<kuid:123:456>");

        kuid.increment();
        assert_eq!(kuid.version, Some(2));
        assert!(kuid.can_decrement());
        assert_eq!(kuid.to_string(), "<kuid2:123:456:2>");

        kuid.decrement();
        assert_eq!(kuid.version, Some(1));
        assert!(!kuid.can_decrement());
        assert_eq!(kuid.to_string(), "<kuid2:123:456:1>");

        kuid.increment();
        assert_eq!(kuid.version, Some(2));
    }

    #[test]
    fn test_acs_text_find_kuids() {
        let kuid1 = Kuid {
            user_id: 1,
            content_id: 1,
            version: None,
            range: crate::Range {
                start: Position {
                    line: 0,
                    character: 10,
                },
                end: Position {
                    line: 0,
                    character: 20,
                },
            },
        };

        let kuid2 = Kuid {
            user_id: 1,
            content_id: 1,
            version: Some(2),
            range: crate::Range {
                start: Position {
                    line: 1,
                    character: 10,
                },
                end: Position {
                    line: 1,
                    character: 20,
                },
            },
        };

        let r1 = kuid1.range;
        let r2 = kuid2.range;

        let acs_text = AcsText {
            key_value_pairs: vec![
                KeyValuePair {
                    key: "k1".to_string(),
                    key_range: crate::Range::default(),
                    value: Some(Value::Kuid(kuid1, r1)),
                    range: crate::Range::default(),
                },
                KeyValuePair {
                    key: "k2".to_string(),
                    key_range: crate::Range::default(),
                    value: Some(Value::Kuid(kuid2, r2)),
                    range: crate::Range::default(),
                },
            ],
            range: crate::Range::default(),
            src: "".to_string(),
        };

        let kuids = acs_text.find_all_kuids();
        assert_eq!(kuids.len(), 2);

        // Find at cursor
        let found = acs_text.find_kuid_at(Position {
            line: 0,
            character: 15,
        });
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.user_id, 1);
        assert_eq!(found.content_id, 1);
        assert_eq!(found.version, None);

        let found_none = acs_text.find_kuid_at(Position {
            line: 0,
            character: 25,
        });
        assert!(found_none.is_none());
    }
}
