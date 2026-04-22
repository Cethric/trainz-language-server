use trainz_ast::acs_text::{KeyValuePair, NumericValue, Value};
pub trait FromNumericValue: Sized {
    fn from_int(value: i64) -> Self;
    fn from_hex(value: u64) -> Self;
    fn from_float(value: f64) -> Self;
}

impl FromNumericValue for bool {
    #[tracing::instrument(skip(value))]
    fn from_int(value: i64) -> Self {
        value > 0
    }

    #[tracing::instrument(skip(value))]
    fn from_hex(value: u64) -> Self {
        value > 0
    }

    #[tracing::instrument(skip(value))]
    fn from_float(value: f64) -> Self {
        value > 0.0
    }
}

impl FromNumericValue for f64 {
    #[tracing::instrument(skip(value))]
    fn from_int(value: i64) -> Self {
        value as f64
    }

    #[tracing::instrument(skip(value))]
    fn from_hex(value: u64) -> Self {
        value as f64
    }

    #[tracing::instrument(skip(value))]
    fn from_float(value: f64) -> Self {
        value
    }
}

impl FromNumericValue for u64 {
    #[tracing::instrument(skip(value))]
    fn from_int(value: i64) -> Self {
        value as u64
    }

    #[tracing::instrument(skip(value))]
    fn from_hex(value: u64) -> Self {
        value
    }

    #[tracing::instrument(skip(value))]
    fn from_float(value: f64) -> Self {
        value as u64
    }
}

impl FromNumericValue for i64 {
    #[tracing::instrument(skip(value))]
    fn from_int(value: i64) -> Self {
        value
    }

    #[tracing::instrument(skip(value))]
    fn from_hex(value: u64) -> Self {
        value as i64
    }

    #[tracing::instrument(skip(value))]
    fn from_float(value: f64) -> Self {
        value as i64
    }
}

impl FromNumericValue for u8 {
    #[tracing::instrument(skip(value))]
    fn from_int(value: i64) -> Self {
        value as u8
    }

    #[tracing::instrument(skip(value))]
    fn from_hex(value: u64) -> Self {
        value as u8
    }

    #[tracing::instrument(skip(value))]
    fn from_float(value: f64) -> Self {
        value as u8
    }
}

#[tracing::instrument(skip(key_value_pair))]
pub fn parse_as_bool(key_value_pair: &KeyValuePair) -> bool {
    parse_as_numeric::<bool>(key_value_pair).unwrap_or(false)
}

#[tracing::instrument(skip(numeric_value))]
pub fn parse_numeric_value<T>(numeric_value: &NumericValue) -> Option<T>
where
    T: FromNumericValue,
{
    match numeric_value {
        NumericValue::Int(value) => Some(T::from_int(*value)),
        NumericValue::Hex(value) => Some(T::from_hex(*value)),
        NumericValue::Float(value) => Some(T::from_float(*value)),
    }
}

#[tracing::instrument(skip(key_value_pair))]
pub fn parse_as_numeric<T>(key_value_pair: &KeyValuePair) -> Option<T>
where
    T: FromNumericValue,
{
    if let Some(Value::Numeric(numeric_value, _)) = &key_value_pair.value {
        parse_numeric_value::<T>(numeric_value)
    } else {
        None
    }
}

#[tracing::instrument(skip(key_value_pair))]
pub fn parse_as_numeric_list<T>(key_value_pair: &KeyValuePair) -> Option<Vec<Option<T>>>
where
    T: FromNumericValue,
{
    if let Some(Value::Array(values, _)) = &key_value_pair.value {
        Some(
            values
                .iter()
                .map(|value| parse_numeric_value::<T>(value))
                .collect::<Vec<Option<T>>>(),
        )
    } else {
        None
    }
}

#[tracing::instrument(skip(key_value_pair))]
pub(crate) fn parse_as_array_option<T>(key_value_pair: &KeyValuePair) -> Option<Vec<T>>
where
    T: FromNumericValue,
{
    if let Some(Value::Array(array_value, _)) = &key_value_pair.value {
        Some(
            array_value
                .iter()
                .map(|numeric_value| match numeric_value {
                    NumericValue::Int(value) => T::from_int(*value),
                    NumericValue::Hex(value) => T::from_hex(*value),
                    NumericValue::Float(value) => T::from_float(*value),
                })
                .collect::<Vec<T>>(),
        )
    } else {
        None
    }
}

#[tracing::instrument(skip(key_value_pair))]
pub(crate) fn parse_as_value_is_non_zero<T>(key_value_pair: &KeyValuePair) -> Option<T>
where
    T: FromNumericValue,
{
    if let Some(Value::Numeric(numeric_value, _)) = &key_value_pair.value {
        match numeric_value {
            NumericValue::Int(value) if *value > 0 => Some(T::from_int(*value)),
            NumericValue::Hex(value) if *value > 0 => Some(T::from_hex(*value)),
            NumericValue::Float(value) if *value > 0.0 => Some(T::from_float(*value)),
            _ => None,
        }
    } else {
        None
    }
}

#[tracing::instrument(skip(key_value_pair))]
pub fn parse_as_string(key_value_pair: &KeyValuePair) -> Option<String> {
    if let Some(Value::String(value, _)) = &key_value_pair.value {
        Some(value.clone())
    } else if let Some(Value::Variable(value, _)) = &key_value_pair.value {
        Some(value.clone())
    } else {
        None
    }
}
