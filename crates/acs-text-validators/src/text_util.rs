use crate::{parse_as_numeric, parse_as_string};
use rayon::prelude::*;
use trainz_ast::Range;
use trainz_ast::acs_text::{AcsText, KeyValuePair};

///
///
/// # Arguments
///
/// * `acs_text`:
///
/// returns: Option<(String, bool)>
///
/// # Examples
///
/// ```
///
/// ```
#[tracing::instrument(skip(acs_text))]
pub fn get_kind_from_text(acs_text: &AcsText) -> Option<(String, Range, bool)> {
    get_kind_from_kvp(&acs_text.key_value_pairs)
}

#[tracing::instrument(skip(key_value_pairs))]
pub fn get_kind_from_kvp(key_value_pairs: &[KeyValuePair]) -> Option<(String, Range, bool)> {
    let kinds: Vec<&KeyValuePair> = key_value_pairs
        .par_iter()
        .filter_map(|kv| {
            if kv.key.eq_ignore_ascii_case("kind") {
                Some(kv)
            } else {
                None
            }
        })
        .collect();

    let kind = kinds.first();

    if let Some(kind) = kind
        && let Some(kind_value) = parse_as_string(kind)
    {
        Some((kind_value, kind.key_range, kinds.len() > 1))
    } else {
        None
    }
}

///
///
/// # Arguments
///
/// * `acs_text`:
///
/// returns: f64
///
/// # Examples
///
/// ```
///
/// ```
#[tracing::instrument(skip(acs_text))]
pub fn get_trainz_build_from_text(acs_text: &AcsText) -> f64 {
    let trainz_build: Vec<&KeyValuePair> = acs_text
        .key_value_pairs
        .par_iter()
        .filter_map(|kv| {
            if kv.key.eq_ignore_ascii_case("trainz-build") {
                Some(kv)
            } else {
                None
            }
        })
        .collect();

    if let Some(kv) = trainz_build.first()
        && let Some(version) = parse_as_numeric::<f64>(kv)
    {
        version
    } else {
        0.0f64
    }
}
