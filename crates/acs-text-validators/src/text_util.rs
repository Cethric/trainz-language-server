use crate::{parse_as_numeric, parse_as_string};
use rayon::prelude::*;
use trainz_ast::Range;
use trainz_ast::acs_text::{AcsText, KeyValuePair};

/// Extracts the "kind" value from the given `AcsText`.
///
/// # Arguments
///
/// * `acs_text`: The `AcsText` object to extract the kind from.
///
/// # Returns
///
/// An `Option<(String, Range, bool)>` containing the kind name, its range in the text, and whether there are multiple "kind" declarations.
///
/// # Examples
///
/// ```
/// # use trainz_ast::Range;
/// # use trainz_ast::acs_text::AcsText;
/// # use trainz_acs_text_validators::text_util::get_kind_from_text;
/// # let acs_text = AcsText { key_value_pairs: vec![], src: String::from(""), range: Range::default() };
/// # let kind = get_kind_from_text(&acs_text);
/// ```
#[tracing::instrument(skip(acs_text))]
pub fn get_kind_from_text(acs_text: &AcsText) -> Option<(String, Range, bool)> {
    get_kind_from_kvp(&acs_text.key_value_pairs)
}

/// Extracts the "kind" value from a slice of `KeyValuePair`s.
///
/// # Arguments
///
/// * `key_value_pairs`: The slice of `KeyValuePair`s to search.
///
/// # Returns
///
/// An `Option<(String, Range, bool)>` containing the kind name, its range in the text, and whether there are multiple "kind" declarations.
///
/// # Examples
///
/// ```
/// use trainz_ast::acs_text::KeyValuePair;
/// use trainz_acs_text_validators::text_util::get_kind_from_kvp;
///
/// let kvps: Vec<KeyValuePair> = vec![]; // Simplified example
/// let kind = get_kind_from_kvp(&kvps);
/// ```
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

/// Extracts the "trainz-build" value from the given `AcsText`.
///
/// # Arguments
///
/// * `acs_text`: The `AcsText` object to extract the build version from.
///
/// # Returns
///
/// The `f64` trainz-build version, or `0.0` if not found or invalid.
///
/// # Examples
///
/// ```
/// # use trainz_ast::Range;
/// # use trainz_ast::acs_text::AcsText;
/// # use trainz_acs_text_validators::text_util::get_trainz_build_from_text;
/// # let acs_text = AcsText { key_value_pairs: vec![], src: String::from(""), range: Range::default() };
/// # let build = get_trainz_build_from_text(&acs_text);
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
