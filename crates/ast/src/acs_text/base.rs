use crate::Position;
use crate::acs_text::Kuid;
use crate::acs_text::Value;
use crate::acs_text::key_value_pair::KeyValuePair;
use crate::find::position_in_range;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcsText {
    pub key_value_pairs: Vec<KeyValuePair>,
    pub range: crate::Range,
    pub src: String,
}

impl AcsText {
    pub fn get_text_at(&self, position: Position) -> Option<String> {
        let lines: Vec<&str> = self.src.split('\n').collect();
        if position.line as usize >= lines.len() {
            return None;
        }
        let mut line = lines[position.line as usize];
        if line.ends_with('\r') {
            line = &line[..line.len() - 1];
        }

        if line.is_empty() {
            return None;
        }

        let char_pos = position.character as usize;
        let bytes = line.as_bytes();

        let mut idx = if char_pos >= bytes.len() {
            bytes.len() - 1
        } else {
            char_pos
        };

        // If idx is whitespace, look left then right
        if bytes[idx].is_ascii_whitespace() {
            let mut left = idx;
            while left > 0 && bytes[left - 1].is_ascii_whitespace() {
                left -= 1;
            }
            if left > 0 {
                idx = left - 1;
            } else {
                let mut right = idx;
                while right < bytes.len() && bytes[right].is_ascii_whitespace() {
                    right += 1;
                }
                if right < bytes.len() {
                    idx = right;
                } else {
                    return None;
                }
            }
        }

        // Now idx should be on a non-whitespace character
        let mut start = idx;
        while start > 0 && !bytes[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        let mut end = idx;
        while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }

        Some(line[start..end].to_string())
    }

    pub fn find_all_kuids(&self) -> Vec<Kuid> {
        let mut kuids = Vec::new();
        for kvp in &self.key_value_pairs {
            self.find_kuids_in_kvp(kvp, &mut kuids);
        }
        kuids
    }

    pub fn find_kuid_at(&self, pos: Position) -> Option<Kuid> {
        for kvp in &self.key_value_pairs {
            if let Some(kuid) = self.find_kuid_at_kvp(kvp, pos) {
                return Some(kuid);
            }
        }
        None
    }

    pub fn find_container_at(&self, pos: Position) -> Option<&Value> {
        for kvp in &self.key_value_pairs {
            if let Some(val) = self.find_container_at_kvp(kvp, pos) {
                return Some(val);
            }
        }
        None
    }

    pub fn get_kvp_to_position(&self, position: Position) -> Vec<KeyValuePair> {
        let mut path = Vec::new();

        for top_kvp in &self.key_value_pairs {
            if position_in_range(position, top_kvp.range) {
                let mut current = top_kvp;
                path.push(current.clone());

                while let Some(Value::Container(inner_kvps, _, _)) = &current.value {
                    let mut found_inner = None;
                    for inner_kvp in inner_kvps {
                        if position_in_range(position, inner_kvp.range) {
                            found_inner = Some(inner_kvp);
                            break;
                        }
                    }

                    if let Some(found) = found_inner {
                        path.push(found.clone());
                        current = found;
                    } else {
                        break;
                    }
                }

                return path;
            }
        }
        path
    }

    pub fn get_path_to_container_at(&self, pos: Position) -> Vec<KeyValuePair> {
        let mut path = Vec::new();
        for kvp in &self.key_value_pairs {
            if let Some(p) = self.get_path_to_container_at_kvp(kvp, pos, &mut path) {
                return p;
            }
        }
        path
    }

    fn get_path_to_container_at_kvp(
        &self,
        kvp: &KeyValuePair,
        pos: Position,
        path: &mut Vec<KeyValuePair>,
    ) -> Option<Vec<KeyValuePair>> {
        if let Some(value) = &kvp.value
            && position_in_range(pos, value.range())
            && let Value::Container(kvps, _, _) = value
        {
            path.push(kvp.clone());
            for inner_kvp in kvps {
                if let Some(p) = self.get_path_to_container_at_kvp(inner_kvp, pos, path) {
                    return Some(p);
                }
            }
            return Some(path.clone());
        }
        None
    }

    fn find_container_at_kvp<'a>(&self, kvp: &'a KeyValuePair, pos: Position) -> Option<&'a Value> {
        if let Some(value) = &kvp.value
            && position_in_range(pos, value.range())
            && let Value::Container(kvps, _, _) = value
        {
            for inner_kvp in kvps {
                if let Some(inner_val) = self.find_container_at_kvp(inner_kvp, pos) {
                    return Some(inner_val);
                }
            }
            return Some(value);
        }
        None
    }

    fn find_kuid_at_kvp(&self, kvp: &KeyValuePair, pos: Position) -> Option<Kuid> {
        if let Some(value) = &kvp.value {
            return self.find_kuid_at_value(value, pos);
        }
        None
    }

    fn find_kuid_at_value(&self, value: &Value, pos: Position) -> Option<Kuid> {
        match value {
            Value::Kuid(kuid, range) if position_in_range(pos, *range) => {
                return Some(kuid.clone());
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
