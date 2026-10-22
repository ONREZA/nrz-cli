use serde::de::{DeserializeSeed, EnumAccess, IgnoredAny, MapAccess, SeqAccess, Visitor};

pub(super) fn flutter_dependency(source: &str) -> Option<bool> {
    // Detection has never expanded YAML merge keys or rejected non-finite custom values.
    let options = serde_saphyr::options! {
        merge_keys: serde_saphyr::MergeKeyPolicy::AsOrdinary,
        non_finite_float_policy: serde_saphyr::NonFiniteFloatPolicy::PassThrough,
        ignore_binary_tag_for_string: true,
        budget: serde_saphyr::budget! {
            max_depth: 128,
        },
    };
    match serde_saphyr::from_str_with_options::<FlutterPubspec>(source, options.clone()) {
        Ok(FlutterPubspec(flutter)) => Some(flutter),
        // A valid scalar, sequence, or differently typed field still permits Dart detection.
        Err(_) => serde_saphyr::from_str_with_options::<IgnoredAny>(source, options)
            .ok()
            .map(|_| false),
    }
}

struct PubspecKey(Option<String>);

// Consume complex keys directly instead of buffering a recursive Serde Content tree.
impl<'de> serde::Deserialize<'de> for PubspecKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Self(None))
    }
}

impl<'de> Visitor<'de> for PubspecKey {
    type Value = Self;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a YAML mapping key")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self, E> {
        Ok(Self(Some(value.to_owned())))
    }

    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self, E> {
        Ok(Self(None))
    }

    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self, E> {
        Ok(Self(None))
    }

    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self, E> {
        Ok(Self(None))
    }

    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self, E> {
        Ok(Self(None))
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self, E> {
        Ok(Self(None))
    }

    fn visit_seq<S: SeqAccess<'de>>(self, seq: S) -> Result<Self, S::Error> {
        IgnoredAny.visit_seq(seq).map(|_| Self(None))
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self, M::Error> {
        IgnoredAny.visit_map(map).map(|_| Self(None))
    }

    fn visit_enum<E: EnumAccess<'de>>(self, data: E) -> Result<Self, E::Error> {
        IgnoredAny.visit_enum(data).map(|_| Self(None))
    }
}

struct FlutterPubspec(bool);
struct FlutterDependency(usize);

impl<'de> serde::Deserialize<'de> for FlutterPubspec {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FlutterDependency(0).deserialize(deserializer).map(Self)
    }
}

impl<'de> DeserializeSeed<'de> for FlutterDependency {
    type Value = bool;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for FlutterDependency {
    type Value = bool;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a pubspec mapping or SDK name")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<bool, E> {
        Ok(self.0 == 3 && value == "flutter")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<bool, M::Error> {
        let mut flutter = false;
        while let Some(serde_saphyr::Tagged(key, tag)) =
            map.next_key::<serde_saphyr::Tagged<PubspecKey>>()?
        {
            // Local tags distinguish YAML keys; tags on values do not change lookup.
            if !tag.is_some_and(|tag| tag.starts_with('!'))
                && let PubspecKey(Some(name)) = key
                && ["dependencies", "flutter", "sdk"].get(self.0) == Some(&name.as_str())
            {
                flutter = map.next_value_seed(FlutterDependency(self.0 + 1))?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(flutter)
    }
}
