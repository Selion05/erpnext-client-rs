#[derive(serde::Serialize)]
pub enum Comparator {
    #[serde(rename = ">=")]
    GreaterEqual,
    #[serde(rename = "=")]
    Equal,
    #[serde(rename = "not in")]
    NotIn,
    #[serde(rename = "is")]
    Is,
    #[serde(rename = "in")]
    In,
}

pub enum FilterValue {
    Str(String),
    VecStr(Vec<String>),
    Float(f64),
    NotSet,
    Bool(bool),
}

impl serde::Serialize for FilterValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            FilterValue::Str(val) => serializer.serialize_str(val),
            FilterValue::VecStr(val) => val.serialize(serializer),
            FilterValue::Float(val) => serializer.serialize_f64(*val),
            FilterValue::NotSet => serializer.serialize_str("not set"),
            FilterValue::Bool(val) => serializer.serialize_bool(*val),
        }
    }
}
