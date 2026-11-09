//! Helpers for [`LuaValue`] / [`LuaTable`]: constructors, accessors, `From` impls and a serde
//! bridge ([`to_lua`] / [`from_lua`]) so exec-op arguments can be plain Rust structs.
//!
//! Mapping between serde data and Lua values:
//!
//! | serde            | Lua                                             |
//! |------------------|-------------------------------------------------|
//! | `None`, `()`     | nil (struct fields that are nil are omitted)    |
//! | bool             | boolean                                         |
//! | integers, floats | number (a double; integers must fit in 2^53)    |
//! | string, char     | string                                          |
//! | seq, tuple       | table with keys 1..n                            |
//! | map, struct      | table with string keys (map keys are stringified only if strings) |
//! | unit enum variant| string with the variant name                    |
//! | other variants   | table `{ <variant> = <value> }`                 |
//!
//! Reading back, a table whose keys are exactly 1..n is a sequence; an empty table is
//! accepted as an empty sequence, map or struct.

use std::fmt;

use serde::de::{self, DeserializeOwned, IntoDeserializer, Visitor};
use serde::{Deserializer, Serialize};

use crate::v1::lua_value::Kind;
use crate::v1::{LuaNil, LuaPair, LuaTable, LuaValue};

/// Largest integer a Lua 5.1 number (double) represents exactly.
pub const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

impl LuaValue {
    /// The nil value (an unset `kind`).
    #[must_use]
    pub const fn nil() -> Self {
        Self { kind: None }
    }

    #[must_use]
    pub const fn boolean(b: bool) -> Self {
        Self {
            kind: Some(Kind::Boolean(b)),
        }
    }

    #[must_use]
    pub const fn number(n: f64) -> Self {
        Self {
            kind: Some(Kind::Number(n)),
        }
    }

    #[must_use]
    pub fn string(s: impl Into<String>) -> Self {
        Self {
            kind: Some(Kind::String(s.into())),
        }
    }

    #[must_use]
    pub fn table(t: LuaTable) -> Self {
        Self {
            kind: Some(Kind::Table(t)),
        }
    }

    /// `true` for an unset kind and for an explicit `LuaNil`.
    #[must_use]
    pub fn is_nil(&self) -> bool {
        matches!(self.kind, None | Some(Kind::Nil(_)))
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self.kind {
            Some(Kind::Boolean(b)) => Some(b),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self.kind {
            Some(Kind::Number(n)) => Some(n),
            _ => None,
        }
    }

    /// The number as an integer, if it is integral and within +-2^53.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().and_then(f64_to_i64)
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Some(Kind::String(s)) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_table(&self) -> Option<&LuaTable> {
        match &self.kind {
            Some(Kind::Table(t)) => Some(t),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_table_mut(&mut self) -> Option<&mut LuaTable> {
        match &mut self.kind {
            Some(Kind::Table(t)) => Some(t),
            _ => None,
        }
    }

    /// Field lookup by string key; `None` if this is not a table or the key is absent.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&LuaValue> {
        self.as_table().and_then(|t| t.get(key))
    }

    /// Lua type name: "nil", "boolean", "number", "string" or "table".
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self.kind {
            None | Some(Kind::Nil(_)) => "nil",
            Some(Kind::Boolean(_)) => "boolean",
            Some(Kind::Number(_)) => "number",
            Some(Kind::String(_)) => "string",
            Some(Kind::Table(_)) => "table",
        }
    }

    fn key_eq(&self, other: &LuaValue) -> bool {
        match (&self.kind, &other.kind) {
            (Some(Kind::Boolean(a)), Some(Kind::Boolean(b))) => a == b,
            (Some(Kind::Number(a)), Some(Kind::Number(b))) => a == b,
            (Some(Kind::String(a)), Some(Kind::String(b))) => a == b,
            _ => false,
        }
    }
}

impl LuaTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An array table with keys 1..n.
    pub fn from_array<I, V>(values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<LuaValue>,
    {
        let mut t = Self::new();
        for v in values {
            t.push(v);
        }
        t
    }

    /// Builder form of [`LuaTable::set`].
    #[must_use]
    pub fn with(mut self, key: impl Into<LuaValue>, value: impl Into<LuaValue>) -> Self {
        self.set(key, value);
        self
    }

    /// Sets `key` to `value`, replacing an existing entry. A nil value removes the key
    /// (Lua tables cannot hold nil). A nil key is ignored.
    pub fn set(&mut self, key: impl Into<LuaValue>, value: impl Into<LuaValue>) {
        let key = key.into();
        let value = value.into();
        if key.is_nil() {
            return;
        }
        let pos = self
            .pairs
            .iter()
            .position(|p| p.key.as_ref().is_some_and(|k| k.key_eq(&key)));
        match (pos, value.is_nil()) {
            (Some(i), true) => {
                self.pairs.remove(i);
            }
            (Some(i), false) => self.pairs[i].value = Some(value),
            (None, true) => {}
            (None, false) => self.pairs.push(LuaPair {
                key: Some(key),
                value: Some(value),
            }),
        }
    }

    /// Appends at key `array_len() + 1`.
    pub fn push(&mut self, value: impl Into<LuaValue>) {
        let next = self.array_len() + 1;
        self.set(next as f64, value);
    }

    /// Value of a string key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&LuaValue> {
        self.pairs
            .iter()
            .find(|p| p.key.as_ref().and_then(LuaValue::as_str) == Some(key))
            .and_then(|p| p.value.as_ref())
    }

    /// Value of an arbitrary key.
    #[must_use]
    pub fn get_key(&self, key: &LuaValue) -> Option<&LuaValue> {
        self.pairs
            .iter()
            .find(|p| p.key.as_ref().is_some_and(|k| k.key_eq(key)))
            .and_then(|p| p.value.as_ref())
    }

    /// Value at a 1-based array index.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<&LuaValue> {
        self.get_key(&LuaValue::number(index as f64))
    }

    /// Number of pairs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Length of the array part: the largest n such that keys 1..=n are all present.
    #[must_use]
    pub fn array_len(&self) -> usize {
        let mut n = 0;
        while self.get_index(n + 1).is_some() {
            n += 1;
        }
        n
    }

    /// `true` if the table is non-empty and its keys are exactly 1..n.
    #[must_use]
    pub fn is_array(&self) -> bool {
        !self.pairs.is_empty() && self.array_len() == self.pairs.len()
    }

    /// Iterates over pairs in stored order (pairs with a missing key or value are skipped).
    pub fn iter(&self) -> impl Iterator<Item = (&LuaValue, &LuaValue)> {
        self.pairs
            .iter()
            .filter_map(|p| Some((p.key.as_ref()?, p.value.as_ref()?)))
    }

    /// Iterates over the array part (keys 1..array_len()).
    pub fn array(&self) -> impl Iterator<Item = &LuaValue> {
        (1..=self.array_len()).filter_map(|i| self.get_index(i))
    }
}

fn f64_to_i64(n: f64) -> Option<i64> {
    if n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER as f64 {
        Some(n as i64)
    } else {
        None
    }
}

impl From<bool> for LuaValue {
    fn from(v: bool) -> Self {
        Self::boolean(v)
    }
}

impl From<f64> for LuaValue {
    fn from(v: f64) -> Self {
        Self::number(v)
    }
}

impl From<f32> for LuaValue {
    fn from(v: f32) -> Self {
        Self::number(f64::from(v))
    }
}

macro_rules! from_int {
    ($($t:ty),*) => {$(
        impl From<$t> for LuaValue {
            fn from(v: $t) -> Self {
                Self::number(v as f64)
            }
        }
    )*};
}
from_int!(i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);

impl From<&str> for LuaValue {
    fn from(v: &str) -> Self {
        Self::string(v)
    }
}

impl From<String> for LuaValue {
    fn from(v: String) -> Self {
        Self::string(v)
    }
}

impl From<&String> for LuaValue {
    fn from(v: &String) -> Self {
        Self::string(v.as_str())
    }
}

impl From<LuaTable> for LuaValue {
    fn from(v: LuaTable) -> Self {
        Self::table(v)
    }
}

impl From<LuaNil> for LuaValue {
    fn from(_: LuaNil) -> Self {
        Self::nil()
    }
}

impl<T: Into<LuaValue>> From<Option<T>> for LuaValue {
    fn from(v: Option<T>) -> Self {
        v.map_or_else(Self::nil, Into::into)
    }
}

impl<T: Into<LuaValue>> From<Vec<T>> for LuaValue {
    fn from(v: Vec<T>) -> Self {
        Self::table(LuaTable::from_array(v))
    }
}

impl<K: Into<LuaValue>, V: Into<LuaValue>> FromIterator<(K, V)> for LuaTable {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut t = Self::new();
        for (k, v) in iter {
            t.set(k, v);
        }
        t
    }
}

impl fmt::Display for LuaValue {
    /// Lua-like rendering for logs and error messages, e.g. `{name="SAM North", [1]=2}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            None | Some(Kind::Nil(_)) => f.write_str("nil"),
            Some(Kind::Boolean(b)) => write!(f, "{b}"),
            Some(Kind::Number(n)) => match f64_to_i64(*n) {
                Some(i) => write!(f, "{i}"),
                None => write!(f, "{n}"),
            },
            Some(Kind::String(s)) => write!(f, "{s:?}"),
            Some(Kind::Table(t)) => {
                f.write_str("{")?;
                for (i, (k, v)) in t.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    match k.as_str() {
                        Some(s) if is_lua_identifier(s) => write!(f, "{s}={v}")?,
                        _ => write!(f, "[{k}]={v}")?,
                    }
                }
                f.write_str("}")
            }
        }
    }
}

fn is_lua_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Error of [`to_lua`] / [`from_lua`].
#[derive(Debug, thiserror::Error)]
pub enum LuaSerdeError {
    #[error("cannot convert to a Lua value: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("cannot read Lua value: {0}")]
    Deserialize(String),
}

impl de::Error for LuaSerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Deserialize(msg.to_string())
    }
}

/// Converts any `Serialize` value into a [`LuaValue`] (see the module docs for the mapping).
pub fn to_lua<T: Serialize + ?Sized>(value: &T) -> Result<LuaValue, LuaSerdeError> {
    Ok(json_to_lua(&serde_json::to_value(value)?))
}

/// Reads a `Deserialize` value out of a [`LuaValue`] (see the module docs for the mapping).
pub fn from_lua<T: DeserializeOwned>(value: &LuaValue) -> Result<T, LuaSerdeError> {
    T::deserialize(value)
}

/// Converts JSON into a Lua value (`null` becomes nil; nil entries are dropped from tables).
#[must_use]
pub fn json_to_lua(value: &serde_json::Value) -> LuaValue {
    use serde_json::Value as J;
    match value {
        J::Null => LuaValue::nil(),
        J::Bool(b) => LuaValue::boolean(*b),
        J::Number(n) => LuaValue::number(n.as_f64().unwrap_or(f64::NAN)),
        J::String(s) => LuaValue::string(s.as_str()),
        J::Array(items) => {
            let mut t = LuaTable::new();
            for (i, item) in items.iter().enumerate() {
                t.set((i + 1) as f64, json_to_lua(item));
            }
            LuaValue::table(t)
        }
        J::Object(map) => LuaValue::table(
            map.iter()
                .map(|(k, v)| (k.as_str(), json_to_lua(v)))
                .collect(),
        ),
    }
}

/// Converts a Lua value into JSON (arrays for 1..n tables, objects otherwise; non-string keys
/// are rendered with `Display`; NaN/inf become `null`).
#[must_use]
pub fn lua_to_json(value: &LuaValue) -> serde_json::Value {
    use serde_json::Value as J;
    match &value.kind {
        None | Some(Kind::Nil(_)) => J::Null,
        Some(Kind::Boolean(b)) => J::Bool(*b),
        Some(Kind::Number(n)) => match f64_to_i64(*n) {
            Some(i) => J::from(i),
            None => serde_json::Number::from_f64(*n).map_or(J::Null, J::Number),
        },
        Some(Kind::String(s)) => J::String(s.clone()),
        Some(Kind::Table(t)) if t.is_array() => J::Array(t.array().map(lua_to_json).collect()),
        Some(Kind::Table(t)) => J::Object(
            t.iter()
                .map(|(k, v)| {
                    let key = k.as_str().map_or_else(|| k.to_string(), str::to_owned);
                    (key, lua_to_json(v))
                })
                .collect(),
        ),
    }
}

// ---------------------------------------------------------------------------------------------
// serde::Deserializer for &LuaValue
// ---------------------------------------------------------------------------------------------

impl<'de> Deserializer<'de> for &'de LuaValue {
    type Error = LuaSerdeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.kind {
            None | Some(Kind::Nil(_)) => visitor.visit_unit(),
            Some(Kind::Boolean(b)) => visitor.visit_bool(*b),
            Some(Kind::Number(n)) => match f64_to_i64(*n) {
                Some(i) => visitor.visit_i64(i),
                None => visitor.visit_f64(*n),
            },
            Some(Kind::String(s)) => visitor.visit_borrowed_str(s),
            Some(Kind::Table(t)) if t.is_array() => visitor.visit_seq(SeqAccess::new(t)),
            Some(Kind::Table(t)) => visitor.visit_map(MapAccess::new(t)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.is_nil() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.kind {
            Some(Kind::Table(t)) if t.is_empty() || t.is_array() => {
                visitor.visit_seq(SeqAccess::new(t))
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.kind {
            Some(Kind::Table(t)) => visitor.visit_map(MapAccess::new(t)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match &self.kind {
            Some(Kind::String(s)) => {
                let de: de::value::StrDeserializer<'_, LuaSerdeError> =
                    s.as_str().into_deserializer();
                visitor.visit_enum(de)
            }
            Some(Kind::Table(t)) if t.len() == 1 => {
                let (k, v) = t
                    .iter()
                    .next()
                    .ok_or_else(|| LuaSerdeError::Deserialize("empty enum table".into()))?;
                let variant = k.as_str().ok_or_else(|| {
                    LuaSerdeError::Deserialize("enum variant key must be a string".into())
                })?;
                visitor.visit_enum(EnumAccess { variant, value: v })
            }
            _ => Err(de::Error::custom(format!(
                "expected an enum (string or one-key table), got {}",
                self.type_name()
            ))),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit_struct identifier
    }
}

struct SeqAccess<'de> {
    table: &'de LuaTable,
    next: usize,
    len: usize,
}

impl<'de> SeqAccess<'de> {
    fn new(table: &'de LuaTable) -> Self {
        Self {
            table,
            next: 1,
            len: table.array_len(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = LuaSerdeError;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.next > self.len {
            return Ok(None);
        }
        let value = self
            .table
            .get_index(self.next)
            .ok_or_else(|| LuaSerdeError::Deserialize("array hole".into()))?;
        self.next += 1;
        seed.deserialize(value).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len + 1 - self.next)
    }
}

struct MapAccess<'de> {
    iter: Box<dyn Iterator<Item = (&'de LuaValue, &'de LuaValue)> + 'de>,
    value: Option<&'de LuaValue>,
}

impl<'de> MapAccess<'de> {
    fn new(table: &'de LuaTable) -> Self {
        Self {
            iter: Box::new(table.iter()),
            value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess<'de> {
    type Error = LuaSerdeError;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((k, v)) => {
                self.value = Some(v);
                seed.deserialize(k).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let value = self
            .value
            .take()
            .ok_or_else(|| LuaSerdeError::Deserialize("value without key".into()))?;
        seed.deserialize(value)
    }
}

struct EnumAccess<'de> {
    variant: &'de str,
    value: &'de LuaValue,
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = LuaSerdeError;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V: de::DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let de: de::value::StrDeserializer<'_, LuaSerdeError> = self.variant.into_deserializer();
        let variant = seed.deserialize(de)?;
        Ok((variant, VariantAccess { value: self.value }))
    }
}

struct VariantAccess<'de> {
    value: &'de LuaValue,
}

impl<'de> de::VariantAccess<'de> for VariantAccess<'de> {
    type Error = LuaSerdeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.value.deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.value.deserialize_map(visitor)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Unit {
        name: String,
        #[serde(rename = "type")]
        type_name: String,
        x: f64,
        alive: bool,
        player: Option<String>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Group {
        name: String,
        id: u32,
        units: Vec<Unit>,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        extra: BTreeMap<String, i32>,
        color: Color,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Color {
        Red,
        Blue,
    }

    #[test]
    fn table_builders() {
        let mut t = LuaTable::new().with("name", "SAM North").with("id", 3);
        t.push(10);
        t.push(20);
        assert_eq!(t.get("name").and_then(LuaValue::as_str), Some("SAM North"));
        assert_eq!(t.get("id").and_then(LuaValue::as_i64), Some(3));
        assert_eq!(t.array_len(), 2);
        assert_eq!(t.get_index(2).and_then(LuaValue::as_i64), Some(20));
        t.set("id", LuaValue::nil());
        assert!(t.get("id").is_none());
        assert_eq!(t.len(), 3);
        let v = LuaValue::from(t);
        assert_eq!(v.to_string(), r#"{name="SAM North", [1]=10, [2]=20}"#);
    }

    #[test]
    fn serde_round_trip() {
        let g = Group {
            name: "SAM North".into(),
            id: 7,
            units: vec![
                Unit {
                    name: "SAM North-1".into(),
                    type_name: "S-300PS 54K6 cp".into(),
                    x: -32798.5,
                    alive: true,
                    player: None,
                },
                Unit {
                    name: "SAM North-2".into(),
                    type_name: "S-300PS 40B6M tr".into(),
                    x: 1.0,
                    alive: false,
                    player: Some("Bob".into()),
                },
            ],
            tags: vec![],
            extra: BTreeMap::new(),
            color: Color::Blue,
        };
        let lua = to_lua(&g).unwrap();
        // nil fields are omitted, empty vectors/maps become empty tables
        assert!(
            lua.get("units")
                .unwrap()
                .as_table()
                .unwrap()
                .get_index(1)
                .unwrap()
                .get("player")
                .is_none()
        );
        assert_eq!(lua.get("color").and_then(LuaValue::as_str), Some("blue"));
        let back: Group = from_lua(&lua).unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn integers_from_doubles() {
        let v = LuaValue::number(16_777_472.0);
        let n: u32 = from_lua(&v).unwrap();
        assert_eq!(n, 16_777_472);
        assert!(from_lua::<u32>(&LuaValue::number(1.5)).is_err());
        let f: f64 = from_lua(&LuaValue::number(2.0)).unwrap();
        assert_eq!(f, 2.0);
    }

    #[test]
    fn json_round_trip() {
        let j = serde_json::json!({"a": [1, 2, 3], "b": {"c": "d"}, "e": 1.5, "f": null});
        let lua = json_to_lua(&j);
        assert!(lua.get("f").is_none());
        let back = lua_to_json(&lua);
        assert_eq!(
            back,
            serde_json::json!({"a": [1, 2, 3], "b": {"c": "d"}, "e": 1.5})
        );
    }
}
