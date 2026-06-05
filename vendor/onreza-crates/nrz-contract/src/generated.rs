// @generated for nrz-cli binary builds.
// Do not edit.

#![allow(dead_code, unused, clippy::all)]
pub mod edge_rules {
    /// Error types.
    pub mod error {
        /// Error from a `TryFrom` or `FromStr` implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }
        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }
        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }
    ///`EdgeRuleActionAuthoring`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "const": "allow"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "const": "log"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "mode": {
    ///          "type": "string",
    ///          "enum": [
    ///            "shadow",
    ///            "enforce"
    ///          ]
    ///        },
    ///        "statusCode": {
    ///          "type": "integer",
    ///          "maximum": 599.0,
    ///          "minimum": 400.0
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "deny"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "target",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "force": {
    ///          "type": "boolean"
    ///        },
    ///        "statusCode": {
    ///          "anyOf": [
    ///            {
    ///              "type": "number",
    ///              "const": 301
    ///            },
    ///            {
    ///              "type": "number",
    ///              "const": 302
    ///            },
    ///            {
    ///              "type": "number",
    ///              "const": 307
    ///            },
    ///            {
    ///              "type": "number",
    ///              "const": 308
    ///            }
    ///          ]
    ///        },
    ///        "target": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "redirect"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "target",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "external": {
    ///          "type": "boolean"
    ///        },
    ///        "force": {
    ///          "type": "boolean"
    ///        },
    ///        "target": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "rewrite"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "headers",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "headers": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "set_headers"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "headers",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "headers": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "remove_headers"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "ttlSeconds",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "swrSeconds": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 0.0
    ///        },
    ///        "ttlSeconds": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "exclusiveMinimum": 0.0
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "cache"
    ///        },
    ///        "vary": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "geo",
    ///              "device",
    ///              "header",
    ///              "cookie",
    ///              "query"
    ///            ]
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "const": "bypass_cache"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(tag = "type", deny_unknown_fields)]
    pub enum EdgeRuleActionAuthoring {
        #[serde(rename = "allow")]
        Allow,
        #[serde(rename = "log")]
        Log,
        #[serde(rename = "deny")]
        Deny {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            mode: ::std::option::Option<EdgeRuleActionAuthoringMode>,
            #[serde(
                rename = "statusCode",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            status_code: ::std::option::Option<i64>,
        },
        #[serde(rename = "redirect")]
        Redirect {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            force: ::std::option::Option<bool>,
            #[serde(
                rename = "statusCode",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            status_code: ::std::option::Option<EdgeRuleActionAuthoringStatusCode>,
            target: EdgeRuleActionAuthoringTarget,
        },
        #[serde(rename = "rewrite")]
        Rewrite {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            external: ::std::option::Option<bool>,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            force: ::std::option::Option<bool>,
            target: EdgeRuleActionAuthoringTarget,
        },
        #[serde(rename = "set_headers")]
        SetHeaders {
            headers: ::std::collections::HashMap<
                EdgeRuleActionAuthoringHeadersKey,
                ::std::string::String,
            >,
        },
        #[serde(rename = "remove_headers")]
        RemoveHeaders {
            headers: ::std::vec::Vec<EdgeRuleActionAuthoringHeadersItem>,
        },
        #[serde(rename = "cache")]
        Cache {
            #[serde(
                rename = "swrSeconds",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            swr_seconds: ::std::option::Option<i64>,
            #[serde(rename = "ttlSeconds")]
            ttl_seconds: ::std::num::NonZeroU64,
            #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
            vary: ::std::vec::Vec<EdgeRuleActionAuthoringVaryItem>,
        },
        #[serde(rename = "bypass_cache")]
        BypassCache,
    }
    ///`EdgeRuleActionAuthoringHeadersItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringHeadersItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringHeadersItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringHeadersItem> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringHeadersItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringHeadersItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringHeadersItem {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleActionAuthoringHeadersKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringHeadersKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleActionAuthoringMode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "shadow",
    ///    "enforce"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleActionAuthoringMode {
        #[serde(rename = "shadow")]
        Shadow,
        #[serde(rename = "enforce")]
        Enforce,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringMode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Shadow => f.write_str("shadow"),
                Self::Enforce => f.write_str("enforce"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringMode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "shadow" => Ok(Self::Shadow),
                "enforce" => Ok(Self::Enforce),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringMode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStatusCode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "anyOf": [
    ///    {
    ///      "type": "number",
    ///      "const": 301
    ///    },
    ///    {
    ///      "type": "number",
    ///      "const": 302
    ///    },
    ///    {
    ///      "type": "number",
    ///      "const": 307
    ///    },
    ///    {
    ///      "type": "number",
    ///      "const": 308
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(untagged)]
    pub enum EdgeRuleActionAuthoringStatusCode {
        Variant0(f64),
        Variant1(f64),
        Variant2(f64),
        Variant3(f64),
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStatusCode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if let Ok(v) = value.parse() {
                Ok(Self::Variant0(v))
            } else if let Ok(v) = value.parse() {
                Ok(Self::Variant1(v))
            } else if let Ok(v) = value.parse() {
                Ok(Self::Variant2(v))
            } else if let Ok(v) = value.parse() {
                Ok(Self::Variant3(v))
            } else {
                Err("string conversion failed for all variants".into())
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStatusCode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringStatusCode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStatusCode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStatusCode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                Self::Variant0(x) => x.fmt(f),
                Self::Variant1(x) => x.fmt(f),
                Self::Variant2(x) => x.fmt(f),
                Self::Variant3(x) => x.fmt(f),
            }
        }
    }
    ///`EdgeRuleActionAuthoringTarget`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringTarget(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringTarget {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringTarget> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringTarget) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringTarget {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringTarget {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringTarget {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringTarget {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringTarget {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleActionAuthoringVaryItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "geo",
    ///    "device",
    ///    "header",
    ///    "cookie",
    ///    "query"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleActionAuthoringVaryItem {
        #[serde(rename = "geo")]
        Geo,
        #[serde(rename = "device")]
        Device,
        #[serde(rename = "header")]
        Header,
        #[serde(rename = "cookie")]
        Cookie,
        #[serde(rename = "query")]
        Query,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringVaryItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Geo => f.write_str("geo"),
                Self::Device => f.write_str("device"),
                Self::Header => f.write_str("header"),
                Self::Cookie => f.write_str("cookie"),
                Self::Query => f.write_str("query"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringVaryItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "geo" => Ok(Self::Geo),
                "device" => Ok(Self::Device),
                "header" => Ok(Self::Header),
                "cookie" => Ok(Self::Cookie),
                "query" => Ok(Self::Query),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoring`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "action",
    ///    "id"
    ///  ],
    ///  "properties": {
    ///    "action": {
    ///      "$ref": "#/definitions/EdgeRuleActionAuthoring"
    ///    },
    ///    "condition": {
    ///      "$ref": "#/definitions/EdgeRuleCondition"
    ///    },
    ///    "enabled": {
    ///      "type": "boolean"
    ///    },
    ///    "id": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "name": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleAuthoring {
        pub action: EdgeRuleActionAuthoring,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub condition: ::std::option::Option<EdgeRuleCondition>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub enabled: ::std::option::Option<bool>,
        pub id: EdgeRuleAuthoringId,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub name: ::std::option::Option<EdgeRuleAuthoringName>,
    }
    ///`EdgeRuleAuthoringId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleAuthoringId(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringId> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleAuthoringName`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleAuthoringName(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringName {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringName> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringName) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringName {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringName {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringName {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleCondition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "cookies": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    },
    ///    "device": {
    ///      "type": "string",
    ///      "enum": [
    ///        "desktop",
    ///        "mobile",
    ///        "tablet",
    ///        "bot"
    ///      ]
    ///    },
    ///    "geo": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "maxLength": 2,
    ///        "minLength": 2
    ///      }
    ///    },
    ///    "headers": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    },
    ///    "host": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "methods": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "enum": [
    ///          "GET",
    ///          "POST",
    ///          "PUT",
    ///          "DELETE",
    ///          "PATCH",
    ///          "HEAD",
    ///          "OPTIONS"
    ///        ]
    ///      }
    ///    },
    ///    "path": {
    ///      "type": "object",
    ///      "required": [
    ///        "type",
    ///        "value"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "enum": [
    ///            "exact",
    ///            "prefix",
    ///            "regex"
    ///          ]
    ///        },
    ///        "value": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "query": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    },
    ///    "sourceIpCidrs": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleCondition {
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies:
            ::std::collections::HashMap<EdgeRuleConditionCookiesKey, ::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleConditionDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleConditionGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers:
            ::std::collections::HashMap<EdgeRuleConditionHeadersKey, ::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleConditionHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleConditionMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleConditionPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query: ::std::collections::HashMap<EdgeRuleConditionQueryKey, ::std::string::String>,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleConditionSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleCondition {
        fn default() -> Self {
            Self {
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                methods: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleConditionCookiesKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleConditionCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionCookiesKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionDevice`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "desktop",
    ///    "mobile",
    ///    "tablet",
    ///    "bot"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleConditionDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleConditionDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionDevice {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "desktop" => Ok(Self::Desktop),
                "mobile" => Ok(Self::Mobile),
                "tablet" => Ok(Self::Tablet),
                "bot" => Ok(Self::Bot),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleConditionGeoItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 2,
    ///  "minLength": 2
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleConditionGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionGeoItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 2usize {
                return Err("longer than 2 characters".into());
            }
            if value.chars().count() < 2usize {
                return Err("shorter than 2 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionGeoItem {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionHeadersKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleConditionHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionHeadersKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionHost`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionHost> for ::std::string::String {
        fn from(value: EdgeRuleConditionHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionHost {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionMethodsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "GET",
    ///    "POST",
    ///    "PUT",
    ///    "DELETE",
    ///    "PATCH",
    ///    "HEAD",
    ///    "OPTIONS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleConditionMethodsItem {
        #[serde(rename = "GET")]
        Get,
        #[serde(rename = "POST")]
        Post,
        #[serde(rename = "PUT")]
        Put,
        #[serde(rename = "DELETE")]
        Delete,
        #[serde(rename = "PATCH")]
        Patch,
        #[serde(rename = "HEAD")]
        Head,
        #[serde(rename = "OPTIONS")]
        Options,
    }
    impl ::std::fmt::Display for EdgeRuleConditionMethodsItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Get => f.write_str("GET"),
                Self::Post => f.write_str("POST"),
                Self::Put => f.write_str("PUT"),
                Self::Delete => f.write_str("DELETE"),
                Self::Patch => f.write_str("PATCH"),
                Self::Head => f.write_str("HEAD"),
                Self::Options => f.write_str("OPTIONS"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionMethodsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "GET" => Ok(Self::Get),
                "POST" => Ok(Self::Post),
                "PUT" => Ok(Self::Put),
                "DELETE" => Ok(Self::Delete),
                "PATCH" => Ok(Self::Patch),
                "HEAD" => Ok(Self::Head),
                "OPTIONS" => Ok(Self::Options),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleConditionPath`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "type",
    ///    "value"
    ///  ],
    ///  "properties": {
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "exact",
    ///        "prefix",
    ///        "regex"
    ///      ]
    ///    },
    ///    "value": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleConditionPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleConditionPathType,
        pub value: EdgeRuleConditionPathValue,
    }
    ///`EdgeRuleConditionPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "regex"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleConditionPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "regex")]
        Regex,
    }
    impl ::std::fmt::Display for EdgeRuleConditionPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Regex => f.write_str("regex"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "regex" => Ok(Self::Regex),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleConditionPathValue`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionPathValue> for ::std::string::String {
        fn from(value: EdgeRuleConditionPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionPathValue {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionQueryKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleConditionQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionQueryKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionSourceIpCidrsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionSourceIpCidrsItem> for ::std::string::String {
        fn from(value: EdgeRuleConditionSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionSourceIpCidrsItem {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///Authoring contract for onreza.rules.toml. nrz-cli and the publish platform validate this shape, then the platform normalizes it into the runtime EdgeRuleSet served by the edge runtime. Server validation additionally enforces unique rule ids and cache-rule Vary coverage.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "$id": "https://docs.onreza.ru/schemas/onreza-rules-v1.schema.json",
    ///  "title": "ONREZA Edge Rule Set v1",
    ///  "description": "Authoring contract for onreza.rules.toml. nrz-cli and the publish platform validate this shape, then the platform normalizes it into the runtime EdgeRuleSet served by the edge runtime. Server validation additionally enforces unique rule ids and cache-rule Vary coverage.",
    ///  "examples": [
    ///    {
    ///      "rules": [
    ///        {
    ///          "action": {
    ///            "statusCode": 301,
    ///            "target": "/docs",
    ///            "type": "redirect"
    ///          },
    ///          "condition": {
    ///            "path": {
    ///              "type": "prefix",
    ///              "value": "/old-docs"
    ///            }
    ///          },
    ///          "id": "redirect-old-docs",
    ///          "name": "Redirect old docs"
    ///        },
    ///        {
    ///          "action": {
    ///            "ttlSeconds": 3600,
    ///            "type": "cache"
    ///          },
    ///          "condition": {
    ///            "path": {
    ///              "type": "prefix",
    ///              "value": "/assets"
    ///            }
    ///          },
    ///          "id": "cache-assets"
    ///        }
    ///      ],
    ///      "schemaVersion": "EDGE_RULE_SET_V1",
    ///      "source": {
    ///        "origin": "build"
    ///      }
    ///    }
    ///  ],
    ///  "type": "object",
    ///  "required": [
    ///    "rules",
    ///    "schemaVersion",
    ///    "source"
    ///  ],
    ///  "properties": {
    ///    "rules": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/definitions/EdgeRuleAuthoring"
    ///      }
    ///    },
    ///    "schemaVersion": {
    ///      "type": "string",
    ///      "const": "EDGE_RULE_SET_V1"
    ///    },
    ///    "source": {
    ///      "type": "object",
    ///      "required": [
    ///        "origin"
    ///      ],
    ///      "properties": {
    ///        "origin": {
    ///          "type": "string",
    ///          "enum": [
    ///            "build",
    ///            "ui"
    ///          ]
    ///        },
    ///        "revisionId": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  },
    ///  "additionalProperties": false,
    ///  "x-onreza-refinements": [
    ///    "unique rule ids per set",
    ///    "cache rule must Vary by request-dependent condition dimensions"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaEdgeRuleSetV1 {
        pub rules: ::std::vec::Vec<EdgeRuleAuthoring>,
        #[serde(rename = "schemaVersion")]
        pub schema_version: ::std::string::String,
        pub source: OnrezaEdgeRuleSetV1Source,
    }
    ///`OnrezaEdgeRuleSetV1Source`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "origin"
    ///  ],
    ///  "properties": {
    ///    "origin": {
    ///      "type": "string",
    ///      "enum": [
    ///        "build",
    ///        "ui"
    ///      ]
    ///    },
    ///    "revisionId": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaEdgeRuleSetV1Source {
        pub origin: OnrezaEdgeRuleSetV1SourceOrigin,
        #[serde(
            rename = "revisionId",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub revision_id: ::std::option::Option<OnrezaEdgeRuleSetV1SourceRevisionId>,
    }
    ///`OnrezaEdgeRuleSetV1SourceOrigin`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "build",
    ///    "ui"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaEdgeRuleSetV1SourceOrigin {
        #[serde(rename = "build")]
        Build,
        #[serde(rename = "ui")]
        Ui,
    }
    impl ::std::fmt::Display for OnrezaEdgeRuleSetV1SourceOrigin {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Build => f.write_str("build"),
                Self::Ui => f.write_str("ui"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaEdgeRuleSetV1SourceOrigin {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "build" => Ok(Self::Build),
                "ui" => Ok(Self::Ui),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaEdgeRuleSetV1SourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaEdgeRuleSetV1SourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaEdgeRuleSetV1SourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaEdgeRuleSetV1SourceRevisionId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaEdgeRuleSetV1SourceRevisionId(::std::string::String);
    impl ::std::ops::Deref for OnrezaEdgeRuleSetV1SourceRevisionId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaEdgeRuleSetV1SourceRevisionId> for ::std::string::String {
        fn from(value: OnrezaEdgeRuleSetV1SourceRevisionId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaEdgeRuleSetV1SourceRevisionId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaEdgeRuleSetV1SourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaEdgeRuleSetV1SourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaEdgeRuleSetV1SourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaEdgeRuleSetV1SourceRevisionId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
}
pub mod cli_api {
    /// Error types.
    pub mod error {
        /// Error from a `TryFrom` or `FromStr` implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }
        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }
        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }
    ///Public wire contract for nrz-cli deploy upload endpoints. Generated from the Zod source of truth used by the API server and consumed by the Rust CLI contract crate.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "$id": "https://docs.onreza.ru/schemas/cli-api-v1.schema.json",
    ///  "title": "ONREZA CLI API v1",
    ///  "description": "Public wire contract for nrz-cli deploy upload endpoints. Generated from the Zod source of truth used by the API server and consumed by the Rust CLI contract crate.",
    ///  "type": "object",
    ///  "required": [
    ///    "multipartCompleteRequest",
    ///    "multipartCompleteResponse",
    ///    "prepareUploadRequest",
    ///    "prepareUploadResponse",
    ///    "uploadCompleteRequest",
    ///    "uploadCompleteResponse",
    ///    "uploadFailedRequest",
    ///    "uploadFailedResponse"
    ///  ],
    ///  "properties": {
    ///    "multipartCompleteRequest": {
    ///      "type": "object",
    ///      "required": [
    ///        "artifactFormat",
    ///        "deploymentAttemptId",
    ///        "deploymentId",
    ///        "operationId",
    ///        "parts",
    ///        "sourceArtifactId",
    ///        "uploadId",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "artifactFormat": {
    ///          "type": "string",
    ///          "const": "SOURCE_BUNDLE_V1"
    ///        },
    ///        "deploymentAttemptId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "operationId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "parts": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "object",
    ///            "required": [
    ///              "eTag",
    ///              "partNumber"
    ///            ],
    ///            "properties": {
    ///              "eTag": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              },
    ///              "partNumber": {
    ///                "type": "integer",
    ///                "maximum": 9007199254740991.0,
    ///                "minimum": 1.0
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "sourceArtifactId": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "uploadId": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "multipartCompleteResponse": {
    ///      "oneOf": [
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "completedTargets",
    ///            "deploymentId",
    ///            "kind",
    ///            "uploadSessionId"
    ///          ],
    ///          "properties": {
    ///            "completedTargets": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 0.0
    ///            },
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "completed"
    ///            },
    ///            "uploadSessionId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "noop_already_completed"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        }
    ///      ]
    ///    },
    ///    "prepareUploadRequest": {
    ///      "type": "object",
    ///      "required": [
    ///        "artifactFormat",
    ///        "cliProtocolVersion",
    ///        "deploymentAttemptId",
    ///        "deploymentId",
    ///        "logicalManifestSha256",
    ///        "logicalManifestSummary",
    ///        "operationId",
    ///        "projectId",
    ///        "sourceFormat",
    ///        "sourceSha256",
    ///        "sourceSizeBytes",
    ///        "workspaceId"
    ///      ],
    ///      "properties": {
    ///        "artifactFormat": {
    ///          "type": "string",
    ///          "const": "SOURCE_BUNDLE_V1"
    ///        },
    ///        "cliProtocolVersion": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1
    ///        },
    ///        "deploymentAttemptId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "logicalManifestSha256": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "logicalManifestSummary": {
    ///          "type": "object",
    ///          "required": [
    ///            "artifactSizeBytes",
    ///            "fileCount",
    ///            "logicalStaticBytes",
    ///            "maxStaticFileSizeBytes"
    ///          ],
    ///          "properties": {
    ///            "artifactSizeBytes": {
    ///              "type": "string",
    ///              "pattern": "^[0-9]+$"
    ///            },
    ///            "fileCount": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 0.0
    ///            },
    ///            "logicalStaticBytes": {
    ///              "type": "string",
    ///              "pattern": "^[0-9]+$"
    ///            },
    ///            "maxStaticFileSizeBytes": {
    ///              "type": "string",
    ///              "pattern": "^[0-9]+$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "multipart": {
    ///          "type": "object",
    ///          "required": [
    ///            "partCount",
    ///            "partSizeBytes",
    ///            "parts"
    ///          ],
    ///          "properties": {
    ///            "partCount": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 1.0
    ///            },
    ///            "partSizeBytes": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 1.0
    ///            },
    ///            "parts": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "object",
    ///                "required": [
    ///                  "partNumber",
    ///                  "sha256",
    ///                  "sizeBytes"
    ///                ],
    ///                "properties": {
    ///                  "partNumber": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "minimum": 1.0
    ///                  },
    ///                  "sha256": {
    ///                    "type": "string",
    ///                    "pattern": "^[0-9a-f]{64}$"
    ///                  },
    ///                  "sizeBytes": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "minimum": 1.0
    ///                  }
    ///                },
    ///                "additionalProperties": false
    ///              },
    ///              "minItems": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "operationId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "projectId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "sourceFormat": {
    ///          "type": "string",
    ///          "const": "tar.zst"
    ///        },
    ///        "sourceSha256": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "sourceSizeBytes": {
    ///          "type": "string",
    ///          "pattern": "^[0-9]+$"
    ///        },
    ///        "workspaceId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "prepareUploadResponse": {
    ///      "type": "object",
    ///      "required": [
    ///        "bucket",
    ///        "expiresAt",
    ///        "fastPath",
    ///        "kind",
    ///        "requiredComplete",
    ///        "sourceArtifactId",
    ///        "sourceObjectKey",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "bucket": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "expiresAt": {
    ///          "type": "string",
    ///          "format": "date-time",
    ///          "pattern": "^(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))T(?:(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d(?:\\.\\d+)?)?(?:Z))$"
    ///        },
    ///        "fastPath": {
    ///          "type": "boolean"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "source-upload"
    ///        },
    ///        "multipart": {
    ///          "type": "object",
    ///          "required": [
    ///            "chunkSize",
    ///            "chunks",
    ///            "mode",
    ///            "uploadId"
    ///          ],
    ///          "properties": {
    ///            "chunkSize": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 1.0
    ///            },
    ///            "chunks": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "object",
    ///                "required": [
    ///                  "contentLength",
    ///                  "partNumber",
    ///                  "sha256",
    ///                  "url"
    ///                ],
    ///                "properties": {
    ///                  "contentLength": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "minimum": 0.0
    ///                  },
    ///                  "partNumber": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "minimum": 1.0
    ///                  },
    ///                  "sha256": {
    ///                    "type": "string",
    ///                    "pattern": "^[0-9a-f]{64}$"
    ///                  },
    ///                  "url": {
    ///                    "type": "string",
    ///                    "format": "uri"
    ///                  }
    ///                },
    ///                "additionalProperties": false
    ///              },
    ///              "minItems": 1
    ///            },
    ///            "mode": {
    ///              "type": "string",
    ///              "const": "multipart"
    ///            },
    ///            "uploadId": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "presignedPut": {
    ///          "type": "object",
    ///          "required": [
    ///            "contentLength",
    ///            "mode",
    ///            "sha256",
    ///            "url"
    ///          ],
    ///          "properties": {
    ///            "contentLength": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 0.0
    ///            },
    ///            "headers": {
    ///              "type": "object",
    ///              "required": [
    ///                "content-type"
    ///              ],
    ///              "properties": {
    ///                "content-type": {
    ///                  "type": "string",
    ///                  "const": "application/zstd"
    ///                },
    ///                "if-none-match": {
    ///                  "type": "string",
    ///                  "const": "*"
    ///                }
    ///              },
    ///              "additionalProperties": false
    ///            },
    ///            "mode": {
    ///              "type": "string",
    ///              "const": "single"
    ///            },
    ///            "sha256": {
    ///              "type": "string",
    ///              "pattern": "^[0-9a-f]{64}$"
    ///            },
    ///            "url": {
    ///              "type": "string",
    ///              "format": "uri"
    ///            },
    ///            "verifyHead": {
    ///              "type": "object",
    ///              "required": [
    ///                "contentLength",
    ///                "sha256",
    ///                "url"
    ///              ],
    ///              "properties": {
    ///                "contentLength": {
    ///                  "type": "integer",
    ///                  "maximum": 9007199254740991.0,
    ///                  "minimum": 0.0
    ///                },
    ///                "sha256": {
    ///                  "type": "string",
    ///                  "pattern": "^[0-9a-f]{64}$"
    ///                },
    ///                "url": {
    ///                  "type": "string",
    ///                  "format": "uri"
    ///                }
    ///              },
    ///              "additionalProperties": false
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "requiredComplete": {
    ///          "type": "string",
    ///          "enum": [
    ///            "upload-complete",
    ///            "multipart-complete+upload-complete"
    ///          ]
    ///        },
    ///        "sourceArtifactId": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "sourceObjectKey": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "uploadCompleteRequest": {
    ///      "type": "object",
    ///      "required": [
    ///        "artifactFormat",
    ///        "deploymentAttemptId",
    ///        "deploymentId",
    ///        "logicalManifestSha256",
    ///        "operationId",
    ///        "sourceArtifactId",
    ///        "sourceSha256",
    ///        "sourceSizeBytes",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "artifactFormat": {
    ///          "type": "string",
    ///          "const": "SOURCE_BUNDLE_V1"
    ///        },
    ///        "deploymentAttemptId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "logicalManifestSha256": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "operationId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "sourceArtifactId": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "sourceSha256": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "sourceSizeBytes": {
    ///          "type": "string",
    ///          "pattern": "^[0-9]+$"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "uploadCompleteResponse": {
    ///      "oneOf": [
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind",
    ///            "uploadSessionId"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "source-upload-completed"
    ///            },
    ///            "uploadSessionId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind",
    ///            "uploadSessionId"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "source-fast-path-completed"
    ///            },
    ///            "uploadSessionId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind",
    ///            "uploadSessionId"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "source-verified-awaiting-runtime"
    ///            },
    ///            "uploadSessionId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "expiredAt",
    ///            "kind"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "expiredAt": {
    ///              "type": "string",
    ///              "format": "date-time",
    ///              "pattern": "^(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))T(?:(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d(?:\\.\\d+)?)?(?:Z))$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "expired"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "kind",
    ///            "missingSourceObject"
    ///          ],
    ///          "properties": {
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "incomplete"
    ///            },
    ///            "missingSourceObject": {
    ///              "type": "boolean"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "noop_already_completed"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        }
    ///      ]
    ///    },
    ///    "uploadFailedRequest": {
    ///      "type": "object",
    ///      "required": [
    ///        "artifactFormat",
    ///        "deploymentAttemptId",
    ///        "deploymentId",
    ///        "errorCode",
    ///        "errorLog",
    ///        "operationId",
    ///        "sourceArtifactId",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "artifactFormat": {
    ///          "type": "string",
    ///          "const": "SOURCE_BUNDLE_V1"
    ///        },
    ///        "deploymentAttemptId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "errorCode": {
    ///          "type": "string",
    ///          "pattern": "^[A-Z0-9_]{1,64}$"
    ///        },
    ///        "errorLog": {
    ///          "type": "string",
    ///          "maxLength": 4096,
    ///          "minLength": 1
    ///        },
    ///        "operationId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "sourceArtifactId": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "uploadFailedResponse": {
    ///      "oneOf": [
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind",
    ///            "uploadSessionId"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "source-upload-failed"
    ///            },
    ///            "uploadSessionId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        {
    ///          "type": "object",
    ///          "required": [
    ///            "deploymentId",
    ///            "kind",
    ///            "uploadSessionId"
    ///          ],
    ///          "properties": {
    ///            "deploymentId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            },
    ///            "kind": {
    ///              "type": "string",
    ///              "const": "noop_already_accepted"
    ///            },
    ///            "uploadSessionId": {
    ///              "type": "string",
    ///              "format": "uuid",
    ///              "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        }
    ///      ]
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1 {
        #[serde(rename = "multipartCompleteRequest")]
        pub multipart_complete_request: OnrezaCliApiV1MultipartCompleteRequest,
        #[serde(rename = "multipartCompleteResponse")]
        pub multipart_complete_response: OnrezaCliApiV1MultipartCompleteResponse,
        #[serde(rename = "prepareUploadRequest")]
        pub prepare_upload_request: OnrezaCliApiV1PrepareUploadRequest,
        #[serde(rename = "prepareUploadResponse")]
        pub prepare_upload_response: OnrezaCliApiV1PrepareUploadResponse,
        #[serde(rename = "uploadCompleteRequest")]
        pub upload_complete_request: OnrezaCliApiV1UploadCompleteRequest,
        #[serde(rename = "uploadCompleteResponse")]
        pub upload_complete_response: OnrezaCliApiV1UploadCompleteResponse,
        #[serde(rename = "uploadFailedRequest")]
        pub upload_failed_request: OnrezaCliApiV1UploadFailedRequest,
        #[serde(rename = "uploadFailedResponse")]
        pub upload_failed_response: OnrezaCliApiV1UploadFailedResponse,
    }
    ///`OnrezaCliApiV1MultipartCompleteRequest`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "artifactFormat",
    ///    "deploymentAttemptId",
    ///    "deploymentId",
    ///    "operationId",
    ///    "parts",
    ///    "sourceArtifactId",
    ///    "uploadId",
    ///    "uploadSessionId"
    ///  ],
    ///  "properties": {
    ///    "artifactFormat": {
    ///      "type": "string",
    ///      "const": "SOURCE_BUNDLE_V1"
    ///    },
    ///    "deploymentAttemptId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "deploymentId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "operationId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "parts": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "required": [
    ///          "eTag",
    ///          "partNumber"
    ///        ],
    ///        "properties": {
    ///          "eTag": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          },
    ///          "partNumber": {
    ///            "type": "integer",
    ///            "maximum": 9007199254740991.0,
    ///            "minimum": 1.0
    ///          }
    ///        },
    ///        "additionalProperties": false
    ///      },
    ///      "minItems": 1
    ///    },
    ///    "sourceArtifactId": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "uploadId": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "uploadSessionId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1MultipartCompleteRequest {
        #[serde(rename = "artifactFormat")]
        pub artifact_format: ::std::string::String,
        #[serde(rename = "deploymentAttemptId")]
        pub deployment_attempt_id: ::uuid::Uuid,
        #[serde(rename = "deploymentId")]
        pub deployment_id: ::uuid::Uuid,
        #[serde(rename = "operationId")]
        pub operation_id: ::uuid::Uuid,
        pub parts: ::std::vec::Vec<OnrezaCliApiV1MultipartCompleteRequestPartsItem>,
        #[serde(rename = "sourceArtifactId")]
        pub source_artifact_id: OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId,
        #[serde(rename = "uploadId")]
        pub upload_id: OnrezaCliApiV1MultipartCompleteRequestUploadId,
        #[serde(rename = "uploadSessionId")]
        pub upload_session_id: ::uuid::Uuid,
    }
    ///`OnrezaCliApiV1MultipartCompleteRequestPartsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "eTag",
    ///    "partNumber"
    ///  ],
    ///  "properties": {
    ///    "eTag": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "partNumber": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1MultipartCompleteRequestPartsItem {
        #[serde(rename = "eTag")]
        pub e_tag: OnrezaCliApiV1MultipartCompleteRequestPartsItemETag,
        #[serde(rename = "partNumber")]
        pub part_number: ::std::num::NonZeroU64,
    }
    ///`OnrezaCliApiV1MultipartCompleteRequestPartsItemETag`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1MultipartCompleteRequestPartsItemETag(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1MultipartCompleteRequestPartsItemETag {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1MultipartCompleteRequestPartsItemETag>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1MultipartCompleteRequestPartsItemETag) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1MultipartCompleteRequestPartsItemETag {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1MultipartCompleteRequestPartsItemETag {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1MultipartCompleteRequestPartsItemETag
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1MultipartCompleteRequestPartsItemETag
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1MultipartCompleteRequestPartsItemETag {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1MultipartCompleteRequestSourceArtifactId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1MultipartCompleteRequestUploadId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1MultipartCompleteRequestUploadId(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1MultipartCompleteRequestUploadId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1MultipartCompleteRequestUploadId>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1MultipartCompleteRequestUploadId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1MultipartCompleteRequestUploadId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1MultipartCompleteRequestUploadId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1MultipartCompleteRequestUploadId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1MultipartCompleteRequestUploadId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1MultipartCompleteRequestUploadId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1MultipartCompleteResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "completedTargets",
    ///        "deploymentId",
    ///        "kind",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "completedTargets": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 0.0
    ///        },
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "completed"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "noop_already_completed"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(tag = "kind", deny_unknown_fields)]
    pub enum OnrezaCliApiV1MultipartCompleteResponse {
        #[serde(rename = "completed")]
        Completed {
            #[serde(rename = "completedTargets")]
            completed_targets: i64,
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "uploadSessionId")]
            upload_session_id: ::uuid::Uuid,
        },
        #[serde(rename = "noop_already_completed")]
        NoopAlreadyCompleted {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
        },
    }
    ///`OnrezaCliApiV1PrepareUploadRequest`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "artifactFormat",
    ///    "cliProtocolVersion",
    ///    "deploymentAttemptId",
    ///    "deploymentId",
    ///    "logicalManifestSha256",
    ///    "logicalManifestSummary",
    ///    "operationId",
    ///    "projectId",
    ///    "sourceFormat",
    ///    "sourceSha256",
    ///    "sourceSizeBytes",
    ///    "workspaceId"
    ///  ],
    ///  "properties": {
    ///    "artifactFormat": {
    ///      "type": "string",
    ///      "const": "SOURCE_BUNDLE_V1"
    ///    },
    ///    "cliProtocolVersion": {
    ///      "type": "string",
    ///      "maxLength": 64,
    ///      "minLength": 1
    ///    },
    ///    "deploymentAttemptId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "deploymentId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "logicalManifestSha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "logicalManifestSummary": {
    ///      "type": "object",
    ///      "required": [
    ///        "artifactSizeBytes",
    ///        "fileCount",
    ///        "logicalStaticBytes",
    ///        "maxStaticFileSizeBytes"
    ///      ],
    ///      "properties": {
    ///        "artifactSizeBytes": {
    ///          "type": "string",
    ///          "pattern": "^[0-9]+$"
    ///        },
    ///        "fileCount": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 0.0
    ///        },
    ///        "logicalStaticBytes": {
    ///          "type": "string",
    ///          "pattern": "^[0-9]+$"
    ///        },
    ///        "maxStaticFileSizeBytes": {
    ///          "type": "string",
    ///          "pattern": "^[0-9]+$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "multipart": {
    ///      "type": "object",
    ///      "required": [
    ///        "partCount",
    ///        "partSizeBytes",
    ///        "parts"
    ///      ],
    ///      "properties": {
    ///        "partCount": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 1.0
    ///        },
    ///        "partSizeBytes": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 1.0
    ///        },
    ///        "parts": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "object",
    ///            "required": [
    ///              "partNumber",
    ///              "sha256",
    ///              "sizeBytes"
    ///            ],
    ///            "properties": {
    ///              "partNumber": {
    ///                "type": "integer",
    ///                "maximum": 9007199254740991.0,
    ///                "minimum": 1.0
    ///              },
    ///              "sha256": {
    ///                "type": "string",
    ///                "pattern": "^[0-9a-f]{64}$"
    ///              },
    ///              "sizeBytes": {
    ///                "type": "integer",
    ///                "maximum": 9007199254740991.0,
    ///                "minimum": 1.0
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "minItems": 1
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "operationId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "projectId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "sourceFormat": {
    ///      "type": "string",
    ///      "const": "tar.zst"
    ///    },
    ///    "sourceSha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "sourceSizeBytes": {
    ///      "type": "string",
    ///      "pattern": "^[0-9]+$"
    ///    },
    ///    "workspaceId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadRequest {
        #[serde(rename = "artifactFormat")]
        pub artifact_format: ::std::string::String,
        #[serde(rename = "cliProtocolVersion")]
        pub cli_protocol_version: OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion,
        #[serde(rename = "deploymentAttemptId")]
        pub deployment_attempt_id: ::uuid::Uuid,
        #[serde(rename = "deploymentId")]
        pub deployment_id: ::uuid::Uuid,
        #[serde(rename = "logicalManifestSha256")]
        pub logical_manifest_sha256: OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256,
        #[serde(rename = "logicalManifestSummary")]
        pub logical_manifest_summary: OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummary,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub multipart: ::std::option::Option<OnrezaCliApiV1PrepareUploadRequestMultipart>,
        #[serde(rename = "operationId")]
        pub operation_id: ::uuid::Uuid,
        #[serde(rename = "projectId")]
        pub project_id: ::uuid::Uuid,
        #[serde(rename = "sourceFormat")]
        pub source_format: ::std::string::String,
        #[serde(rename = "sourceSha256")]
        pub source_sha256: OnrezaCliApiV1PrepareUploadRequestSourceSha256,
        #[serde(rename = "sourceSizeBytes")]
        pub source_size_bytes: OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes,
        #[serde(rename = "workspaceId")]
        pub workspace_id: ::uuid::Uuid,
    }
    ///`OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 64,
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 64usize {
                return Err("longer than 64 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadRequestCliProtocolVersion {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256 {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSha256 {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummary`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "artifactSizeBytes",
    ///    "fileCount",
    ///    "logicalStaticBytes",
    ///    "maxStaticFileSizeBytes"
    ///  ],
    ///  "properties": {
    ///    "artifactSizeBytes": {
    ///      "type": "string",
    ///      "pattern": "^[0-9]+$"
    ///    },
    ///    "fileCount": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 0.0
    ///    },
    ///    "logicalStaticBytes": {
    ///      "type": "string",
    ///      "pattern": "^[0-9]+$"
    ///    },
    ///    "maxStaticFileSizeBytes": {
    ///      "type": "string",
    ///      "pattern": "^[0-9]+$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummary {
        #[serde(rename = "artifactSizeBytes")]
        pub artifact_size_bytes:
            OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes,
        #[serde(rename = "fileCount")]
        pub file_count: i64,
        #[serde(rename = "logicalStaticBytes")]
        pub logical_static_bytes:
            OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes,
        #[serde(rename = "maxStaticFileSizeBytes")]
        pub max_static_file_size_bytes:
            OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes,
    }
    ///`OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9]+$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes(
        ::std::string::String,
    );
    impl ::std::ops::Deref
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes
    {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl
        ::std::convert::From<
            OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes,
        > for ::std::string::String
    {
        fn from(
            value: OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes,
        ) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes
    {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9]+$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9]+$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryArtifactSizeBytes
    {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9]+$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes(
        ::std::string::String,
    );
    impl ::std::ops::Deref
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes
    {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl
        ::std::convert::From<
            OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes,
        > for ::std::string::String
    {
        fn from(
            value: OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes,
        ) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes
    {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9]+$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9]+$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryLogicalStaticBytes
    {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9]+$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes(
        ::std::string::String,
    );
    impl ::std::ops::Deref
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes
    {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl
        ::std::convert::From<
            OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes,
        > for ::std::string::String
    {
        fn from(
            value: OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes,
        ) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes
    {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9]+$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9]+$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de>
        for OnrezaCliApiV1PrepareUploadRequestLogicalManifestSummaryMaxStaticFileSizeBytes
    {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestMultipart`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "partCount",
    ///    "partSizeBytes",
    ///    "parts"
    ///  ],
    ///  "properties": {
    ///    "partCount": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    },
    ///    "partSizeBytes": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    },
    ///    "parts": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "required": [
    ///          "partNumber",
    ///          "sha256",
    ///          "sizeBytes"
    ///        ],
    ///        "properties": {
    ///          "partNumber": {
    ///            "type": "integer",
    ///            "maximum": 9007199254740991.0,
    ///            "minimum": 1.0
    ///          },
    ///          "sha256": {
    ///            "type": "string",
    ///            "pattern": "^[0-9a-f]{64}$"
    ///          },
    ///          "sizeBytes": {
    ///            "type": "integer",
    ///            "maximum": 9007199254740991.0,
    ///            "minimum": 1.0
    ///          }
    ///        },
    ///        "additionalProperties": false
    ///      },
    ///      "minItems": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadRequestMultipart {
        #[serde(rename = "partCount")]
        pub part_count: ::std::num::NonZeroU64,
        #[serde(rename = "partSizeBytes")]
        pub part_size_bytes: ::std::num::NonZeroU64,
        pub parts: ::std::vec::Vec<OnrezaCliApiV1PrepareUploadRequestMultipartPartsItem>,
    }
    ///`OnrezaCliApiV1PrepareUploadRequestMultipartPartsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "partNumber",
    ///    "sha256",
    ///    "sizeBytes"
    ///  ],
    ///  "properties": {
    ///    "partNumber": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    },
    ///    "sha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "sizeBytes": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadRequestMultipartPartsItem {
        #[serde(rename = "partNumber")]
        pub part_number: ::std::num::NonZeroU64,
        pub sha256: OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256,
        #[serde(rename = "sizeBytes")]
        pub size_bytes: ::std::num::NonZeroU64,
    }
    ///`OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256 {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadRequestMultipartPartsItemSha256 {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestSourceSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestSourceSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadRequestSourceSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadRequestSourceSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadRequestSourceSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadRequestSourceSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadRequestSourceSha256 {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestSourceSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestSourceSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadRequestSourceSha256 {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9]+$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9]+$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9]+$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadRequestSourceSizeBytes {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "bucket",
    ///    "expiresAt",
    ///    "fastPath",
    ///    "kind",
    ///    "requiredComplete",
    ///    "sourceArtifactId",
    ///    "sourceObjectKey",
    ///    "uploadSessionId"
    ///  ],
    ///  "properties": {
    ///    "bucket": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "expiresAt": {
    ///      "type": "string",
    ///      "format": "date-time",
    ///      "pattern": "^(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))T(?:(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d(?:\\.\\d+)?)?(?:Z))$"
    ///    },
    ///    "fastPath": {
    ///      "type": "boolean"
    ///    },
    ///    "kind": {
    ///      "type": "string",
    ///      "const": "source-upload"
    ///    },
    ///    "multipart": {
    ///      "type": "object",
    ///      "required": [
    ///        "chunkSize",
    ///        "chunks",
    ///        "mode",
    ///        "uploadId"
    ///      ],
    ///      "properties": {
    ///        "chunkSize": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 1.0
    ///        },
    ///        "chunks": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "object",
    ///            "required": [
    ///              "contentLength",
    ///              "partNumber",
    ///              "sha256",
    ///              "url"
    ///            ],
    ///            "properties": {
    ///              "contentLength": {
    ///                "type": "integer",
    ///                "maximum": 9007199254740991.0,
    ///                "minimum": 0.0
    ///              },
    ///              "partNumber": {
    ///                "type": "integer",
    ///                "maximum": 9007199254740991.0,
    ///                "minimum": 1.0
    ///              },
    ///              "sha256": {
    ///                "type": "string",
    ///                "pattern": "^[0-9a-f]{64}$"
    ///              },
    ///              "url": {
    ///                "type": "string",
    ///                "format": "uri"
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "mode": {
    ///          "type": "string",
    ///          "const": "multipart"
    ///        },
    ///        "uploadId": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "presignedPut": {
    ///      "type": "object",
    ///      "required": [
    ///        "contentLength",
    ///        "mode",
    ///        "sha256",
    ///        "url"
    ///      ],
    ///      "properties": {
    ///        "contentLength": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 0.0
    ///        },
    ///        "headers": {
    ///          "type": "object",
    ///          "required": [
    ///            "content-type"
    ///          ],
    ///          "properties": {
    ///            "content-type": {
    ///              "type": "string",
    ///              "const": "application/zstd"
    ///            },
    ///            "if-none-match": {
    ///              "type": "string",
    ///              "const": "*"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "mode": {
    ///          "type": "string",
    ///          "const": "single"
    ///        },
    ///        "sha256": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "url": {
    ///          "type": "string",
    ///          "format": "uri"
    ///        },
    ///        "verifyHead": {
    ///          "type": "object",
    ///          "required": [
    ///            "contentLength",
    ///            "sha256",
    ///            "url"
    ///          ],
    ///          "properties": {
    ///            "contentLength": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "minimum": 0.0
    ///            },
    ///            "sha256": {
    ///              "type": "string",
    ///              "pattern": "^[0-9a-f]{64}$"
    ///            },
    ///            "url": {
    ///              "type": "string",
    ///              "format": "uri"
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "requiredComplete": {
    ///      "type": "string",
    ///      "enum": [
    ///        "upload-complete",
    ///        "multipart-complete+upload-complete"
    ///      ]
    ///    },
    ///    "sourceArtifactId": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "sourceObjectKey": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "uploadSessionId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadResponse {
        pub bucket: OnrezaCliApiV1PrepareUploadResponseBucket,
        #[serde(rename = "expiresAt")]
        pub expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
        #[serde(rename = "fastPath")]
        pub fast_path: bool,
        pub kind: ::std::string::String,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub multipart: ::std::option::Option<OnrezaCliApiV1PrepareUploadResponseMultipart>,
        #[serde(
            rename = "presignedPut",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub presigned_put: ::std::option::Option<OnrezaCliApiV1PrepareUploadResponsePresignedPut>,
        #[serde(rename = "requiredComplete")]
        pub required_complete: OnrezaCliApiV1PrepareUploadResponseRequiredComplete,
        #[serde(rename = "sourceArtifactId")]
        pub source_artifact_id: OnrezaCliApiV1PrepareUploadResponseSourceArtifactId,
        #[serde(rename = "sourceObjectKey")]
        pub source_object_key: OnrezaCliApiV1PrepareUploadResponseSourceObjectKey,
        #[serde(rename = "uploadSessionId")]
        pub upload_session_id: ::uuid::Uuid,
    }
    ///`OnrezaCliApiV1PrepareUploadResponseBucket`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponseBucket(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponseBucket {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponseBucket> for ::std::string::String {
        fn from(value: OnrezaCliApiV1PrepareUploadResponseBucket) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponseBucket {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadResponseBucket {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaCliApiV1PrepareUploadResponseBucket {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaCliApiV1PrepareUploadResponseBucket {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadResponseBucket {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponseMultipart`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "chunkSize",
    ///    "chunks",
    ///    "mode",
    ///    "uploadId"
    ///  ],
    ///  "properties": {
    ///    "chunkSize": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    },
    ///    "chunks": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "required": [
    ///          "contentLength",
    ///          "partNumber",
    ///          "sha256",
    ///          "url"
    ///        ],
    ///        "properties": {
    ///          "contentLength": {
    ///            "type": "integer",
    ///            "maximum": 9007199254740991.0,
    ///            "minimum": 0.0
    ///          },
    ///          "partNumber": {
    ///            "type": "integer",
    ///            "maximum": 9007199254740991.0,
    ///            "minimum": 1.0
    ///          },
    ///          "sha256": {
    ///            "type": "string",
    ///            "pattern": "^[0-9a-f]{64}$"
    ///          },
    ///          "url": {
    ///            "type": "string",
    ///            "format": "uri"
    ///          }
    ///        },
    ///        "additionalProperties": false
    ///      },
    ///      "minItems": 1
    ///    },
    ///    "mode": {
    ///      "type": "string",
    ///      "const": "multipart"
    ///    },
    ///    "uploadId": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadResponseMultipart {
        #[serde(rename = "chunkSize")]
        pub chunk_size: ::std::num::NonZeroU64,
        pub chunks: ::std::vec::Vec<OnrezaCliApiV1PrepareUploadResponseMultipartChunksItem>,
        pub mode: ::std::string::String,
        #[serde(rename = "uploadId")]
        pub upload_id: OnrezaCliApiV1PrepareUploadResponseMultipartUploadId,
    }
    ///`OnrezaCliApiV1PrepareUploadResponseMultipartChunksItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "contentLength",
    ///    "partNumber",
    ///    "sha256",
    ///    "url"
    ///  ],
    ///  "properties": {
    ///    "contentLength": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 0.0
    ///    },
    ///    "partNumber": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 1.0
    ///    },
    ///    "sha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "url": {
    ///      "type": "string",
    ///      "format": "uri"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadResponseMultipartChunksItem {
        #[serde(rename = "contentLength")]
        pub content_length: i64,
        #[serde(rename = "partNumber")]
        pub part_number: ::std::num::NonZeroU64,
        pub sha256: OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256,
        pub url: ::std::string::String,
    }
    ///`OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str>
        for OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de>
        for OnrezaCliApiV1PrepareUploadResponseMultipartChunksItemSha256
    {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponseMultipartUploadId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponseMultipartUploadId(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponseMultipartUploadId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponseMultipartUploadId>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadResponseMultipartUploadId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponseMultipartUploadId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadResponseMultipartUploadId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseMultipartUploadId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseMultipartUploadId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadResponseMultipartUploadId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponsePresignedPut`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "contentLength",
    ///    "mode",
    ///    "sha256",
    ///    "url"
    ///  ],
    ///  "properties": {
    ///    "contentLength": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 0.0
    ///    },
    ///    "headers": {
    ///      "type": "object",
    ///      "required": [
    ///        "content-type"
    ///      ],
    ///      "properties": {
    ///        "content-type": {
    ///          "type": "string",
    ///          "const": "application/zstd"
    ///        },
    ///        "if-none-match": {
    ///          "type": "string",
    ///          "const": "*"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "mode": {
    ///      "type": "string",
    ///      "const": "single"
    ///    },
    ///    "sha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "url": {
    ///      "type": "string",
    ///      "format": "uri"
    ///    },
    ///    "verifyHead": {
    ///      "type": "object",
    ///      "required": [
    ///        "contentLength",
    ///        "sha256",
    ///        "url"
    ///      ],
    ///      "properties": {
    ///        "contentLength": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 0.0
    ///        },
    ///        "sha256": {
    ///          "type": "string",
    ///          "pattern": "^[0-9a-f]{64}$"
    ///        },
    ///        "url": {
    ///          "type": "string",
    ///          "format": "uri"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadResponsePresignedPut {
        #[serde(rename = "contentLength")]
        pub content_length: i64,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub headers: ::std::option::Option<OnrezaCliApiV1PrepareUploadResponsePresignedPutHeaders>,
        pub mode: ::std::string::String,
        pub sha256: OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256,
        pub url: ::std::string::String,
        #[serde(
            rename = "verifyHead",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub verify_head:
            ::std::option::Option<OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHead>,
    }
    ///`OnrezaCliApiV1PrepareUploadResponsePresignedPutHeaders`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "content-type"
    ///  ],
    ///  "properties": {
    ///    "content-type": {
    ///      "type": "string",
    ///      "const": "application/zstd"
    ///    },
    ///    "if-none-match": {
    ///      "type": "string",
    ///      "const": "*"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadResponsePresignedPutHeaders {
        #[serde(rename = "content-type")]
        pub content_type: ::std::string::String,
        #[serde(
            rename = "if-none-match",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub if_none_match: ::std::option::Option<::std::string::String>,
    }
    ///`OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256 {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadResponsePresignedPutSha256 {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHead`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "contentLength",
    ///    "sha256",
    ///    "url"
    ///  ],
    ///  "properties": {
    ///    "contentLength": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 0.0
    ///    },
    ///    "sha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "url": {
    ///      "type": "string",
    ///      "format": "uri"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHead {
        #[serde(rename = "contentLength")]
        pub content_length: i64,
        pub sha256: OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256,
        pub url: ::std::string::String,
    }
    ///`OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256(
        ::std::string::String,
    );
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str>
        for OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de>
        for OnrezaCliApiV1PrepareUploadResponsePresignedPutVerifyHeadSha256
    {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponseRequiredComplete`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "upload-complete",
    ///    "multipart-complete+upload-complete"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaCliApiV1PrepareUploadResponseRequiredComplete {
        #[serde(rename = "upload-complete")]
        UploadComplete,
        #[serde(rename = "multipart-complete+upload-complete")]
        MultipartCompleteUploadComplete,
    }
    impl ::std::fmt::Display for OnrezaCliApiV1PrepareUploadResponseRequiredComplete {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::UploadComplete => f.write_str("upload-complete"),
                Self::MultipartCompleteUploadComplete => {
                    f.write_str("multipart-complete+upload-complete")
                }
            }
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponseRequiredComplete {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "upload-complete" => Ok(Self::UploadComplete),
                "multipart-complete+upload-complete" => Ok(Self::MultipartCompleteUploadComplete),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadResponseRequiredComplete {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseRequiredComplete
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseRequiredComplete
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponseSourceArtifactId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponseSourceArtifactId(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponseSourceArtifactId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponseSourceArtifactId>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadResponseSourceArtifactId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponseSourceArtifactId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadResponseSourceArtifactId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadResponseSourceArtifactId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1PrepareUploadResponseSourceObjectKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1PrepareUploadResponseSourceObjectKey(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1PrepareUploadResponseSourceObjectKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1PrepareUploadResponseSourceObjectKey>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1PrepareUploadResponseSourceObjectKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1PrepareUploadResponseSourceObjectKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1PrepareUploadResponseSourceObjectKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseSourceObjectKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1PrepareUploadResponseSourceObjectKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1PrepareUploadResponseSourceObjectKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadCompleteRequest`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "artifactFormat",
    ///    "deploymentAttemptId",
    ///    "deploymentId",
    ///    "logicalManifestSha256",
    ///    "operationId",
    ///    "sourceArtifactId",
    ///    "sourceSha256",
    ///    "sourceSizeBytes",
    ///    "uploadSessionId"
    ///  ],
    ///  "properties": {
    ///    "artifactFormat": {
    ///      "type": "string",
    ///      "const": "SOURCE_BUNDLE_V1"
    ///    },
    ///    "deploymentAttemptId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "deploymentId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "logicalManifestSha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "operationId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "sourceArtifactId": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "sourceSha256": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "sourceSizeBytes": {
    ///      "type": "string",
    ///      "pattern": "^[0-9]+$"
    ///    },
    ///    "uploadSessionId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1UploadCompleteRequest {
        #[serde(rename = "artifactFormat")]
        pub artifact_format: ::std::string::String,
        #[serde(rename = "deploymentAttemptId")]
        pub deployment_attempt_id: ::uuid::Uuid,
        #[serde(rename = "deploymentId")]
        pub deployment_id: ::uuid::Uuid,
        #[serde(rename = "logicalManifestSha256")]
        pub logical_manifest_sha256: OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256,
        #[serde(rename = "operationId")]
        pub operation_id: ::uuid::Uuid,
        #[serde(rename = "sourceArtifactId")]
        pub source_artifact_id: OnrezaCliApiV1UploadCompleteRequestSourceArtifactId,
        #[serde(rename = "sourceSha256")]
        pub source_sha256: OnrezaCliApiV1UploadCompleteRequestSourceSha256,
        #[serde(rename = "sourceSizeBytes")]
        pub source_size_bytes: OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes,
        #[serde(rename = "uploadSessionId")]
        pub upload_session_id: ::uuid::Uuid,
    }
    ///`OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256 {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadCompleteRequestLogicalManifestSha256 {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadCompleteRequestSourceArtifactId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadCompleteRequestSourceArtifactId(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadCompleteRequestSourceArtifactId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadCompleteRequestSourceArtifactId>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1UploadCompleteRequestSourceArtifactId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadCompleteRequestSourceArtifactId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadCompleteRequestSourceArtifactId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadCompleteRequestSourceArtifactId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadCompleteRequestSourceSha256`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadCompleteRequestSourceSha256(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadCompleteRequestSourceSha256 {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadCompleteRequestSourceSha256>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1UploadCompleteRequestSourceSha256) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadCompleteRequestSourceSha256 {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadCompleteRequestSourceSha256 {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestSourceSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestSourceSha256
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadCompleteRequestSourceSha256 {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9]+$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9]+$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9]+$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadCompleteRequestSourceSizeBytes {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadCompleteResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "source-upload-completed"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "source-fast-path-completed"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "source-verified-awaiting-runtime"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "expiredAt",
    ///        "kind"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "expiredAt": {
    ///          "type": "string",
    ///          "format": "date-time",
    ///          "pattern": "^(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))T(?:(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d(?:\\.\\d+)?)?(?:Z))$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "expired"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "kind",
    ///        "missingSourceObject"
    ///      ],
    ///      "properties": {
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "incomplete"
    ///        },
    ///        "missingSourceObject": {
    ///          "type": "boolean"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "noop_already_completed"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(tag = "kind", deny_unknown_fields)]
    pub enum OnrezaCliApiV1UploadCompleteResponse {
        #[serde(rename = "source-upload-completed")]
        SourceUploadCompleted {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "uploadSessionId")]
            upload_session_id: ::uuid::Uuid,
        },
        #[serde(rename = "source-fast-path-completed")]
        SourceFastPathCompleted {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "uploadSessionId")]
            upload_session_id: ::uuid::Uuid,
        },
        #[serde(rename = "source-verified-awaiting-runtime")]
        SourceVerifiedAwaitingRuntime {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "uploadSessionId")]
            upload_session_id: ::uuid::Uuid,
        },
        #[serde(rename = "expired")]
        Expired {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "expiredAt")]
            expired_at: ::chrono::DateTime<::chrono::offset::Utc>,
        },
        #[serde(rename = "incomplete")]
        Incomplete {
            #[serde(rename = "missingSourceObject")]
            missing_source_object: bool,
        },
        #[serde(rename = "noop_already_completed")]
        NoopAlreadyCompleted {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
        },
    }
    ///`OnrezaCliApiV1UploadFailedRequest`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "artifactFormat",
    ///    "deploymentAttemptId",
    ///    "deploymentId",
    ///    "errorCode",
    ///    "errorLog",
    ///    "operationId",
    ///    "sourceArtifactId",
    ///    "uploadSessionId"
    ///  ],
    ///  "properties": {
    ///    "artifactFormat": {
    ///      "type": "string",
    ///      "const": "SOURCE_BUNDLE_V1"
    ///    },
    ///    "deploymentAttemptId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "deploymentId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "errorCode": {
    ///      "type": "string",
    ///      "pattern": "^[A-Z0-9_]{1,64}$"
    ///    },
    ///    "errorLog": {
    ///      "type": "string",
    ///      "maxLength": 4096,
    ///      "minLength": 1
    ///    },
    ///    "operationId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    },
    ///    "sourceArtifactId": {
    ///      "type": "string",
    ///      "pattern": "^[0-9a-f]{64}$"
    ///    },
    ///    "uploadSessionId": {
    ///      "type": "string",
    ///      "format": "uuid",
    ///      "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaCliApiV1UploadFailedRequest {
        #[serde(rename = "artifactFormat")]
        pub artifact_format: ::std::string::String,
        #[serde(rename = "deploymentAttemptId")]
        pub deployment_attempt_id: ::uuid::Uuid,
        #[serde(rename = "deploymentId")]
        pub deployment_id: ::uuid::Uuid,
        #[serde(rename = "errorCode")]
        pub error_code: OnrezaCliApiV1UploadFailedRequestErrorCode,
        #[serde(rename = "errorLog")]
        pub error_log: OnrezaCliApiV1UploadFailedRequestErrorLog,
        #[serde(rename = "operationId")]
        pub operation_id: ::uuid::Uuid,
        #[serde(rename = "sourceArtifactId")]
        pub source_artifact_id: OnrezaCliApiV1UploadFailedRequestSourceArtifactId,
        #[serde(rename = "uploadSessionId")]
        pub upload_session_id: ::uuid::Uuid,
    }
    ///`OnrezaCliApiV1UploadFailedRequestErrorCode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[A-Z0-9_]{1,64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadFailedRequestErrorCode(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadFailedRequestErrorCode {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadFailedRequestErrorCode> for ::std::string::String {
        fn from(value: OnrezaCliApiV1UploadFailedRequestErrorCode) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadFailedRequestErrorCode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[A-Z0-9_]{1,64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[A-Z0-9_]{1,64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadFailedRequestErrorCode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1UploadFailedRequestErrorCode
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaCliApiV1UploadFailedRequestErrorCode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadFailedRequestErrorCode {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadFailedRequestErrorLog`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 4096,
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadFailedRequestErrorLog(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadFailedRequestErrorLog {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadFailedRequestErrorLog> for ::std::string::String {
        fn from(value: OnrezaCliApiV1UploadFailedRequestErrorLog) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadFailedRequestErrorLog {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 4096usize {
                return Err("longer than 4096 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadFailedRequestErrorLog {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaCliApiV1UploadFailedRequestErrorLog {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaCliApiV1UploadFailedRequestErrorLog {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadFailedRequestErrorLog {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadFailedRequestSourceArtifactId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^[0-9a-f]{64}$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaCliApiV1UploadFailedRequestSourceArtifactId(::std::string::String);
    impl ::std::ops::Deref for OnrezaCliApiV1UploadFailedRequestSourceArtifactId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaCliApiV1UploadFailedRequestSourceArtifactId>
        for ::std::string::String
    {
        fn from(value: OnrezaCliApiV1UploadFailedRequestSourceArtifactId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaCliApiV1UploadFailedRequestSourceArtifactId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaCliApiV1UploadFailedRequestSourceArtifactId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaCliApiV1UploadFailedRequestSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaCliApiV1UploadFailedRequestSourceArtifactId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaCliApiV1UploadFailedRequestSourceArtifactId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaCliApiV1UploadFailedResponse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "source-upload-failed"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "deploymentId",
    ///        "kind",
    ///        "uploadSessionId"
    ///      ],
    ///      "properties": {
    ///        "deploymentId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        },
    ///        "kind": {
    ///          "type": "string",
    ///          "const": "noop_already_accepted"
    ///        },
    ///        "uploadSessionId": {
    ///          "type": "string",
    ///          "format": "uuid",
    ///          "pattern": "^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(tag = "kind", deny_unknown_fields)]
    pub enum OnrezaCliApiV1UploadFailedResponse {
        #[serde(rename = "source-upload-failed")]
        SourceUploadFailed {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "uploadSessionId")]
            upload_session_id: ::uuid::Uuid,
        },
        #[serde(rename = "noop_already_accepted")]
        NoopAlreadyAccepted {
            #[serde(rename = "deploymentId")]
            deployment_id: ::uuid::Uuid,
            #[serde(rename = "uploadSessionId")]
            upload_session_id: ::uuid::Uuid,
        },
    }
}
pub mod onreza_functions_publish {
    /// Error types.
    pub mod error {
        /// Error from a `TryFrom` or `FromStr` implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }
        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }
        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }
    ///`EdgeRuleActionAuthoring`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "const": "allow"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "const": "log"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "mode": {
    ///          "type": "string",
    ///          "enum": [
    ///            "shadow",
    ///            "enforce"
    ///          ]
    ///        },
    ///        "statusCode": {
    ///          "type": "integer",
    ///          "maximum": 599.0,
    ///          "minimum": 400.0
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "deny"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "target",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "force": {
    ///          "type": "boolean"
    ///        },
    ///        "statusCode": {
    ///          "anyOf": [
    ///            {
    ///              "type": "number",
    ///              "const": 301
    ///            },
    ///            {
    ///              "type": "number",
    ///              "const": 302
    ///            },
    ///            {
    ///              "type": "number",
    ///              "const": 307
    ///            },
    ///            {
    ///              "type": "number",
    ///              "const": 308
    ///            }
    ///          ]
    ///        },
    ///        "target": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "redirect"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "target",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "external": {
    ///          "type": "boolean"
    ///        },
    ///        "force": {
    ///          "type": "boolean"
    ///        },
    ///        "target": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "rewrite"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "headers",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "headers": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "set_headers"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "headers",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "headers": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "remove_headers"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "ttlSeconds",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "swrSeconds": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": 0.0
    ///        },
    ///        "ttlSeconds": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "exclusiveMinimum": 0.0
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "cache"
    ///        },
    ///        "vary": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "geo",
    ///              "device",
    ///              "header",
    ///              "cookie",
    ///              "query"
    ///            ]
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "const": "bypass_cache"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(tag = "type", deny_unknown_fields)]
    pub enum EdgeRuleActionAuthoring {
        #[serde(rename = "allow")]
        Allow,
        #[serde(rename = "log")]
        Log,
        #[serde(rename = "deny")]
        Deny {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            mode: ::std::option::Option<EdgeRuleActionAuthoringMode>,
            #[serde(
                rename = "statusCode",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            status_code: ::std::option::Option<i64>,
        },
        #[serde(rename = "redirect")]
        Redirect {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            force: ::std::option::Option<bool>,
            #[serde(
                rename = "statusCode",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            status_code: ::std::option::Option<EdgeRuleActionAuthoringStatusCode>,
            target: EdgeRuleActionAuthoringTarget,
        },
        #[serde(rename = "rewrite")]
        Rewrite {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            external: ::std::option::Option<bool>,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            force: ::std::option::Option<bool>,
            target: EdgeRuleActionAuthoringTarget,
        },
        #[serde(rename = "set_headers")]
        SetHeaders {
            headers: ::std::collections::HashMap<
                EdgeRuleActionAuthoringHeadersKey,
                ::std::string::String,
            >,
        },
        #[serde(rename = "remove_headers")]
        RemoveHeaders {
            headers: ::std::vec::Vec<EdgeRuleActionAuthoringHeadersItem>,
        },
        #[serde(rename = "cache")]
        Cache {
            #[serde(
                rename = "swrSeconds",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            swr_seconds: ::std::option::Option<i64>,
            #[serde(rename = "ttlSeconds")]
            ttl_seconds: ::std::num::NonZeroU64,
            #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
            vary: ::std::vec::Vec<EdgeRuleActionAuthoringVaryItem>,
        },
        #[serde(rename = "bypass_cache")]
        BypassCache,
    }
    ///`EdgeRuleActionAuthoringHeadersItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringHeadersItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringHeadersItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringHeadersItem> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringHeadersItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringHeadersItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringHeadersItem {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleActionAuthoringHeadersKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringHeadersKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleActionAuthoringMode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "shadow",
    ///    "enforce"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleActionAuthoringMode {
        #[serde(rename = "shadow")]
        Shadow,
        #[serde(rename = "enforce")]
        Enforce,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringMode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Shadow => f.write_str("shadow"),
                Self::Enforce => f.write_str("enforce"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringMode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "shadow" => Ok(Self::Shadow),
                "enforce" => Ok(Self::Enforce),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringMode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStatusCode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "anyOf": [
    ///    {
    ///      "type": "number",
    ///      "const": 301
    ///    },
    ///    {
    ///      "type": "number",
    ///      "const": 302
    ///    },
    ///    {
    ///      "type": "number",
    ///      "const": 307
    ///    },
    ///    {
    ///      "type": "number",
    ///      "const": 308
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(untagged)]
    pub enum EdgeRuleActionAuthoringStatusCode {
        Variant0(f64),
        Variant1(f64),
        Variant2(f64),
        Variant3(f64),
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStatusCode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if let Ok(v) = value.parse() {
                Ok(Self::Variant0(v))
            } else if let Ok(v) = value.parse() {
                Ok(Self::Variant1(v))
            } else if let Ok(v) = value.parse() {
                Ok(Self::Variant2(v))
            } else if let Ok(v) = value.parse() {
                Ok(Self::Variant3(v))
            } else {
                Err("string conversion failed for all variants".into())
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStatusCode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringStatusCode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStatusCode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStatusCode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                Self::Variant0(x) => x.fmt(f),
                Self::Variant1(x) => x.fmt(f),
                Self::Variant2(x) => x.fmt(f),
                Self::Variant3(x) => x.fmt(f),
            }
        }
    }
    ///`EdgeRuleActionAuthoringTarget`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringTarget(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringTarget {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringTarget> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringTarget) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringTarget {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringTarget {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringTarget {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringTarget {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringTarget {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleActionAuthoringVaryItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "geo",
    ///    "device",
    ///    "header",
    ///    "cookie",
    ///    "query"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleActionAuthoringVaryItem {
        #[serde(rename = "geo")]
        Geo,
        #[serde(rename = "device")]
        Device,
        #[serde(rename = "header")]
        Header,
        #[serde(rename = "cookie")]
        Cookie,
        #[serde(rename = "query")]
        Query,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringVaryItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Geo => f.write_str("geo"),
                Self::Device => f.write_str("device"),
                Self::Header => f.write_str("header"),
                Self::Cookie => f.write_str("cookie"),
                Self::Query => f.write_str("query"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringVaryItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "geo" => Ok(Self::Geo),
                "device" => Ok(Self::Device),
                "header" => Ok(Self::Header),
                "cookie" => Ok(Self::Cookie),
                "query" => Ok(Self::Query),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoring`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "action",
    ///    "id"
    ///  ],
    ///  "properties": {
    ///    "action": {
    ///      "$ref": "#/definitions/EdgeRuleActionAuthoring"
    ///    },
    ///    "condition": {
    ///      "$ref": "#/definitions/EdgeRuleCondition"
    ///    },
    ///    "enabled": {
    ///      "type": "boolean"
    ///    },
    ///    "id": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "name": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleAuthoring {
        pub action: EdgeRuleActionAuthoring,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub condition: ::std::option::Option<EdgeRuleCondition>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub enabled: ::std::option::Option<bool>,
        pub id: EdgeRuleAuthoringId,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub name: ::std::option::Option<EdgeRuleAuthoringName>,
    }
    ///`EdgeRuleAuthoringId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleAuthoringId(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringId> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleAuthoringName`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleAuthoringName(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringName {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringName> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringName) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringName {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringName {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringName {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleCondition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "cookies": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    },
    ///    "device": {
    ///      "type": "string",
    ///      "enum": [
    ///        "desktop",
    ///        "mobile",
    ///        "tablet",
    ///        "bot"
    ///      ]
    ///    },
    ///    "geo": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "maxLength": 2,
    ///        "minLength": 2
    ///      }
    ///    },
    ///    "headers": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    },
    ///    "host": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "methods": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "enum": [
    ///          "GET",
    ///          "POST",
    ///          "PUT",
    ///          "DELETE",
    ///          "PATCH",
    ///          "HEAD",
    ///          "OPTIONS"
    ///        ]
    ///      }
    ///    },
    ///    "path": {
    ///      "type": "object",
    ///      "required": [
    ///        "type",
    ///        "value"
    ///      ],
    ///      "properties": {
    ///        "type": {
    ///          "type": "string",
    ///          "enum": [
    ///            "exact",
    ///            "prefix",
    ///            "regex"
    ///          ]
    ///        },
    ///        "value": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "query": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    },
    ///    "sourceIpCidrs": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "minLength": 1
    ///      }
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleCondition {
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies:
            ::std::collections::HashMap<EdgeRuleConditionCookiesKey, ::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleConditionDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleConditionGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers:
            ::std::collections::HashMap<EdgeRuleConditionHeadersKey, ::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleConditionHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleConditionMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleConditionPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query: ::std::collections::HashMap<EdgeRuleConditionQueryKey, ::std::string::String>,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleConditionSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleCondition {
        fn default() -> Self {
            Self {
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                methods: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleConditionCookiesKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleConditionCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionCookiesKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionDevice`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "desktop",
    ///    "mobile",
    ///    "tablet",
    ///    "bot"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleConditionDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleConditionDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionDevice {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "desktop" => Ok(Self::Desktop),
                "mobile" => Ok(Self::Mobile),
                "tablet" => Ok(Self::Tablet),
                "bot" => Ok(Self::Bot),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleConditionGeoItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 2,
    ///  "minLength": 2
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleConditionGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionGeoItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 2usize {
                return Err("longer than 2 characters".into());
            }
            if value.chars().count() < 2usize {
                return Err("shorter than 2 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionGeoItem {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionHeadersKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleConditionHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionHeadersKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionHost`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionHost> for ::std::string::String {
        fn from(value: EdgeRuleConditionHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionHost {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionMethodsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "GET",
    ///    "POST",
    ///    "PUT",
    ///    "DELETE",
    ///    "PATCH",
    ///    "HEAD",
    ///    "OPTIONS"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleConditionMethodsItem {
        #[serde(rename = "GET")]
        Get,
        #[serde(rename = "POST")]
        Post,
        #[serde(rename = "PUT")]
        Put,
        #[serde(rename = "DELETE")]
        Delete,
        #[serde(rename = "PATCH")]
        Patch,
        #[serde(rename = "HEAD")]
        Head,
        #[serde(rename = "OPTIONS")]
        Options,
    }
    impl ::std::fmt::Display for EdgeRuleConditionMethodsItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Get => f.write_str("GET"),
                Self::Post => f.write_str("POST"),
                Self::Put => f.write_str("PUT"),
                Self::Delete => f.write_str("DELETE"),
                Self::Patch => f.write_str("PATCH"),
                Self::Head => f.write_str("HEAD"),
                Self::Options => f.write_str("OPTIONS"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionMethodsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "GET" => Ok(Self::Get),
                "POST" => Ok(Self::Post),
                "PUT" => Ok(Self::Put),
                "DELETE" => Ok(Self::Delete),
                "PATCH" => Ok(Self::Patch),
                "HEAD" => Ok(Self::Head),
                "OPTIONS" => Ok(Self::Options),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleConditionPath`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "type",
    ///    "value"
    ///  ],
    ///  "properties": {
    ///    "type": {
    ///      "type": "string",
    ///      "enum": [
    ///        "exact",
    ///        "prefix",
    ///        "regex"
    ///      ]
    ///    },
    ///    "value": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleConditionPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleConditionPathType,
        pub value: EdgeRuleConditionPathValue,
    }
    ///`EdgeRuleConditionPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "regex"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleConditionPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "regex")]
        Regex,
    }
    impl ::std::fmt::Display for EdgeRuleConditionPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Regex => f.write_str("regex"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "regex" => Ok(Self::Regex),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleConditionPathValue`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionPathValue> for ::std::string::String {
        fn from(value: EdgeRuleConditionPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionPathValue {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionQueryKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleConditionQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionQueryKey {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleConditionSourceIpCidrsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleConditionSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleConditionSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleConditionSourceIpCidrsItem> for ::std::string::String {
        fn from(value: EdgeRuleConditionSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleConditionSourceIpCidrsItem {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`EdgeRuleSetAuthoring`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "rules",
    ///    "schemaVersion",
    ///    "source"
    ///  ],
    ///  "properties": {
    ///    "rules": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/definitions/EdgeRuleAuthoring"
    ///      }
    ///    },
    ///    "schemaVersion": {
    ///      "type": "string",
    ///      "const": "EDGE_RULE_SET_V1"
    ///    },
    ///    "source": {
    ///      "type": "object",
    ///      "required": [
    ///        "origin"
    ///      ],
    ///      "properties": {
    ///        "origin": {
    ///          "type": "string",
    ///          "enum": [
    ///            "build",
    ///            "ui"
    ///          ]
    ///        },
    ///        "revisionId": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleSetAuthoring {
        pub rules: ::std::vec::Vec<EdgeRuleAuthoring>,
        #[serde(rename = "schemaVersion")]
        pub schema_version: ::std::string::String,
        pub source: EdgeRuleSetAuthoringSource,
    }
    ///`EdgeRuleSetAuthoringSource`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "origin"
    ///  ],
    ///  "properties": {
    ///    "origin": {
    ///      "type": "string",
    ///      "enum": [
    ///        "build",
    ///        "ui"
    ///      ]
    ///    },
    ///    "revisionId": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRuleSetAuthoringSource {
        pub origin: EdgeRuleSetAuthoringSourceOrigin,
        #[serde(
            rename = "revisionId",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub revision_id: ::std::option::Option<EdgeRuleSetAuthoringSourceRevisionId>,
    }
    ///`EdgeRuleSetAuthoringSourceOrigin`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "build",
    ///    "ui"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum EdgeRuleSetAuthoringSourceOrigin {
        #[serde(rename = "build")]
        Build,
        #[serde(rename = "ui")]
        Ui,
    }
    impl ::std::fmt::Display for EdgeRuleSetAuthoringSourceOrigin {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Build => f.write_str("build"),
                Self::Ui => f.write_str("ui"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleSetAuthoringSourceOrigin {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "build" => Ok(Self::Build),
                "ui" => Ok(Self::Ui),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleSetAuthoringSourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleSetAuthoringSourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleSetAuthoringSourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleSetAuthoringSourceRevisionId`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleSetAuthoringSourceRevisionId(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleSetAuthoringSourceRevisionId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleSetAuthoringSourceRevisionId> for ::std::string::String {
        fn from(value: EdgeRuleSetAuthoringSourceRevisionId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleSetAuthoringSourceRevisionId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleSetAuthoringSourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleSetAuthoringSourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleSetAuthoringSourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleSetAuthoringSourceRevisionId {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///Public wire contract for CLI/UI/deploy-origin ONREZA Functions publishing. Each v1 function contains exactly one self-contained entry source file; edge rules use the authoring shape before platform normalization.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "$id": "https://docs.onreza.ru/schemas/onreza-functions-publish-payload-v1.schema.json",
    ///  "title": "ONREZA Functions Publish Payload v1",
    ///  "description": "Public wire contract for CLI/UI/deploy-origin ONREZA Functions publishing. Each v1 function contains exactly one self-contained entry source file; edge rules use the authoring shape before platform normalization.",
    ///  "examples": [
    ///    {
    ///      "edgeRules": {
    ///        "rules": [],
    ///        "schemaVersion": "EDGE_RULE_SET_V1",
    ///        "source": {
    ///          "origin": "build"
    ///        }
    ///      },
    ///      "functions": [
    ///        {
    ///          "source": {
    ///            "contentText": "export const config = { triggers: [{ type: \"http\", matchers: [\"/api/hello\"], methods: [\"GET\"] }] } as const;\\n\\nexport default { fetch() { return Response.json({ ok: true }); } };\\n",
    ///            "path": "api/hello.nrz-fn.ts"
    ///          }
    ///        }
    ///      ],
    ///      "origin": "CLI"
    ///    }
    ///  ],
    ///  "type": "object",
    ///  "required": [
    ///    "origin"
    ///  ],
    ///  "properties": {
    ///    "edgeRules": {
    ///      "$ref": "#/definitions/EdgeRuleSetAuthoring"
    ///    },
    ///    "functions": {
    ///      "default": [],
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "required": [
    ///          "source"
    ///        ],
    ///        "properties": {
    ///          "source": {
    ///            "type": "object",
    ///            "required": [
    ///              "contentText",
    ///              "path"
    ///            ],
    ///            "properties": {
    ///              "contentText": {
    ///                "description": "UTF-8 function source text. ONREZA Functions v1 accepts at most 131072 bytes per entry file.",
    ///                "type": "string",
    ///                "maxLength": 131072
    ///              },
    ///              "path": {
    ///                "description": "Relative self-contained function entry path. Must not contain path traversal, null bytes or node_modules segments, and must end with *.nrz-fn.ts/js/mjs.",
    ///                "type": "string",
    ///                "maxLength": 512,
    ///                "minLength": 1,
    ///                "pattern": "\\.nrz-fn\\.(?:ts|tsx|js|jsx|mjs)$"
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          }
    ///        },
    ///        "additionalProperties": false
    ///      },
    ///      "maxItems": 1000
    ///    },
    ///    "origin": {
    ///      "type": "string",
    ///      "enum": [
    ///        "DEPLOYMENT",
    ///        "UI",
    ///        "CLI"
    ///      ]
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsPublishPayloadV1 {
        #[serde(
            rename = "edgeRules",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub edge_rules: ::std::option::Option<EdgeRuleSetAuthoring>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub functions: ::std::vec::Vec<OnrezaFunctionsPublishPayloadV1FunctionsItem>,
        pub origin: OnrezaFunctionsPublishPayloadV1Origin,
    }
    ///`OnrezaFunctionsPublishPayloadV1FunctionsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "source"
    ///  ],
    ///  "properties": {
    ///    "source": {
    ///      "type": "object",
    ///      "required": [
    ///        "contentText",
    ///        "path"
    ///      ],
    ///      "properties": {
    ///        "contentText": {
    ///          "description": "UTF-8 function source text. ONREZA Functions v1 accepts at most 131072 bytes per entry file.",
    ///          "type": "string",
    ///          "maxLength": 131072
    ///        },
    ///        "path": {
    ///          "description": "Relative self-contained function entry path. Must not contain path traversal, null bytes or node_modules segments, and must end with *.nrz-fn.ts/js/mjs.",
    ///          "type": "string",
    ///          "maxLength": 512,
    ///          "minLength": 1,
    ///          "pattern": "\\.nrz-fn\\.(?:ts|tsx|js|jsx|mjs)$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsPublishPayloadV1FunctionsItem {
        pub source: OnrezaFunctionsPublishPayloadV1FunctionsItemSource,
    }
    ///`OnrezaFunctionsPublishPayloadV1FunctionsItemSource`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "contentText",
    ///    "path"
    ///  ],
    ///  "properties": {
    ///    "contentText": {
    ///      "description": "UTF-8 function source text. ONREZA Functions v1 accepts at most 131072 bytes per entry file.",
    ///      "type": "string",
    ///      "maxLength": 131072
    ///    },
    ///    "path": {
    ///      "description": "Relative self-contained function entry path. Must not contain path traversal, null bytes or node_modules segments, and must end with *.nrz-fn.ts/js/mjs.",
    ///      "type": "string",
    ///      "maxLength": 512,
    ///      "minLength": 1,
    ///      "pattern": "\\.nrz-fn\\.(?:ts|tsx|js|jsx|mjs)$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsPublishPayloadV1FunctionsItemSource {
        ///UTF-8 function source text. ONREZA Functions v1 accepts at most 131072 bytes per entry file.
        #[serde(rename = "contentText")]
        pub content_text: OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText,
        ///Relative self-contained function entry path. Must not contain path traversal, null bytes or node_modules segments, and must end with *.nrz-fn.ts/js/mjs.
        pub path: OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath,
    }
    ///UTF-8 function source text. ONREZA Functions v1 accepts at most 131072 bytes per entry file.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "description": "UTF-8 function source text. ONREZA Functions v1 accepts at most 131072 bytes per entry file.",
    ///  "type": "string",
    ///  "maxLength": 131072
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText>
        for ::std::string::String
    {
        fn from(value: OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 131072usize {
                return Err("longer than 131072 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str>
        for OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText
    {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de>
        for OnrezaFunctionsPublishPayloadV1FunctionsItemSourceContentText
    {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///Relative self-contained function entry path. Must not contain path traversal, null bytes or node_modules segments, and must end with *.nrz-fn.ts/js/mjs.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "description": "Relative self-contained function entry path. Must not contain path traversal, null bytes or node_modules segments, and must end with *.nrz-fn.ts/js/mjs.",
    ///  "type": "string",
    ///  "maxLength": 512,
    ///  "minLength": 1,
    ///  "pattern": "\\.nrz-fn\\.(?:ts|tsx|js|jsx|mjs)$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath>
        for ::std::string::String
    {
        fn from(value: OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 512usize {
                return Err("longer than 512 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| {
                    ::regress::Regex::new("\\.nrz-fn\\.(?:ts|tsx|js|jsx|mjs)$").unwrap()
                });
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"\\.nrz-fn\\.(?:ts|tsx|js|jsx|mjs)$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaFunctionsPublishPayloadV1FunctionsItemSourcePath {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaFunctionsPublishPayloadV1Origin`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "DEPLOYMENT",
    ///    "UI",
    ///    "CLI"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaFunctionsPublishPayloadV1Origin {
        #[serde(rename = "DEPLOYMENT")]
        Deployment,
        #[serde(rename = "UI")]
        Ui,
        #[serde(rename = "CLI")]
        Cli,
    }
    impl ::std::fmt::Display for OnrezaFunctionsPublishPayloadV1Origin {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Deployment => f.write_str("DEPLOYMENT"),
                Self::Ui => f.write_str("UI"),
                Self::Cli => f.write_str("CLI"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPublishPayloadV1Origin {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "DEPLOYMENT" => Ok(Self::Deployment),
                "UI" => Ok(Self::Ui),
                "CLI" => Ok(Self::Cli),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPublishPayloadV1Origin {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPublishPayloadV1Origin {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPublishPayloadV1Origin {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
}
pub mod onreza_functions_policy {
    /// Error types.
    pub mod error {
        /// Error from a `TryFrom` or `FromStr` implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }
        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }
        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }
    ///Outcome of the publish-time function policy scan, shared between nrz-cli preview and the platform artifact-ingest authority. Generated from the Zod source of truth.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "$id": "https://docs.onreza.ru/schemas/onreza-functions-policy-result-v1.schema.json",
    ///  "title": "ONREZA Functions Policy Result v1",
    ///  "description": "Outcome of the publish-time function policy scan, shared between nrz-cli preview and the platform artifact-ingest authority. Generated from the Zod source of truth.",
    ///  "examples": [
    ///    {
    ///      "checkedModules": 3,
    ///      "entrypoint": "functions/api.ts",
    ///      "policyVersion": "onreza-functions-policy/v1",
    ///      "status": "failed",
    ///      "violations": [
    ///        {
    ///          "capability": "denied-module-specifier",
    ///          "importer": "functions/api.ts",
    ///          "reason": "Module specifier 'net' is not allowed in ONREZA Functions",
    ///          "specifier": "net"
    ///        }
    ///      ]
    ///    }
    ///  ],
    ///  "type": "object",
    ///  "required": [
    ///    "checkedModules",
    ///    "entrypoint",
    ///    "policyVersion",
    ///    "status",
    ///    "violations"
    ///  ],
    ///  "properties": {
    ///    "checkedModules": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 0.0
    ///    },
    ///    "entrypoint": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "policyVersion": {
    ///      "type": "string",
    ///      "const": "onreza-functions-policy/v1"
    ///    },
    ///    "status": {
    ///      "type": "string",
    ///      "enum": [
    ///        "passed",
    ///        "failed"
    ///      ]
    ///    },
    ///    "violations": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/definitions/OnrezaFunctionsPolicyViolation"
    ///      }
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsPolicyResultV1 {
        #[serde(rename = "checkedModules")]
        pub checked_modules: i64,
        pub entrypoint: OnrezaFunctionsPolicyResultV1Entrypoint,
        #[serde(rename = "policyVersion")]
        pub policy_version: ::std::string::String,
        pub status: OnrezaFunctionsPolicyResultV1Status,
        pub violations: ::std::vec::Vec<OnrezaFunctionsPolicyViolation>,
    }
    ///`OnrezaFunctionsPolicyResultV1Entrypoint`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPolicyResultV1Entrypoint(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPolicyResultV1Entrypoint {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPolicyResultV1Entrypoint> for ::std::string::String {
        fn from(value: OnrezaFunctionsPolicyResultV1Entrypoint) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPolicyResultV1Entrypoint {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPolicyResultV1Entrypoint {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPolicyResultV1Entrypoint {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPolicyResultV1Entrypoint {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaFunctionsPolicyResultV1Entrypoint {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaFunctionsPolicyResultV1Status`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "passed",
    ///    "failed"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaFunctionsPolicyResultV1Status {
        #[serde(rename = "passed")]
        Passed,
        #[serde(rename = "failed")]
        Failed,
    }
    impl ::std::fmt::Display for OnrezaFunctionsPolicyResultV1Status {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Passed => f.write_str("passed"),
                Self::Failed => f.write_str("failed"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPolicyResultV1Status {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "passed" => Ok(Self::Passed),
                "failed" => Ok(Self::Failed),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPolicyResultV1Status {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPolicyResultV1Status {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPolicyResultV1Status {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaFunctionsPolicyViolation`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "capability",
    ///    "reason"
    ///  ],
    ///  "properties": {
    ///    "capability": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "importer": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "metadata": {
    ///      "type": "object",
    ///      "additionalProperties": {},
    ///      "propertyNames": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "reason": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "specifier": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsPolicyViolation {
        pub capability: OnrezaFunctionsPolicyViolationCapability,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub importer: ::std::option::Option<OnrezaFunctionsPolicyViolationImporter>,
        #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
        pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
        pub reason: OnrezaFunctionsPolicyViolationReason,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub specifier: ::std::option::Option<OnrezaFunctionsPolicyViolationSpecifier>,
    }
    ///`OnrezaFunctionsPolicyViolationCapability`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPolicyViolationCapability(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPolicyViolationCapability {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPolicyViolationCapability> for ::std::string::String {
        fn from(value: OnrezaFunctionsPolicyViolationCapability) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPolicyViolationCapability {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPolicyViolationCapability {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPolicyViolationCapability {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPolicyViolationCapability {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaFunctionsPolicyViolationCapability {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaFunctionsPolicyViolationImporter`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPolicyViolationImporter(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPolicyViolationImporter {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPolicyViolationImporter> for ::std::string::String {
        fn from(value: OnrezaFunctionsPolicyViolationImporter) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPolicyViolationImporter {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPolicyViolationImporter {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPolicyViolationImporter {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPolicyViolationImporter {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaFunctionsPolicyViolationImporter {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaFunctionsPolicyViolationReason`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPolicyViolationReason(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPolicyViolationReason {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPolicyViolationReason> for ::std::string::String {
        fn from(value: OnrezaFunctionsPolicyViolationReason) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPolicyViolationReason {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPolicyViolationReason {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPolicyViolationReason {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPolicyViolationReason {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaFunctionsPolicyViolationReason {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
    ///`OnrezaFunctionsPolicyViolationSpecifier`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaFunctionsPolicyViolationSpecifier(::std::string::String);
    impl ::std::ops::Deref for OnrezaFunctionsPolicyViolationSpecifier {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaFunctionsPolicyViolationSpecifier> for ::std::string::String {
        fn from(value: OnrezaFunctionsPolicyViolationSpecifier) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsPolicyViolationSpecifier {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsPolicyViolationSpecifier {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaFunctionsPolicyViolationSpecifier {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaFunctionsPolicyViolationSpecifier {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaFunctionsPolicyViolationSpecifier {
        fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::std::string::String::deserialize(deserializer)?
                .parse()
                .map_err(|e: self::error::ConversionError| {
                    <D::Error as ::serde::de::Error>::custom(e.to_string())
                })
        }
    }
}
pub mod onreza_functions_runtime_policy {
    /// Error types.
    pub mod error {
        /// Error from a `TryFrom` or `FromStr` implementation.
        pub struct ConversionError(::std::borrow::Cow<'static, str>);
        impl ::std::error::Error for ConversionError {}
        impl ::std::fmt::Display for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Display::fmt(&self.0, f)
            }
        }
        impl ::std::fmt::Debug for ConversionError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
                ::std::fmt::Debug::fmt(&self.0, f)
            }
        }
        impl From<&'static str> for ConversionError {
            fn from(value: &'static str) -> Self {
                Self(value.into())
            }
        }
        impl From<String> for ConversionError {
            fn from(value: String) -> Self {
                Self(value.into())
            }
        }
    }
    ///Frozen runtime sandbox policy advertised to the function runtime. Generated from the Zod source of truth.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "$id": "https://docs.onreza.ru/schemas/onreza-functions-runtime-policy-v1.schema.json",
    ///  "title": "ONREZA Functions Runtime Policy v1",
    ///  "description": "Frozen runtime sandbox policy advertised to the function runtime. Generated from the Zod source of truth.",
    ///  "type": "object",
    ///  "required": [
    ///    "dynamicImport",
    ///    "egress",
    ///    "esmOnly",
    ///    "filesystem",
    ///    "localImportExtensions",
    ///    "moduleSpecifiers",
    ///    "runtimeApis",
    ///    "version"
    ///  ],
    ///  "properties": {
    ///    "dynamicImport": {
    ///      "type": "object",
    ///      "required": [
    ///        "resolvedSpecifiersOnly"
    ///      ],
    ///      "properties": {
    ///        "resolvedSpecifiersOnly": {
    ///          "type": "boolean",
    ///          "const": true
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "egress": {
    ///      "type": "object",
    ///      "required": [
    ///        "profile"
    ///      ],
    ///      "properties": {
    ///        "profile": {
    ///          "type": "string",
    ///          "const": "public-internet-with-private-deny"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "esmOnly": {
    ///      "type": "boolean",
    ///      "const": true
    ///    },
    ///    "filesystem": {
    ///      "type": "object",
    ///      "required": [
    ///        "bundleReadOnly",
    ///        "read",
    ///        "write"
    ///      ],
    ///      "properties": {
    ///        "bundleReadOnly": {
    ///          "type": "boolean",
    ///          "const": true
    ///        },
    ///        "read": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "bundle",
    ///              "tmp"
    ///            ]
    ///          }
    ///        },
    ///        "write": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "tmp"
    ///            ]
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "localImportExtensions": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "enum": [
    ///          ".ts",
    ///          ".tsx",
    ///          ".js",
    ///          ".jsx",
    ///          ".mjs"
    ///        ]
    ///      }
    ///    },
    ///    "moduleSpecifiers": {
    ///      "type": "object",
    ///      "required": [
    ///        "allowed",
    ///        "default",
    ///        "denied"
    ///      ],
    ///      "properties": {
    ///        "allowed": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string"
    ///          }
    ///        },
    ///        "default": {
    ///          "type": "string",
    ///          "const": "deny"
    ///        },
    ///        "denied": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string"
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "runtimeApis": {
    ///      "type": "object",
    ///      "required": [
    ///        "allowedBunProperties",
    ///        "ambientEnv",
    ///        "nestedWorkers",
    ///        "parentMessageChannel",
    ///        "processControl"
    ///      ],
    ///      "properties": {
    ///        "allowedBunProperties": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string"
    ///          }
    ///        },
    ///        "ambientEnv": {
    ///          "type": "boolean",
    ///          "const": false
    ///        },
    ///        "nestedWorkers": {
    ///          "type": "boolean",
    ///          "const": false
    ///        },
    ///        "parentMessageChannel": {
    ///          "type": "boolean",
    ///          "const": false
    ///        },
    ///        "processControl": {
    ///          "type": "boolean",
    ///          "const": false
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "version": {
    ///      "type": "string",
    ///      "const": "onreza-functions-policy/v1"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsRuntimePolicyV1 {
        #[serde(rename = "dynamicImport")]
        pub dynamic_import: OnrezaFunctionsRuntimePolicyV1DynamicImport,
        pub egress: OnrezaFunctionsRuntimePolicyV1Egress,
        #[serde(rename = "esmOnly")]
        pub esm_only: bool,
        pub filesystem: OnrezaFunctionsRuntimePolicyV1Filesystem,
        #[serde(rename = "localImportExtensions")]
        pub local_import_extensions:
            ::std::vec::Vec<OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem>,
        #[serde(rename = "moduleSpecifiers")]
        pub module_specifiers: OnrezaFunctionsRuntimePolicyV1ModuleSpecifiers,
        #[serde(rename = "runtimeApis")]
        pub runtime_apis: OnrezaFunctionsRuntimePolicyV1RuntimeApis,
        pub version: ::std::string::String,
    }
    ///`OnrezaFunctionsRuntimePolicyV1DynamicImport`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "resolvedSpecifiersOnly"
    ///  ],
    ///  "properties": {
    ///    "resolvedSpecifiersOnly": {
    ///      "type": "boolean",
    ///      "const": true
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsRuntimePolicyV1DynamicImport {
        #[serde(rename = "resolvedSpecifiersOnly")]
        pub resolved_specifiers_only: bool,
    }
    ///`OnrezaFunctionsRuntimePolicyV1Egress`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "profile"
    ///  ],
    ///  "properties": {
    ///    "profile": {
    ///      "type": "string",
    ///      "const": "public-internet-with-private-deny"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsRuntimePolicyV1Egress {
        pub profile: ::std::string::String,
    }
    ///`OnrezaFunctionsRuntimePolicyV1Filesystem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "bundleReadOnly",
    ///    "read",
    ///    "write"
    ///  ],
    ///  "properties": {
    ///    "bundleReadOnly": {
    ///      "type": "boolean",
    ///      "const": true
    ///    },
    ///    "read": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "enum": [
    ///          "bundle",
    ///          "tmp"
    ///        ]
    ///      }
    ///    },
    ///    "write": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string",
    ///        "enum": [
    ///          "tmp"
    ///        ]
    ///      }
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsRuntimePolicyV1Filesystem {
        #[serde(rename = "bundleReadOnly")]
        pub bundle_read_only: bool,
        pub read: ::std::vec::Vec<OnrezaFunctionsRuntimePolicyV1FilesystemReadItem>,
        pub write: ::std::vec::Vec<OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem>,
    }
    ///`OnrezaFunctionsRuntimePolicyV1FilesystemReadItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "bundle",
    ///    "tmp"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaFunctionsRuntimePolicyV1FilesystemReadItem {
        #[serde(rename = "bundle")]
        Bundle,
        #[serde(rename = "tmp")]
        Tmp,
    }
    impl ::std::fmt::Display for OnrezaFunctionsRuntimePolicyV1FilesystemReadItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Bundle => f.write_str("bundle"),
                Self::Tmp => f.write_str("tmp"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsRuntimePolicyV1FilesystemReadItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "bundle" => Ok(Self::Bundle),
                "tmp" => Ok(Self::Tmp),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsRuntimePolicyV1FilesystemReadItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaFunctionsRuntimePolicyV1FilesystemReadItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaFunctionsRuntimePolicyV1FilesystemReadItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "tmp"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem {
        #[serde(rename = "tmp")]
        Tmp,
    }
    impl ::std::fmt::Display for OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Tmp => f.write_str("tmp"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "tmp" => Ok(Self::Tmp),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaFunctionsRuntimePolicyV1FilesystemWriteItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    ".ts",
    ///    ".tsx",
    ///    ".js",
    ///    ".jsx",
    ///    ".mjs"
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(
        ::serde::Deserialize,
        ::serde::Serialize,
        Clone,
        Copy,
        Debug,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
    )]
    pub enum OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem {
        #[serde(rename = ".ts")]
        Ts,
        #[serde(rename = ".tsx")]
        Tsx,
        #[serde(rename = ".js")]
        Js,
        #[serde(rename = ".jsx")]
        Jsx,
        #[serde(rename = ".mjs")]
        Mjs,
    }
    impl ::std::fmt::Display for OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Ts => f.write_str(".ts"),
                Self::Tsx => f.write_str(".tsx"),
                Self::Js => f.write_str(".js"),
                Self::Jsx => f.write_str(".jsx"),
                Self::Mjs => f.write_str(".mjs"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                ".ts" => Ok(Self::Ts),
                ".tsx" => Ok(Self::Tsx),
                ".js" => Ok(Self::Js),
                ".jsx" => Ok(Self::Jsx),
                ".mjs" => Ok(Self::Mjs),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaFunctionsRuntimePolicyV1LocalImportExtensionsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaFunctionsRuntimePolicyV1ModuleSpecifiers`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "allowed",
    ///    "default",
    ///    "denied"
    ///  ],
    ///  "properties": {
    ///    "allowed": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "default": {
    ///      "type": "string",
    ///      "const": "deny"
    ///    },
    ///    "denied": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsRuntimePolicyV1ModuleSpecifiers {
        pub allowed: ::std::vec::Vec<::std::string::String>,
        pub default: ::std::string::String,
        pub denied: ::std::vec::Vec<::std::string::String>,
    }
    ///`OnrezaFunctionsRuntimePolicyV1RuntimeApis`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "allowedBunProperties",
    ///    "ambientEnv",
    ///    "nestedWorkers",
    ///    "parentMessageChannel",
    ///    "processControl"
    ///  ],
    ///  "properties": {
    ///    "allowedBunProperties": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "ambientEnv": {
    ///      "type": "boolean",
    ///      "const": false
    ///    },
    ///    "nestedWorkers": {
    ///      "type": "boolean",
    ///      "const": false
    ///    },
    ///    "parentMessageChannel": {
    ///      "type": "boolean",
    ///      "const": false
    ///    },
    ///    "processControl": {
    ///      "type": "boolean",
    ///      "const": false
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaFunctionsRuntimePolicyV1RuntimeApis {
        #[serde(rename = "allowedBunProperties")]
        pub allowed_bun_properties: ::std::vec::Vec<::std::string::String>,
        #[serde(rename = "ambientEnv")]
        pub ambient_env: bool,
        #[serde(rename = "nestedWorkers")]
        pub nested_workers: bool,
        #[serde(rename = "parentMessageChannel")]
        pub parent_message_channel: bool,
        #[serde(rename = "processControl")]
        pub process_control: bool,
    }
}
