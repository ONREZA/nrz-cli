use std::{borrow::Cow, fmt};

use nrz_contract::artifact_json_v1::AllocationProfile;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct StructuralCounts {
    pub allocation_profile: AllocationProfile,
    pub json_values: u64,
    pub objects: u64,
    pub object_members: u64,
    pub array_elements: u64,
    pub strings: u64,
    pub decoded_string_bytes: u64,
    pub owned_parser_string_bytes: u64,
    pub max_string_bytes: u64,
    pub max_error_string_bytes: u64,
    pub metadata_objects: u64,
    pub metadata_object_members: u64,
    pub metadata_array_elements: u64,
    pub metadata_strings: u64,
    pub metadata_decoded_string_bytes: u64,
    pub files: u64,
    pub path_components: u64,
    /// Exact for accepted sorted paths; an upper bound for other input.
    pub unique_path_nodes: u64,
    pub max_path_bytes: u64,
    pub max_target_bytes: u64,
    pub symlinks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PreflightResourceLimit {
    FileCount,
    CounterOverflow,
    AllocationFailure,
}

#[derive(Debug)]
pub(super) enum PreflightError {
    ResourceLimit(PreflightResourceLimit),
    Invalid(serde_json::Error),
}

/// The caller reserves parser/previous-path scratch before entering this pass.
pub(super) fn inspect(
    bytes: &[u8],
    max_file_count: u64,
    allocation_profile: AllocationProfile,
) -> Result<StructuralCounts, PreflightError> {
    let mut state = State {
        counts: StructuralCounts {
            allocation_profile,
            ..StructuralCounts::default()
        },
        previous_path: Cow::Borrowed(""),
        max_file_count,
        resource_limit: None,
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let result = Seed {
        state: &mut state,
        context: Context::Root,
        depth: 0,
        array_element: false,
        element_limit: None,
        metadata: false,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end());
    match result {
        Ok(()) => Ok(state.counts),
        Err(error) => Err(match state.resource_limit {
            Some(limit) => PreflightError::ResourceLimit(limit),
            None => PreflightError::Invalid(error),
        }),
    }
}

struct State<'de> {
    counts: StructuralCounts,
    previous_path: Cow<'de, str>,
    max_file_count: u64,
    resource_limit: Option<PreflightResourceLimit>,
}

impl<'de> State<'de> {
    fn resource_error<E: de::Error>(&mut self, limit: PreflightResourceLimit) -> E {
        self.resource_limit = Some(limit);
        E::custom("SOURCE_BUNDLE_V2 preflight resource limit")
    }

    fn add<E: de::Error>(
        &mut self,
        field: fn(&mut StructuralCounts) -> &mut u64,
        amount: u64,
    ) -> Result<(), E> {
        let target = field(&mut self.counts);
        *target = match target.checked_add(amount) {
            Some(value) => value,
            None => return Err(self.resource_error(PreflightResourceLimit::CounterOverflow)),
        };
        Ok(())
    }

    fn string<E: de::Error>(
        &mut self,
        value: &str,
        context: Context,
        metadata: bool,
        borrowed: Option<&'de str>,
    ) -> Result<(), E> {
        self.add(|counts| &mut counts.strings, 1)?;
        self.add(
            |counts| &mut counts.decoded_string_bytes,
            value.len() as u64,
        )?;
        self.counts.max_string_bytes = self.counts.max_string_bytes.max(value.len() as u64);
        if !matches!(
            context,
            Context::Path | Context::Target | Context::StringAllowed
        ) {
            self.counts.max_error_string_bytes =
                self.counts.max_error_string_bytes.max(value.len() as u64);
        }
        if metadata {
            self.add(|counts| &mut counts.metadata_strings, 1)?;
            self.add(
                |counts| &mut counts.metadata_decoded_string_bytes,
                value.len() as u64,
            )?;
        }
        match context {
            Context::Path => {
                let components = value.split('/').count() as u64;
                let shared = if self.counts.files == 1 {
                    0
                } else {
                    self.previous_path
                        .split('/')
                        .zip(value.split('/'))
                        .take_while(|(previous, current)| previous == current)
                        .count() as u64
                };
                self.add(|counts| &mut counts.path_components, components)?;
                self.add(|counts| &mut counts.unique_path_nodes, components - shared)?;
                self.counts.max_path_bytes = self.counts.max_path_bytes.max(value.len() as u64);
                if let Some(value) = borrowed {
                    self.previous_path = Cow::Borrowed(value);
                } else {
                    let mut owned =
                        match std::mem::replace(&mut self.previous_path, Cow::Borrowed("")) {
                            Cow::Owned(value) => value,
                            Cow::Borrowed(_) => String::new(),
                        };
                    owned.clear();
                    owned.try_reserve_exact(value.len()).map_err(|_| {
                        self.resource_error::<E>(PreflightResourceLimit::AllocationFailure)
                    })?;
                    owned.push_str(value);
                    self.previous_path = Cow::Owned(owned);
                }
            }
            Context::Target => {
                self.counts.max_target_bytes = self.counts.max_target_bytes.max(value.len() as u64);
            }
            Context::EntryType => match value {
                "file" => {}
                "symlink" => self.add(|counts| &mut counts.symlinks, 1)?,
                _ => return Err(E::custom("unsupported file entryType")),
            },
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Context {
    Root,
    Files,
    Layers,
    Routes,
    File,
    Path,
    Target,
    EntryType,
    StringAllowed,
    Other,
}

struct Seed<'a, 'de> {
    state: &'a mut State<'de>,
    context: Context,
    depth: usize,
    array_element: bool,
    element_limit: Option<(u64, u64)>,
    metadata: bool,
}

impl<'de> DeserializeSeed<'de> for Seed<'_, 'de> {
    type Value = ();

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        if matches!(self.context, Context::File) {
            if self.state.counts.files >= self.state.max_file_count {
                return Err(self.state.resource_error(PreflightResourceLimit::FileCount));
            }
            self.state.add(|counts| &mut counts.files, 1)?;
        }
        if self
            .element_limit
            .is_some_and(|(count, limit)| count >= limit)
        {
            return Err(de::Error::custom(
                "layer or route count exceeds its supported bound",
            ));
        }
        self.state.add(|counts| &mut counts.json_values, 1)?;
        if self.array_element {
            self.state.add(|counts| &mut counts.array_elements, 1)?;
            if self.metadata {
                self.state
                    .add(|counts| &mut counts.metadata_array_elements, 1)?;
            }
        }
        // Typed entrypoints format unexpected strings into unbounded diagnostics.
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_, 'de> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SOURCE_BUNDLE_V2 JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        self.check_context(matches!(self.context, Context::Other))
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        self.check_context(matches!(self.context, Context::Other))
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        self.check_context(matches!(self.context, Context::Other))
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        self.check_context(matches!(self.context, Context::Other))
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        self.check_context(matches!(self.context, Context::Other))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<(), E> {
        self.visit_input_str(value, None)
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<(), E> {
        self.visit_input_str(value, Some(value))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<(), E> {
        // SliceRead strings borrow/copy; AP numeric payloads arrive owned.
        self.state.add(
            |counts| &mut counts.owned_parser_string_bytes,
            value.len() as u64,
        )?;
        self.visit_input_str(&value, None)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        self.check_context(matches!(
            self.context,
            Context::Other | Context::Files | Context::Layers | Context::Routes
        ))?;
        self.check_depth()?;
        let context = if matches!(self.context, Context::Files) {
            Context::File
        } else {
            Context::Other
        };
        let limit = match self.context {
            Context::Layers => Some(10),
            Context::Routes => Some(200),
            _ => None,
        };
        let mut elements = 0_u64;
        while let Some(()) = sequence.next_element_seed(Seed {
            state: &mut *self.state,
            context,
            depth: self.depth + 1,
            array_element: true,
            element_limit: limit.map(|limit| (elements, limit)),
            metadata: self.metadata,
        })? {
            elements = elements.checked_add(1).ok_or_else(|| {
                self.state
                    .resource_error::<A::Error>(PreflightResourceLimit::CounterOverflow)
            })?;
        }
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        self.check_context(matches!(
            self.context,
            Context::Other | Context::Root | Context::File
        ))?;
        self.check_depth()?;
        self.state.add(|counts| &mut counts.objects, 1)?;
        if self.metadata {
            self.state.add(|counts| &mut counts.metadata_objects, 1)?;
        }
        let mut seen = 0_u8;
        while let Some(key) = map.next_key_seed(KeySeed(&mut *self.state, self.metadata))? {
            self.state.add(|counts| &mut counts.object_members, 1)?;
            if self.metadata {
                self.state
                    .add(|counts| &mut counts.metadata_object_members, 1)?;
            }
            let (context, bit) = match (self.context, key) {
                (Context::Root, Key::Files) => (Context::Files, 1),
                (Context::Root, Key::Layers) => (Context::Layers, 2),
                (Context::Root, Key::Routes) => (Context::Routes, 4),
                (Context::File, Key::Path) => (Context::Path, 1),
                (Context::File, Key::Target) => (Context::Target, 2),
                (Context::File, Key::EntryType) => (Context::EntryType, 4),
                (Context::File, Key::ContentType) => (Context::StringAllowed, 8),
                _ => (Context::Other, 0),
            };
            if bit & seen != 0 {
                return Err(de::Error::custom("duplicate preflight structural field"));
            }
            seen |= bit;
            map.next_value_seed(Seed {
                state: &mut *self.state,
                context,
                depth: self.depth + 1,
                array_element: false,
                element_limit: None,
                metadata: self.metadata || matches!(context, Context::Layers),
            })?;
        }
        if matches!(self.context, Context::File) && seen & 5 != 5 {
            return Err(de::Error::custom("file requires path and entryType"));
        }
        Ok(())
    }
}

impl<'de> Seed<'_, 'de> {
    fn visit_input_str<E: de::Error>(
        self,
        value: &str,
        borrowed: Option<&'de str>,
    ) -> Result<(), E> {
        self.check_context(matches!(
            self.context,
            Context::Other
                | Context::Path
                | Context::Target
                | Context::EntryType
                | Context::StringAllowed
        ))?;
        self.state
            .string(value, self.context, self.metadata, borrowed)
    }

    fn check_context<E: de::Error>(&self, accepted: bool) -> Result<(), E> {
        if !accepted {
            return Err(E::custom("invalid SOURCE_BUNDLE_V2 structural field shape"));
        }
        Ok(())
    }

    fn check_depth<E: de::Error>(&self) -> Result<(), E> {
        if self.depth >= nrz_contract::artifact_json_v1::MAX_SUPPORTED_DEPTH {
            return Err(E::custom("artifact JSON depth budget exceeded"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Key {
    Files,
    Layers,
    Routes,
    Path,
    Target,
    EntryType,
    ContentType,
    Other,
}

struct KeySeed<'a, 'de>(&'a mut State<'de>, bool);

impl<'de> DeserializeSeed<'de> for KeySeed<'_, 'de> {
    type Value = Key;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Key, D::Error> {
        deserializer.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for KeySeed<'_, 'de> {
    type Value = Key;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON object key")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Key, E> {
        self.0.string(value, Context::Other, self.1, None)?;
        Ok(match value {
            "files" => Key::Files,
            "layers" => Key::Layers,
            "routes" => Key::Routes,
            "path" => Key::Path,
            "linkTarget" => Key::Target,
            "entryType" => Key::EntryType,
            "contentType" => Key::ContentType,
            _ => Key::Other,
        })
    }
}

#[cfg(test)]
#[path = "source_bundle_v2_preflight_tests.rs"]
mod tests;
