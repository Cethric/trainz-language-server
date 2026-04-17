use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TdxValue {
    Container(Vec<(String, TdxValue)>),
    Int32(i32),
    Int64(i64),
    Float64(f64),
    String(String),
    Bool(bool),
    Kuid(i64),
    AssetDescriptor {
        marker: String,
        name: String,
        unk: Vec<u8>,
        paths: Vec<String>,
        params: Vec<u8>,
    },
    AssetHeader {
        marker: String,
        name: String,
    },
    AssetPath(String),
    Binary(Vec<u8>),
    TypedProperty {
        value: u32,
        signature: u32,
    },
    Footer {
        strings: Vec<String>,
    },
    Unknown(u8, Vec<u8>),
    Array(Vec<TdxValue>),
    Float32(f32),
}
