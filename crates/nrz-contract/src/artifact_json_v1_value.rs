//! Plain JSON objects at the artifact boundary, without serde's Value sentinels.

use std::fmt;

use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

pub(super) fn from_str(text: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str::<PlainValue>(text).map(|value| value.0)
}

struct PlainValue(Value);

impl<'de> Deserialize<'de> for PlainValue {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor).map(Self)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<PlainValue>()? {
            values.push(value.0);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        match map.next_key_seed(KeySeed)? {
            Some(Key::Number) => return map.next_value_seed(NumberSeed).map(Value::Number),
            Some(Key::Object(key)) => {
                values.insert(key, map.next_value::<PlainValue>()?.0);
            }
            None => return Ok(Value::Object(values)),
        }
        while let Some((key, value)) = map.next_entry::<String, PlainValue>()? {
            values.insert(key, value.0);
        }
        Ok(Value::Object(values))
    }
}

enum Key {
    Object(String),
    Number,
}

struct KeySeed;

impl<'de> DeserializeSeed<'de> for KeySeed {
    type Value = Key;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Key, D::Error> {
        // Real JSON keys use the newtype callback; synthetic AP numbers bypass it.
        deserializer.deserialize_newtype_struct("ONREZA_ARTIFACT_JSON_KEY", self)
    }
}

impl<'de> Visitor<'de> for KeySeed {
    type Value = Key;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON object key or synthetic number key")
    }

    fn visit_newtype_struct<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Key, D::Error> {
        String::deserialize(deserializer).map(Key::Object)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Key, E> {
        if value == "$serde_json::private::Number" {
            Ok(Key::Number)
        } else {
            Err(E::custom("unexpected synthetic JSON number key"))
        }
    }
}

struct NumberSeed;

impl<'de> DeserializeSeed<'de> for NumberSeed {
    type Value = Number;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Number, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for NumberSeed {
    type Value = Number;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("string containing a number")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Number, E> {
        value.parse().map_err(E::custom)
    }
}
