use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kuid {
    pub user_id: i32,
    pub content_id: i32,
    pub version: Option<u8>,
    pub range: crate::Range,
}

impl Kuid {
    pub fn can_increment(&self) -> bool {
        if let Some(version) = self.version {
            version < 255
        } else {
            true
        }
    }

    pub fn can_decrement(&self) -> bool {
        if let Some(version) = self.version {
            version > 1
        } else {
            false
        }
    }

    pub fn increment(&mut self) {
        if let Some(version) = self.version {
            self.version = Some(version + 1);
        } else {
            self.version = Some(2);
        }
    }

    pub fn decrement(&mut self) {
        if let Some(version) = self.version
            && version > 1
        {
            self.version = Some(version - 1);
        }
    }
}

impl Display for Kuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(version) = self.version {
            write!(
                f,
                "<kuid2:{}:{}:{}>",
                self.user_id, self.content_id, version
            )
        } else {
            write!(f, "<kuid:{}:{}>", self.user_id, self.content_id)
        }
    }
}
