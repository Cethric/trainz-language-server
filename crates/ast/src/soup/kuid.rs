use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kuid {
    pub user_id: i64,
    pub content_id: i64,
    pub version: Option<u32>,
    pub range: crate::Range,
}
