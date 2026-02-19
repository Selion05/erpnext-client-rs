use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize)]
pub enum Comparator {
    #[serde(rename = ">=")]
    GreaterEqual,
    #[serde(rename = "<=")]
    SmallerEqual,
    #[serde(rename = "=")]
    Equal,
    #[serde(rename = "not in")]
    NotIn,
    #[serde(rename = "is")]
    Is,
    #[serde(rename = "in")]
    In,
}

#[derive(Debug, Clone)]
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

// Trait for automatic conversion to FilterValue
pub trait IntoFilterValue {
    fn into_filter_value(self) -> FilterValue;
}

impl IntoFilterValue for String {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Str(self)
    }
}

impl IntoFilterValue for &str {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Str(self.to_string())
    }
}

impl IntoFilterValue for f64 {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Float(self)
    }
}

impl IntoFilterValue for i32 {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Float(self as f64)
    }
}

impl IntoFilterValue for i64 {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Float(self as f64)
    }
}

impl IntoFilterValue for u32 {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Float(self as f64)
    }
}

impl IntoFilterValue for u64 {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Float(self as f64)
    }
}

impl IntoFilterValue for bool {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::Bool(self)
    }
}

impl IntoFilterValue for Vec<String> {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::VecStr(self)
    }
}

impl IntoFilterValue for Vec<&str> {
    fn into_filter_value(self) -> FilterValue {
        FilterValue::VecStr(self.into_iter().map(String::from).collect())
    }
}

// Filters struct for building filter lists
#[derive(Debug, Clone)]
pub struct Filters {
    filters: Vec<(String, Comparator, FilterValue)>,
}

impl Filters {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    fn add(mut self, field: impl Into<String>, comparator: Comparator, value: FilterValue) -> Self {
        self.filters.push((field.into(), comparator, value));
        self
    }

    pub fn add_equal<V: IntoFilterValue>(self, field: impl Into<String>, value: V) -> Self {
        self.add(field, Comparator::Equal, value.into_filter_value())
    }

    pub fn add_greater_equal<V: IntoFilterValue>(self, field: impl Into<String>, value: V) -> Self {
        self.add(field, Comparator::GreaterEqual, value.into_filter_value())
    }

    pub fn add_smaller_equal<V: IntoFilterValue>(self, field: impl Into<String>, value: V) -> Self {
        self.add(field, Comparator::SmallerEqual, value.into_filter_value())
    }

    pub fn add_not_in(self, field: impl Into<String>, values: Vec<String>) -> Self {
        self.add(field, Comparator::NotIn, FilterValue::VecStr(values))
    }

    pub fn add_is<V: IntoFilterValue>(self, field: impl Into<String>, value: V) -> Self {
        self.add(field, Comparator::Is, value.into_filter_value())
    }

    pub fn add_in(self, field: impl Into<String>, values: Vec<String>) -> Self {
        self.add(field, Comparator::In, FilterValue::VecStr(values))
    }
}

impl Default for Filters {
    fn default() -> Self {
        Self::new()
    }
}

// For backward compatibility - convert HashMap to Filters
impl From<HashMap<String, (Comparator, FilterValue)>> for Filters {
    fn from(map: HashMap<String, (Comparator, FilterValue)>) -> Self {
        let filters = map
            .into_iter()
            .map(|(field, (comp, val))| (field, comp, val))
            .collect();
        Self { filters }
    }
}

// Serialize Filters as an array for ERPNext
impl serde::Serialize for Filters {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.filters.len()))?;
        for (field, comparator, value) in &self.filters {
            seq.serialize_element(&(field, comparator, value))?;
        }
        seq.end()
    }
}
