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
    ///        "ifNoFile": {
    ///          "type": "boolean"
    ///        },
    ///        "statusCode": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": -9007199254740991.0
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
    ///        "ifNoFile": {
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
    ///              "asn",
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
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "steps",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "inheritGate": {
    ///          "type": "boolean"
    ///        },
    ///        "override": {
    ///          "type": "boolean"
    ///        },
    ///        "steps": {
    ///          "type": "array",
    ///          "items": {
    ///            "anyOf": [
    ///              {
    ///                "type": "object",
    ///                "required": [
    ///                  "mode",
    ///                  "use"
    ///                ],
    ///                "properties": {
    ///                  "as": {
    ///                    "type": "string",
    ///                    "maxLength": 64,
    ///                    "minLength": 1
    ///                  },
    ///                  "cachePosition": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "before",
    ///                      "after"
    ///                    ]
    ///                  },
    ///                  "failure": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "closed",
    ///                      "open"
    ///                    ]
    ///                  },
    ///                  "mode": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "request",
    ///                      "response",
    ///                      "observe"
    ///                    ]
    ///                  },
    ///                  "use": {
    ///                    "type": "string",
    ///                    "maxLength": 64,
    ///                    "minLength": 1,
    ///                    "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///                  }
    ///                },
    ///                "additionalProperties": false
    ///              },
    ///              {
    ///                "$ref": "#/definitions/PipelineHandleStep"
    ///              }
    ///            ]
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "pipeline"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "limit",
    ///        "type",
    ///        "windowSeconds"
    ///      ],
    ///      "properties": {
    ///        "key": {
    ///          "type": "string",
    ///          "enum": [
    ///            "ip",
    ///            "ip_path",
    ///            "ip_host",
    ///            "host"
    ///          ]
    ///        },
    ///        "limit": {
    ///          "type": "integer",
    ///          "maximum": 100000.0,
    ///          "minimum": 1.0
    ///        },
    ///        "mode": {
    ///          "type": "string",
    ///          "enum": [
    ///            "shadow",
    ///            "enforce"
    ///          ]
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "rate_limit"
    ///        },
    ///        "windowSeconds": {
    ///          "type": "integer",
    ///          "maximum": 600.0,
    ///          "minimum": 10.0
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
            #[serde(
                rename = "ifNoFile",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            if_no_file: ::std::option::Option<bool>,
            #[serde(
                rename = "statusCode",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            status_code: ::std::option::Option<i64>,
            target: EdgeRuleActionAuthoringTarget,
        },
        #[serde(rename = "rewrite")]
        Rewrite {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            external: ::std::option::Option<bool>,
            #[serde(
                rename = "ifNoFile",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            if_no_file: ::std::option::Option<bool>,
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
        #[serde(rename = "pipeline")]
        Pipeline {
            #[serde(
                rename = "inheritGate",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            inherit_gate: ::std::option::Option<bool>,
            #[serde(
                rename = "override",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            override_: ::std::option::Option<bool>,
            steps: ::std::vec::Vec<EdgeRuleActionAuthoringStepsItem>,
        },
        #[serde(rename = "rate_limit")]
        RateLimit {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            key: ::std::option::Option<EdgeRuleActionAuthoringKey>,
            limit: ::std::num::NonZeroU64,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            mode: ::std::option::Option<EdgeRuleActionAuthoringMode>,
            #[serde(rename = "windowSeconds")]
            window_seconds: i64,
        },
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
    ///`EdgeRuleActionAuthoringKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ip",
    ///    "ip_path",
    ///    "ip_host",
    ///    "host"
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
    pub enum EdgeRuleActionAuthoringKey {
        #[serde(rename = "ip")]
        Ip,
        #[serde(rename = "ip_path")]
        IpPath,
        #[serde(rename = "ip_host")]
        IpHost,
        #[serde(rename = "host")]
        Host,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringKey {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Ip => f.write_str("ip"),
                Self::IpPath => f.write_str("ip_path"),
                Self::IpHost => f.write_str("ip_host"),
                Self::Host => f.write_str("host"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "ip" => Ok(Self::Ip),
                "ip_path" => Ok(Self::IpPath),
                "ip_host" => Ok(Self::IpHost),
                "host" => Ok(Self::Host),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
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
    ///`EdgeRuleActionAuthoringStepsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "anyOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "mode",
    ///        "use"
    ///      ],
    ///      "properties": {
    ///        "as": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1
    ///        },
    ///        "cachePosition": {
    ///          "type": "string",
    ///          "enum": [
    ///            "before",
    ///            "after"
    ///          ]
    ///        },
    ///        "failure": {
    ///          "type": "string",
    ///          "enum": [
    ///            "closed",
    ///            "open"
    ///          ]
    ///        },
    ///        "mode": {
    ///          "type": "string",
    ///          "enum": [
    ///            "request",
    ///            "response",
    ///            "observe"
    ///          ]
    ///        },
    ///        "use": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1,
    ///          "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "$ref": "#/definitions/PipelineHandleStep"
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(untagged, deny_unknown_fields)]
    pub enum EdgeRuleActionAuthoringStepsItem {
        Object {
            #[serde(
                rename = "as",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            as_: ::std::option::Option<EdgeRuleActionAuthoringStepsItemObjectAs>,
            #[serde(
                rename = "cachePosition",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            cache_position:
                ::std::option::Option<EdgeRuleActionAuthoringStepsItemObjectCachePosition>,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            failure: ::std::option::Option<EdgeRuleActionAuthoringStepsItemObjectFailure>,
            mode: EdgeRuleActionAuthoringStepsItemObjectMode,
            #[serde(rename = "use")]
            use_: EdgeRuleActionAuthoringStepsItemObjectUse,
        },
        PipelineHandleStep(PipelineHandleStep),
    }
    impl ::std::convert::From<PipelineHandleStep> for EdgeRuleActionAuthoringStepsItem {
        fn from(value: PipelineHandleStep) -> Self {
            Self::PipelineHandleStep(value)
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectAs`
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
    pub struct EdgeRuleActionAuthoringStepsItemObjectAs(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringStepsItemObjectAs> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringStepsItemObjectAs) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectAs {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringStepsItemObjectAs {
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
    ///`EdgeRuleActionAuthoringStepsItemObjectCachePosition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "before",
    ///    "after"
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
    pub enum EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        #[serde(rename = "before")]
        Before,
        #[serde(rename = "after")]
        After,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Before => f.write_str("before"),
                Self::After => f.write_str("after"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "before" => Ok(Self::Before),
                "after" => Ok(Self::After),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectCachePosition
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectCachePosition
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectFailure`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "closed",
    ///    "open"
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
    pub enum EdgeRuleActionAuthoringStepsItemObjectFailure {
        #[serde(rename = "closed")]
        Closed,
        #[serde(rename = "open")]
        Open,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStepsItemObjectFailure {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Closed => f.write_str("closed"),
                Self::Open => f.write_str("open"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectFailure {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "closed" => Ok(Self::Closed),
                "open" => Ok(Self::Open),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectFailure {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectFailure
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectFailure
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectMode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "request",
    ///    "response",
    ///    "observe"
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
    pub enum EdgeRuleActionAuthoringStepsItemObjectMode {
        #[serde(rename = "request")]
        Request,
        #[serde(rename = "response")]
        Response,
        #[serde(rename = "observe")]
        Observe,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStepsItemObjectMode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Request => f.write_str("request"),
                Self::Response => f.write_str("response"),
                Self::Observe => f.write_str("observe"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectMode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "request" => Ok(Self::Request),
                "response" => Ok(Self::Response),
                "observe" => Ok(Self::Observe),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectMode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectMode
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectUse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 64,
    ///  "minLength": 1,
    ///  "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringStepsItemObjectUse(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringStepsItemObjectUse> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringStepsItemObjectUse) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 64usize {
                return Err("longer than 64 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| {
                    ::regress::Regex::new("^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$").unwrap()
                });
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringStepsItemObjectUse {
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
    ///    "asn",
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
        #[serde(rename = "asn")]
        Asn,
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
                Self::Asn => f.write_str("asn"),
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
                "asn" => Ok(Self::Asn),
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
    ///      "type": "object",
    ///      "properties": {
    ///        "any": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "object",
    ///            "properties": {
    ///              "asn": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "integer",
    ///                  "maximum": 4294967295.0,
    ///                  "minimum": 1.0
    ///                }
    ///              },
    ///              "cookies": {
    ///                "type": "object",
    ///                "additionalProperties": {
    ///                  "type": "string"
    ///                },
    ///                "propertyNames": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "device": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "desktop",
    ///                  "mobile",
    ///                  "tablet",
    ///                  "bot"
    ///                ]
    ///              },
    ///              "geo": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "maxLength": 2,
    ///                  "minLength": 2
    ///                }
    ///              },
    ///              "headers": {
    ///                "type": "object",
    ///                "additionalProperties": {
    ///                  "type": "string"
    ///                },
    ///                "propertyNames": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "host": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              },
    ///              "method": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "enum": [
    ///                    "GET",
    ///                    "POST",
    ///                    "PUT",
    ///                    "DELETE",
    ///                    "PATCH",
    ///                    "HEAD",
    ///                    "OPTIONS"
    ///                  ]
    ///                }
    ///              },
    ///              "methods": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "enum": [
    ///                    "GET",
    ///                    "POST",
    ///                    "PUT",
    ///                    "DELETE",
    ///                    "PATCH",
    ///                    "HEAD",
    ///                    "OPTIONS"
    ///                  ]
    ///                }
    ///              },
    ///              "path": {
    ///                "type": "object",
    ///                "required": [
    ///                  "type",
    ///                  "value"
    ///                ],
    ///                "properties": {
    ///                  "type": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "exact",
    ///                      "prefix",
    ///                      "glob"
    ///                    ]
    ///                  },
    ///                  "value": {
    ///                    "type": "string",
    ///                    "minLength": 1
    ///                  }
    ///                },
    ///                "additionalProperties": false
    ///              },
    ///              "query": {
    ///                "type": "object",
    ///                "additionalProperties": {
    ///                  "type": "string"
    ///                },
    ///                "propertyNames": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "sourceIpCidrs": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "asn": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "integer",
    ///            "maximum": 4294967295.0,
    ///            "minimum": 1.0
    ///          }
    ///        },
    ///        "cookies": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "device": {
    ///          "type": "string",
    ///          "enum": [
    ///            "desktop",
    ///            "mobile",
    ///            "tablet",
    ///            "bot"
    ///          ]
    ///        },
    ///        "geo": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "maxLength": 2,
    ///            "minLength": 2
    ///          }
    ///        },
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
    ///        "host": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "method": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "methods": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "not": {
    ///          "type": "object",
    ///          "properties": {
    ///            "asn": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "integer",
    ///                "maximum": 4294967295.0,
    ///                "minimum": 1.0
    ///              }
    ///            },
    ///            "cookies": {
    ///              "type": "object",
    ///              "additionalProperties": {
    ///                "type": "string"
    ///              },
    ///              "propertyNames": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "device": {
    ///              "type": "string",
    ///              "enum": [
    ///                "desktop",
    ///                "mobile",
    ///                "tablet",
    ///                "bot"
    ///              ]
    ///            },
    ///            "geo": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "maxLength": 2,
    ///                "minLength": 2
    ///              }
    ///            },
    ///            "headers": {
    ///              "type": "object",
    ///              "additionalProperties": {
    ///                "type": "string"
    ///              },
    ///              "propertyNames": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "host": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            },
    ///            "method": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "GET",
    ///                  "POST",
    ///                  "PUT",
    ///                  "DELETE",
    ///                  "PATCH",
    ///                  "HEAD",
    ///                  "OPTIONS"
    ///                ]
    ///              }
    ///            },
    ///            "methods": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "GET",
    ///                  "POST",
    ///                  "PUT",
    ///                  "DELETE",
    ///                  "PATCH",
    ///                  "HEAD",
    ///                  "OPTIONS"
    ///                ]
    ///              }
    ///            },
    ///            "path": {
    ///              "type": "object",
    ///              "required": [
    ///                "type",
    ///                "value"
    ///              ],
    ///              "properties": {
    ///                "type": {
    ///                  "type": "string",
    ///                  "enum": [
    ///                    "exact",
    ///                    "prefix",
    ///                    "glob"
    ///                  ]
    ///                },
    ///                "value": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "additionalProperties": false
    ///            },
    ///            "query": {
    ///              "type": "object",
    ///              "additionalProperties": {
    ///                "type": "string"
    ///              },
    ///              "propertyNames": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "sourceIpCidrs": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "path": {
    ///          "type": "object",
    ///          "required": [
    ///            "type",
    ///            "value"
    ///          ],
    ///          "properties": {
    ///            "type": {
    ///              "type": "string",
    ///              "enum": [
    ///                "exact",
    ///                "prefix",
    ///                "glob"
    ///              ]
    ///            },
    ///            "value": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "query": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "sourceIpCidrs": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
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
        pub condition: ::std::option::Option<EdgeRuleAuthoringCondition>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub enabled: ::std::option::Option<bool>,
        pub id: EdgeRuleAuthoringId,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub name: ::std::option::Option<EdgeRuleAuthoringName>,
    }
    ///`EdgeRuleAuthoringCondition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "any": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "properties": {
    ///          "asn": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "integer",
    ///              "maximum": 4294967295.0,
    ///              "minimum": 1.0
    ///            }
    ///          },
    ///          "cookies": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "device": {
    ///            "type": "string",
    ///            "enum": [
    ///              "desktop",
    ///              "mobile",
    ///              "tablet",
    ///              "bot"
    ///            ]
    ///          },
    ///          "geo": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "maxLength": 2,
    ///              "minLength": 2
    ///            }
    ///          },
    ///          "headers": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "host": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          },
    ///          "method": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "enum": [
    ///                "GET",
    ///                "POST",
    ///                "PUT",
    ///                "DELETE",
    ///                "PATCH",
    ///                "HEAD",
    ///                "OPTIONS"
    ///              ]
    ///            }
    ///          },
    ///          "methods": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "enum": [
    ///                "GET",
    ///                "POST",
    ///                "PUT",
    ///                "DELETE",
    ///                "PATCH",
    ///                "HEAD",
    ///                "OPTIONS"
    ///              ]
    ///            }
    ///          },
    ///          "path": {
    ///            "type": "object",
    ///            "required": [
    ///              "type",
    ///              "value"
    ///            ],
    ///            "properties": {
    ///              "type": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "exact",
    ///                  "prefix",
    ///                  "glob"
    ///                ]
    ///              },
    ///              "value": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "query": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "sourceIpCidrs": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          }
    ///        },
    ///        "additionalProperties": false
    ///      },
    ///      "minItems": 1
    ///    },
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "method": {
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
    ///    "not": {
    ///      "type": "object",
    ///      "properties": {
    ///        "asn": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "integer",
    ///            "maximum": 4294967295.0,
    ///            "minimum": 1.0
    ///          }
    ///        },
    ///        "cookies": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "device": {
    ///          "type": "string",
    ///          "enum": [
    ///            "desktop",
    ///            "mobile",
    ///            "tablet",
    ///            "bot"
    ///          ]
    ///        },
    ///        "geo": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "maxLength": 2,
    ///            "minLength": 2
    ///          }
    ///        },
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
    ///        "host": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "method": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "methods": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "path": {
    ///          "type": "object",
    ///          "required": [
    ///            "type",
    ///            "value"
    ///          ],
    ///          "properties": {
    ///            "type": {
    ///              "type": "string",
    ///              "enum": [
    ///                "exact",
    ///                "prefix",
    ///                "glob"
    ///              ]
    ///            },
    ///            "value": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "query": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "sourceIpCidrs": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
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
    ///            "glob"
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
    pub struct EdgeRuleAuthoringCondition {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub any: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionCookiesKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleAuthoringConditionDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleAuthoringConditionGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionHeadersKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleAuthoringConditionHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub method: ::std::vec::Vec<EdgeRuleAuthoringConditionMethodItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleAuthoringConditionMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub not: ::std::option::Option<EdgeRuleAuthoringConditionNot>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleAuthoringConditionPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query:
            ::std::collections::HashMap<EdgeRuleAuthoringConditionQueryKey, ::std::string::String>,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleAuthoringConditionSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleAuthoringCondition {
        fn default() -> Self {
            Self {
                any: Default::default(),
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                method: Default::default(),
                methods: Default::default(),
                not: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "method": {
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
    ///            "glob"
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
    pub struct EdgeRuleAuthoringConditionAnyItem {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionAnyItemCookiesKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleAuthoringConditionAnyItemDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionAnyItemHeadersKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleAuthoringConditionAnyItemHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub method: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemMethodItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleAuthoringConditionAnyItemPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionAnyItemQueryKey,
            ::std::string::String,
        >,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleAuthoringConditionAnyItem {
        fn default() -> Self {
            Self {
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                method: Default::default(),
                methods: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemCookiesKey`
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
    pub struct EdgeRuleAuthoringConditionAnyItemCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemCookiesKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemCookiesKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemCookiesKey {
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
    ///`EdgeRuleAuthoringConditionAnyItemDevice`
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
    pub enum EdgeRuleAuthoringConditionAnyItemDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemGeoItem`
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
    pub struct EdgeRuleAuthoringConditionAnyItemGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemGeoItem {
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
    ///`EdgeRuleAuthoringConditionAnyItemHeadersKey`
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
    pub struct EdgeRuleAuthoringConditionAnyItemHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemHeadersKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemHeadersKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemHeadersKey {
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
    ///`EdgeRuleAuthoringConditionAnyItemHost`
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
    pub struct EdgeRuleAuthoringConditionAnyItemHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemHost> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemHost {
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
    ///`EdgeRuleAuthoringConditionAnyItemMethodItem`
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
    pub enum EdgeRuleAuthoringConditionAnyItemMethodItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemMethodItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemMethodItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemMethodsItem`
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
    pub enum EdgeRuleAuthoringConditionAnyItemMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemPath`
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
    ///        "glob"
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
    pub struct EdgeRuleAuthoringConditionAnyItemPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleAuthoringConditionAnyItemPathType,
        pub value: EdgeRuleAuthoringConditionAnyItemPathValue,
    }
    ///`EdgeRuleAuthoringConditionAnyItemPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleAuthoringConditionAnyItemPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemPathValue`
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
    pub struct EdgeRuleAuthoringConditionAnyItemPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemPathValue> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemPathValue
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemPathValue {
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
    ///`EdgeRuleAuthoringConditionAnyItemQueryKey`
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
    pub struct EdgeRuleAuthoringConditionAnyItemQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemQueryKey {
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
    ///`EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem`
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
    pub struct EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem>
        for ::std::string::String
    {
        fn from(value: EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
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
    ///`EdgeRuleAuthoringConditionCookiesKey`
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
    pub struct EdgeRuleAuthoringConditionCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionCookiesKey {
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
    ///`EdgeRuleAuthoringConditionDevice`
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
    pub enum EdgeRuleAuthoringConditionDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionGeoItem`
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
    pub struct EdgeRuleAuthoringConditionGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionGeoItem {
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
    ///`EdgeRuleAuthoringConditionHeadersKey`
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
    pub struct EdgeRuleAuthoringConditionHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionHeadersKey {
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
    ///`EdgeRuleAuthoringConditionHost`
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
    pub struct EdgeRuleAuthoringConditionHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionHost> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionHost {
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
    ///`EdgeRuleAuthoringConditionMethodItem`
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
    pub enum EdgeRuleAuthoringConditionMethodItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionMethodItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionMethodItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionMethodsItem`
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
    pub enum EdgeRuleAuthoringConditionMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNot`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "method": {
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
    ///            "glob"
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
    pub struct EdgeRuleAuthoringConditionNot {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionNotCookiesKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleAuthoringConditionNotDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleAuthoringConditionNotGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionNotHeadersKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleAuthoringConditionNotHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub method: ::std::vec::Vec<EdgeRuleAuthoringConditionNotMethodItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleAuthoringConditionNotMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleAuthoringConditionNotPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionNotQueryKey,
            ::std::string::String,
        >,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleAuthoringConditionNotSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleAuthoringConditionNot {
        fn default() -> Self {
            Self {
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                method: Default::default(),
                methods: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleAuthoringConditionNotCookiesKey`
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
    pub struct EdgeRuleAuthoringConditionNotCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotCookiesKey {
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
    ///`EdgeRuleAuthoringConditionNotDevice`
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
    pub enum EdgeRuleAuthoringConditionNotDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotGeoItem`
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
    pub struct EdgeRuleAuthoringConditionNotGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotGeoItem {
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
    ///`EdgeRuleAuthoringConditionNotHeadersKey`
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
    pub struct EdgeRuleAuthoringConditionNotHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotHeadersKey {
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
    ///`EdgeRuleAuthoringConditionNotHost`
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
    pub struct EdgeRuleAuthoringConditionNotHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotHost> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotHost {
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
    ///`EdgeRuleAuthoringConditionNotMethodItem`
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
    pub enum EdgeRuleAuthoringConditionNotMethodItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotMethodItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotMethodItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotMethodsItem`
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
    pub enum EdgeRuleAuthoringConditionNotMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotPath`
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
    ///        "glob"
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
    pub struct EdgeRuleAuthoringConditionNotPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleAuthoringConditionNotPathType,
        pub value: EdgeRuleAuthoringConditionNotPathValue,
    }
    ///`EdgeRuleAuthoringConditionNotPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleAuthoringConditionNotPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotPathValue`
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
    pub struct EdgeRuleAuthoringConditionNotPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotPathValue> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotPathValue {
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
    ///`EdgeRuleAuthoringConditionNotQueryKey`
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
    pub struct EdgeRuleAuthoringConditionNotQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotQueryKey {
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
    ///`EdgeRuleAuthoringConditionNotSourceIpCidrsItem`
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
    pub struct EdgeRuleAuthoringConditionNotSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotSourceIpCidrsItem>
        for ::std::string::String
    {
        fn from(value: EdgeRuleAuthoringConditionNotSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionNotSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionNotSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
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
    ///`EdgeRuleAuthoringConditionPath`
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
    ///        "glob"
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
    pub struct EdgeRuleAuthoringConditionPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleAuthoringConditionPathType,
        pub value: EdgeRuleAuthoringConditionPathValue,
    }
    ///`EdgeRuleAuthoringConditionPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleAuthoringConditionPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionPathValue`
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
    pub struct EdgeRuleAuthoringConditionPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionPathValue> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionPathValue {
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
    ///`EdgeRuleAuthoringConditionQueryKey`
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
    pub struct EdgeRuleAuthoringConditionQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionQueryKey {
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
    ///`EdgeRuleAuthoringConditionSourceIpCidrsItem`
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
    pub struct EdgeRuleAuthoringConditionSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionSourceIpCidrsItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionSourceIpCidrsItem {
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
    ///            "ifNoFile": false,
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
    ///        },
    ///        {
    ///          "action": {
    ///            "steps": [
    ///              {
    ///                "mode": "request",
    ///                "use": "require-session"
    ///              },
    ///              {
    ///                "handle": "@app"
    ///              }
    ///            ],
    ///            "type": "pipeline"
    ///          },
    ///          "condition": {
    ///            "path": {
    ///              "type": "prefix",
    ///              "value": "/dashboard"
    ///            }
    ///          },
    ///          "id": "dashboard-auth"
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
    ///    "cache rule must Vary by request-dependent condition dimensions",
    ///    "pipeline actions must declare exactly one terminal handle step",
    ///    "pipeline response/observe steps must appear after the terminal handle",
    ///    "a narrower pipeline rule must re-declare a broader failure=closed request gate or set inherit_gate=false"
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
    ///`PipelineHandleStep`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "handle"
    ///  ],
    ///  "properties": {
    ///    "handle": {
    ///      "type": "string",
    ///      "maxLength": 64,
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct PipelineHandleStep {
        pub handle: PipelineHandleStepHandle,
    }
    ///`PipelineHandleStepHandle`
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
    pub struct PipelineHandleStepHandle(::std::string::String);
    impl ::std::ops::Deref for PipelineHandleStepHandle {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<PipelineHandleStepHandle> for ::std::string::String {
        fn from(value: PipelineHandleStepHandle) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for PipelineHandleStepHandle {
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
    impl ::std::convert::TryFrom<&str> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for PipelineHandleStepHandle {
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
    ///        "ifNoFile": {
    ///          "type": "boolean"
    ///        },
    ///        "statusCode": {
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": -9007199254740991.0
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
    ///        "ifNoFile": {
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
    ///              "asn",
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
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "steps",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "inheritGate": {
    ///          "type": "boolean"
    ///        },
    ///        "override": {
    ///          "type": "boolean"
    ///        },
    ///        "steps": {
    ///          "type": "array",
    ///          "items": {
    ///            "anyOf": [
    ///              {
    ///                "type": "object",
    ///                "required": [
    ///                  "mode",
    ///                  "use"
    ///                ],
    ///                "properties": {
    ///                  "as": {
    ///                    "type": "string",
    ///                    "maxLength": 64,
    ///                    "minLength": 1
    ///                  },
    ///                  "cachePosition": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "before",
    ///                      "after"
    ///                    ]
    ///                  },
    ///                  "failure": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "closed",
    ///                      "open"
    ///                    ]
    ///                  },
    ///                  "mode": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "request",
    ///                      "response",
    ///                      "observe"
    ///                    ]
    ///                  },
    ///                  "use": {
    ///                    "type": "string",
    ///                    "maxLength": 64,
    ///                    "minLength": 1,
    ///                    "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///                  }
    ///                },
    ///                "additionalProperties": false
    ///              },
    ///              {
    ///                "$ref": "#/definitions/PipelineHandleStep"
    ///              }
    ///            ]
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "pipeline"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "limit",
    ///        "type",
    ///        "windowSeconds"
    ///      ],
    ///      "properties": {
    ///        "key": {
    ///          "type": "string",
    ///          "enum": [
    ///            "ip",
    ///            "ip_path",
    ///            "ip_host",
    ///            "host"
    ///          ]
    ///        },
    ///        "limit": {
    ///          "type": "integer",
    ///          "maximum": 100000.0,
    ///          "minimum": 1.0
    ///        },
    ///        "mode": {
    ///          "type": "string",
    ///          "enum": [
    ///            "shadow",
    ///            "enforce"
    ///          ]
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "rate_limit"
    ///        },
    ///        "windowSeconds": {
    ///          "type": "integer",
    ///          "maximum": 600.0,
    ///          "minimum": 10.0
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
            #[serde(
                rename = "ifNoFile",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            if_no_file: ::std::option::Option<bool>,
            #[serde(
                rename = "statusCode",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            status_code: ::std::option::Option<i64>,
            target: EdgeRuleActionAuthoringTarget,
        },
        #[serde(rename = "rewrite")]
        Rewrite {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            external: ::std::option::Option<bool>,
            #[serde(
                rename = "ifNoFile",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            if_no_file: ::std::option::Option<bool>,
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
        #[serde(rename = "pipeline")]
        Pipeline {
            #[serde(
                rename = "inheritGate",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            inherit_gate: ::std::option::Option<bool>,
            #[serde(
                rename = "override",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            override_: ::std::option::Option<bool>,
            steps: ::std::vec::Vec<EdgeRuleActionAuthoringStepsItem>,
        },
        #[serde(rename = "rate_limit")]
        RateLimit {
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            key: ::std::option::Option<EdgeRuleActionAuthoringKey>,
            limit: ::std::num::NonZeroU64,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            mode: ::std::option::Option<EdgeRuleActionAuthoringMode>,
            #[serde(rename = "windowSeconds")]
            window_seconds: i64,
        },
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
    ///`EdgeRuleActionAuthoringKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "ip",
    ///    "ip_path",
    ///    "ip_host",
    ///    "host"
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
    pub enum EdgeRuleActionAuthoringKey {
        #[serde(rename = "ip")]
        Ip,
        #[serde(rename = "ip_path")]
        IpPath,
        #[serde(rename = "ip_host")]
        IpHost,
        #[serde(rename = "host")]
        Host,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringKey {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Ip => f.write_str("ip"),
                Self::IpPath => f.write_str("ip_path"),
                Self::IpHost => f.write_str("ip_host"),
                Self::Host => f.write_str("host"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "ip" => Ok(Self::Ip),
                "ip_path" => Ok(Self::IpPath),
                "ip_host" => Ok(Self::IpHost),
                "host" => Ok(Self::Host),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
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
    ///`EdgeRuleActionAuthoringStepsItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "anyOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "mode",
    ///        "use"
    ///      ],
    ///      "properties": {
    ///        "as": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1
    ///        },
    ///        "cachePosition": {
    ///          "type": "string",
    ///          "enum": [
    ///            "before",
    ///            "after"
    ///          ]
    ///        },
    ///        "failure": {
    ///          "type": "string",
    ///          "enum": [
    ///            "closed",
    ///            "open"
    ///          ]
    ///        },
    ///        "mode": {
    ///          "type": "string",
    ///          "enum": [
    ///            "request",
    ///            "response",
    ///            "observe"
    ///          ]
    ///        },
    ///        "use": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1,
    ///          "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "$ref": "#/definitions/PipelineHandleStep"
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(untagged, deny_unknown_fields)]
    pub enum EdgeRuleActionAuthoringStepsItem {
        Object {
            #[serde(
                rename = "as",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            as_: ::std::option::Option<EdgeRuleActionAuthoringStepsItemObjectAs>,
            #[serde(
                rename = "cachePosition",
                default,
                skip_serializing_if = "::std::option::Option::is_none"
            )]
            cache_position:
                ::std::option::Option<EdgeRuleActionAuthoringStepsItemObjectCachePosition>,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            failure: ::std::option::Option<EdgeRuleActionAuthoringStepsItemObjectFailure>,
            mode: EdgeRuleActionAuthoringStepsItemObjectMode,
            #[serde(rename = "use")]
            use_: EdgeRuleActionAuthoringStepsItemObjectUse,
        },
        PipelineHandleStep(PipelineHandleStep),
    }
    impl ::std::convert::From<PipelineHandleStep> for EdgeRuleActionAuthoringStepsItem {
        fn from(value: PipelineHandleStep) -> Self {
            Self::PipelineHandleStep(value)
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectAs`
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
    pub struct EdgeRuleActionAuthoringStepsItemObjectAs(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringStepsItemObjectAs> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringStepsItemObjectAs) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectAs {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectAs {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringStepsItemObjectAs {
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
    ///`EdgeRuleActionAuthoringStepsItemObjectCachePosition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "before",
    ///    "after"
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
    pub enum EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        #[serde(rename = "before")]
        Before,
        #[serde(rename = "after")]
        After,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Before => f.write_str("before"),
                Self::After => f.write_str("after"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "before" => Ok(Self::Before),
                "after" => Ok(Self::After),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectCachePosition {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectCachePosition
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectCachePosition
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectFailure`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "closed",
    ///    "open"
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
    pub enum EdgeRuleActionAuthoringStepsItemObjectFailure {
        #[serde(rename = "closed")]
        Closed,
        #[serde(rename = "open")]
        Open,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStepsItemObjectFailure {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Closed => f.write_str("closed"),
                Self::Open => f.write_str("open"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectFailure {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "closed" => Ok(Self::Closed),
                "open" => Ok(Self::Open),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectFailure {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectFailure
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectFailure
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectMode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "request",
    ///    "response",
    ///    "observe"
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
    pub enum EdgeRuleActionAuthoringStepsItemObjectMode {
        #[serde(rename = "request")]
        Request,
        #[serde(rename = "response")]
        Response,
        #[serde(rename = "observe")]
        Observe,
    }
    impl ::std::fmt::Display for EdgeRuleActionAuthoringStepsItemObjectMode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Request => f.write_str("request"),
                Self::Response => f.write_str("response"),
                Self::Observe => f.write_str("observe"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectMode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "request" => Ok(Self::Request),
                "response" => Ok(Self::Response),
                "observe" => Ok(Self::Observe),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectMode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleActionAuthoringStepsItemObjectMode
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleActionAuthoringStepsItemObjectUse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 64,
    ///  "minLength": 1,
    ///  "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct EdgeRuleActionAuthoringStepsItemObjectUse(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionAuthoringStepsItemObjectUse> for ::std::string::String {
        fn from(value: EdgeRuleActionAuthoringStepsItemObjectUse) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 64usize {
                return Err("longer than 64 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| {
                    ::regress::Regex::new("^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$").unwrap()
                });
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionAuthoringStepsItemObjectUse {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionAuthoringStepsItemObjectUse {
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
    ///    "asn",
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
        #[serde(rename = "asn")]
        Asn,
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
                Self::Asn => f.write_str("asn"),
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
                "asn" => Ok(Self::Asn),
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
    ///      "type": "object",
    ///      "properties": {
    ///        "any": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "object",
    ///            "properties": {
    ///              "asn": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "integer",
    ///                  "maximum": 4294967295.0,
    ///                  "minimum": 1.0
    ///                }
    ///              },
    ///              "cookies": {
    ///                "type": "object",
    ///                "additionalProperties": {
    ///                  "type": "string"
    ///                },
    ///                "propertyNames": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "device": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "desktop",
    ///                  "mobile",
    ///                  "tablet",
    ///                  "bot"
    ///                ]
    ///              },
    ///              "geo": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "maxLength": 2,
    ///                  "minLength": 2
    ///                }
    ///              },
    ///              "headers": {
    ///                "type": "object",
    ///                "additionalProperties": {
    ///                  "type": "string"
    ///                },
    ///                "propertyNames": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "host": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              },
    ///              "method": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "enum": [
    ///                    "GET",
    ///                    "POST",
    ///                    "PUT",
    ///                    "DELETE",
    ///                    "PATCH",
    ///                    "HEAD",
    ///                    "OPTIONS"
    ///                  ]
    ///                }
    ///              },
    ///              "methods": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "enum": [
    ///                    "GET",
    ///                    "POST",
    ///                    "PUT",
    ///                    "DELETE",
    ///                    "PATCH",
    ///                    "HEAD",
    ///                    "OPTIONS"
    ///                  ]
    ///                }
    ///              },
    ///              "path": {
    ///                "type": "object",
    ///                "required": [
    ///                  "type",
    ///                  "value"
    ///                ],
    ///                "properties": {
    ///                  "type": {
    ///                    "type": "string",
    ///                    "enum": [
    ///                      "exact",
    ///                      "prefix",
    ///                      "glob"
    ///                    ]
    ///                  },
    ///                  "value": {
    ///                    "type": "string",
    ///                    "minLength": 1
    ///                  }
    ///                },
    ///                "additionalProperties": false
    ///              },
    ///              "query": {
    ///                "type": "object",
    ///                "additionalProperties": {
    ///                  "type": "string"
    ///                },
    ///                "propertyNames": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "sourceIpCidrs": {
    ///                "type": "array",
    ///                "items": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "asn": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "integer",
    ///            "maximum": 4294967295.0,
    ///            "minimum": 1.0
    ///          }
    ///        },
    ///        "cookies": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "device": {
    ///          "type": "string",
    ///          "enum": [
    ///            "desktop",
    ///            "mobile",
    ///            "tablet",
    ///            "bot"
    ///          ]
    ///        },
    ///        "geo": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "maxLength": 2,
    ///            "minLength": 2
    ///          }
    ///        },
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
    ///        "host": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "method": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "methods": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "not": {
    ///          "type": "object",
    ///          "properties": {
    ///            "asn": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "integer",
    ///                "maximum": 4294967295.0,
    ///                "minimum": 1.0
    ///              }
    ///            },
    ///            "cookies": {
    ///              "type": "object",
    ///              "additionalProperties": {
    ///                "type": "string"
    ///              },
    ///              "propertyNames": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "device": {
    ///              "type": "string",
    ///              "enum": [
    ///                "desktop",
    ///                "mobile",
    ///                "tablet",
    ///                "bot"
    ///              ]
    ///            },
    ///            "geo": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "maxLength": 2,
    ///                "minLength": 2
    ///              }
    ///            },
    ///            "headers": {
    ///              "type": "object",
    ///              "additionalProperties": {
    ///                "type": "string"
    ///              },
    ///              "propertyNames": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "host": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            },
    ///            "method": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "GET",
    ///                  "POST",
    ///                  "PUT",
    ///                  "DELETE",
    ///                  "PATCH",
    ///                  "HEAD",
    ///                  "OPTIONS"
    ///                ]
    ///              }
    ///            },
    ///            "methods": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "GET",
    ///                  "POST",
    ///                  "PUT",
    ///                  "DELETE",
    ///                  "PATCH",
    ///                  "HEAD",
    ///                  "OPTIONS"
    ///                ]
    ///              }
    ///            },
    ///            "path": {
    ///              "type": "object",
    ///              "required": [
    ///                "type",
    ///                "value"
    ///              ],
    ///              "properties": {
    ///                "type": {
    ///                  "type": "string",
    ///                  "enum": [
    ///                    "exact",
    ///                    "prefix",
    ///                    "glob"
    ///                  ]
    ///                },
    ///                "value": {
    ///                  "type": "string",
    ///                  "minLength": 1
    ///                }
    ///              },
    ///              "additionalProperties": false
    ///            },
    ///            "query": {
    ///              "type": "object",
    ///              "additionalProperties": {
    ///                "type": "string"
    ///              },
    ///              "propertyNames": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "sourceIpCidrs": {
    ///              "type": "array",
    ///              "items": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "path": {
    ///          "type": "object",
    ///          "required": [
    ///            "type",
    ///            "value"
    ///          ],
    ///          "properties": {
    ///            "type": {
    ///              "type": "string",
    ///              "enum": [
    ///                "exact",
    ///                "prefix",
    ///                "glob"
    ///              ]
    ///            },
    ///            "value": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "query": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "sourceIpCidrs": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
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
        pub condition: ::std::option::Option<EdgeRuleAuthoringCondition>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub enabled: ::std::option::Option<bool>,
        pub id: EdgeRuleAuthoringId,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub name: ::std::option::Option<EdgeRuleAuthoringName>,
    }
    ///`EdgeRuleAuthoringCondition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "any": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "properties": {
    ///          "asn": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "integer",
    ///              "maximum": 4294967295.0,
    ///              "minimum": 1.0
    ///            }
    ///          },
    ///          "cookies": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "device": {
    ///            "type": "string",
    ///            "enum": [
    ///              "desktop",
    ///              "mobile",
    ///              "tablet",
    ///              "bot"
    ///            ]
    ///          },
    ///          "geo": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "maxLength": 2,
    ///              "minLength": 2
    ///            }
    ///          },
    ///          "headers": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "host": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          },
    ///          "method": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "enum": [
    ///                "GET",
    ///                "POST",
    ///                "PUT",
    ///                "DELETE",
    ///                "PATCH",
    ///                "HEAD",
    ///                "OPTIONS"
    ///              ]
    ///            }
    ///          },
    ///          "methods": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "enum": [
    ///                "GET",
    ///                "POST",
    ///                "PUT",
    ///                "DELETE",
    ///                "PATCH",
    ///                "HEAD",
    ///                "OPTIONS"
    ///              ]
    ///            }
    ///          },
    ///          "path": {
    ///            "type": "object",
    ///            "required": [
    ///              "type",
    ///              "value"
    ///            ],
    ///            "properties": {
    ///              "type": {
    ///                "type": "string",
    ///                "enum": [
    ///                  "exact",
    ///                  "prefix",
    ///                  "glob"
    ///                ]
    ///              },
    ///              "value": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          "query": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "sourceIpCidrs": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          }
    ///        },
    ///        "additionalProperties": false
    ///      },
    ///      "minItems": 1
    ///    },
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "method": {
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
    ///    "not": {
    ///      "type": "object",
    ///      "properties": {
    ///        "asn": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "integer",
    ///            "maximum": 4294967295.0,
    ///            "minimum": 1.0
    ///          }
    ///        },
    ///        "cookies": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "device": {
    ///          "type": "string",
    ///          "enum": [
    ///            "desktop",
    ///            "mobile",
    ///            "tablet",
    ///            "bot"
    ///          ]
    ///        },
    ///        "geo": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "maxLength": 2,
    ///            "minLength": 2
    ///          }
    ///        },
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
    ///        "host": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "method": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "methods": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "GET",
    ///              "POST",
    ///              "PUT",
    ///              "DELETE",
    ///              "PATCH",
    ///              "HEAD",
    ///              "OPTIONS"
    ///            ]
    ///          }
    ///        },
    ///        "path": {
    ///          "type": "object",
    ///          "required": [
    ///            "type",
    ///            "value"
    ///          ],
    ///          "properties": {
    ///            "type": {
    ///              "type": "string",
    ///              "enum": [
    ///                "exact",
    ///                "prefix",
    ///                "glob"
    ///              ]
    ///            },
    ///            "value": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        },
    ///        "query": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "string"
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        },
    ///        "sourceIpCidrs": {
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        }
    ///      },
    ///      "additionalProperties": false
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
    ///            "glob"
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
    pub struct EdgeRuleAuthoringCondition {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub any: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionCookiesKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleAuthoringConditionDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleAuthoringConditionGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionHeadersKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleAuthoringConditionHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub method: ::std::vec::Vec<EdgeRuleAuthoringConditionMethodItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleAuthoringConditionMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub not: ::std::option::Option<EdgeRuleAuthoringConditionNot>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleAuthoringConditionPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query:
            ::std::collections::HashMap<EdgeRuleAuthoringConditionQueryKey, ::std::string::String>,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleAuthoringConditionSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleAuthoringCondition {
        fn default() -> Self {
            Self {
                any: Default::default(),
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                method: Default::default(),
                methods: Default::default(),
                not: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "method": {
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
    ///            "glob"
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
    pub struct EdgeRuleAuthoringConditionAnyItem {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionAnyItemCookiesKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleAuthoringConditionAnyItemDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionAnyItemHeadersKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleAuthoringConditionAnyItemHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub method: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemMethodItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleAuthoringConditionAnyItemPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionAnyItemQueryKey,
            ::std::string::String,
        >,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleAuthoringConditionAnyItem {
        fn default() -> Self {
            Self {
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                method: Default::default(),
                methods: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemCookiesKey`
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
    pub struct EdgeRuleAuthoringConditionAnyItemCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemCookiesKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemCookiesKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemCookiesKey {
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
    ///`EdgeRuleAuthoringConditionAnyItemDevice`
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
    pub enum EdgeRuleAuthoringConditionAnyItemDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemGeoItem`
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
    pub struct EdgeRuleAuthoringConditionAnyItemGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemGeoItem {
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
    ///`EdgeRuleAuthoringConditionAnyItemHeadersKey`
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
    pub struct EdgeRuleAuthoringConditionAnyItemHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemHeadersKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemHeadersKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemHeadersKey {
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
    ///`EdgeRuleAuthoringConditionAnyItemHost`
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
    pub struct EdgeRuleAuthoringConditionAnyItemHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemHost> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemHost {
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
    ///`EdgeRuleAuthoringConditionAnyItemMethodItem`
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
    pub enum EdgeRuleAuthoringConditionAnyItemMethodItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemMethodItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemMethodItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemMethodsItem`
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
    pub enum EdgeRuleAuthoringConditionAnyItemMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemMethodsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemPath`
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
    ///        "glob"
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
    pub struct EdgeRuleAuthoringConditionAnyItemPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleAuthoringConditionAnyItemPathType,
        pub value: EdgeRuleAuthoringConditionAnyItemPathValue,
    }
    ///`EdgeRuleAuthoringConditionAnyItemPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleAuthoringConditionAnyItemPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionAnyItemPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionAnyItemPathValue`
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
    pub struct EdgeRuleAuthoringConditionAnyItemPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemPathValue> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemPathValue
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemPathValue {
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
    ///`EdgeRuleAuthoringConditionAnyItemQueryKey`
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
    pub struct EdgeRuleAuthoringConditionAnyItemQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionAnyItemQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionAnyItemQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemQueryKey {
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
    ///`EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem`
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
    pub struct EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem>
        for ::std::string::String
    {
        fn from(value: EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionAnyItemSourceIpCidrsItem {
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
    ///`EdgeRuleAuthoringConditionCookiesKey`
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
    pub struct EdgeRuleAuthoringConditionCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionCookiesKey {
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
    ///`EdgeRuleAuthoringConditionDevice`
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
    pub enum EdgeRuleAuthoringConditionDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionGeoItem`
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
    pub struct EdgeRuleAuthoringConditionGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionGeoItem {
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
    ///`EdgeRuleAuthoringConditionHeadersKey`
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
    pub struct EdgeRuleAuthoringConditionHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionHeadersKey {
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
    ///`EdgeRuleAuthoringConditionHost`
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
    pub struct EdgeRuleAuthoringConditionHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionHost> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionHost {
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
    ///`EdgeRuleAuthoringConditionMethodItem`
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
    pub enum EdgeRuleAuthoringConditionMethodItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionMethodItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionMethodItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionMethodsItem`
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
    pub enum EdgeRuleAuthoringConditionMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNot`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "method": {
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
    ///            "glob"
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
    pub struct EdgeRuleAuthoringConditionNot {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionNotCookiesKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleAuthoringConditionNotDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleAuthoringConditionNotGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionNotHeadersKey,
            ::std::string::String,
        >,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleAuthoringConditionNotHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub method: ::std::vec::Vec<EdgeRuleAuthoringConditionNotMethodItem>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleAuthoringConditionNotMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleAuthoringConditionNotPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query: ::std::collections::HashMap<
            EdgeRuleAuthoringConditionNotQueryKey,
            ::std::string::String,
        >,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleAuthoringConditionNotSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleAuthoringConditionNot {
        fn default() -> Self {
            Self {
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                method: Default::default(),
                methods: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
    }
    ///`EdgeRuleAuthoringConditionNotCookiesKey`
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
    pub struct EdgeRuleAuthoringConditionNotCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotCookiesKey {
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
    ///`EdgeRuleAuthoringConditionNotDevice`
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
    pub enum EdgeRuleAuthoringConditionNotDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotGeoItem`
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
    pub struct EdgeRuleAuthoringConditionNotGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotGeoItem {
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
    ///`EdgeRuleAuthoringConditionNotHeadersKey`
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
    pub struct EdgeRuleAuthoringConditionNotHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotHeadersKey {
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
    ///`EdgeRuleAuthoringConditionNotHost`
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
    pub struct EdgeRuleAuthoringConditionNotHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotHost> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotHost {
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
    ///`EdgeRuleAuthoringConditionNotMethodItem`
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
    pub enum EdgeRuleAuthoringConditionNotMethodItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotMethodItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotMethodItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotMethodItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotMethodsItem`
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
    pub enum EdgeRuleAuthoringConditionNotMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotPath`
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
    ///        "glob"
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
    pub struct EdgeRuleAuthoringConditionNotPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleAuthoringConditionNotPathType,
        pub value: EdgeRuleAuthoringConditionNotPathValue,
    }
    ///`EdgeRuleAuthoringConditionNotPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleAuthoringConditionNotPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionNotPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionNotPathValue`
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
    pub struct EdgeRuleAuthoringConditionNotPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotPathValue> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotPathValue {
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
    ///`EdgeRuleAuthoringConditionNotQueryKey`
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
    pub struct EdgeRuleAuthoringConditionNotQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionNotQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionNotQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionNotQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotQueryKey {
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
    ///`EdgeRuleAuthoringConditionNotSourceIpCidrsItem`
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
    pub struct EdgeRuleAuthoringConditionNotSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionNotSourceIpCidrsItem>
        for ::std::string::String
    {
        fn from(value: EdgeRuleAuthoringConditionNotSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionNotSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionNotSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionNotSourceIpCidrsItem {
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
    ///`EdgeRuleAuthoringConditionPath`
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
    ///        "glob"
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
    pub struct EdgeRuleAuthoringConditionPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleAuthoringConditionPathType,
        pub value: EdgeRuleAuthoringConditionPathValue,
    }
    ///`EdgeRuleAuthoringConditionPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleAuthoringConditionPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleAuthoringConditionPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleAuthoringConditionPathValue`
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
    pub struct EdgeRuleAuthoringConditionPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionPathValue> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionPathValue {
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
    ///`EdgeRuleAuthoringConditionQueryKey`
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
    pub struct EdgeRuleAuthoringConditionQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleAuthoringConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleAuthoringConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionQueryKey {
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
    ///`EdgeRuleAuthoringConditionSourceIpCidrsItem`
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
    pub struct EdgeRuleAuthoringConditionSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleAuthoringConditionSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleAuthoringConditionSourceIpCidrsItem> for ::std::string::String {
        fn from(value: EdgeRuleAuthoringConditionSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleAuthoringConditionSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleAuthoringConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for EdgeRuleAuthoringConditionSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for EdgeRuleAuthoringConditionSourceIpCidrsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleAuthoringConditionSourceIpCidrsItem {
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
    ///        "rules": [
    ///          {
    ///            "action": {
    ///              "steps": [
    ///                {
    ///                  "handle": "hello-api"
    ///                }
    ///              ],
    ///              "type": "pipeline"
    ///            },
    ///            "condition": {
    ///              "method": [
    ///                "GET"
    ///              ],
    ///              "path": {
    ///                "type": "exact",
    ///                "value": "/api/hello"
    ///              }
    ///            },
    ///            "id": "hello-api"
    ///          }
    ///        ],
    ///        "schemaVersion": "EDGE_RULE_SET_V1",
    ///        "source": {
    ///          "origin": "build"
    ///        }
    ///      },
    ///      "functions": [
    ///        {
    ///          "source": {
    ///            "contentText": "export const config = { name: \"hello-api\" } as const;\\n\\nexport default { fetch() { return Response.json({ ok: true }); } };\\n",
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
    ///`PipelineHandleStep`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "handle"
    ///  ],
    ///  "properties": {
    ///    "handle": {
    ///      "type": "string",
    ///      "maxLength": 64,
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct PipelineHandleStep {
        pub handle: PipelineHandleStepHandle,
    }
    ///`PipelineHandleStepHandle`
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
    pub struct PipelineHandleStepHandle(::std::string::String);
    impl ::std::ops::Deref for PipelineHandleStepHandle {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<PipelineHandleStepHandle> for ::std::string::String {
        fn from(value: PipelineHandleStepHandle) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for PipelineHandleStepHandle {
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
    impl ::std::convert::TryFrom<&str> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for PipelineHandleStepHandle {
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
pub mod manifest {
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
    ///`EdgeRule`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "action",
    ///    "id",
    ///    "position"
    ///  ],
    ///  "properties": {
    ///    "action": {
    ///      "$ref": "#/definitions/EdgeRuleAction"
    ///    },
    ///    "condition": {
    ///      "default": {},
    ///      "$ref": "#/definitions/EdgeRuleCondition"
    ///    },
    ///    "enabled": {
    ///      "default": true,
    ///      "type": "boolean"
    ///    },
    ///    "id": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "name": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "position": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": 0.0
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct EdgeRule {
        pub action: EdgeRuleAction,
        #[serde(default = "defaults::edge_rule_condition")]
        pub condition: EdgeRuleCondition,
        #[serde(default = "defaults::default_bool::<true>")]
        pub enabled: bool,
        pub id: EdgeRuleId,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub name: ::std::option::Option<EdgeRuleName>,
        pub position: i64,
    }
    ///`EdgeRuleAction`
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
    ///          "default": "enforce",
    ///          "type": "string",
    ///          "enum": [
    ///            "shadow",
    ///            "enforce"
    ///          ]
    ///        },
    ///        "statusCode": {
    ///          "default": 403,
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
    ///        "ifNoFile": {
    ///          "default": true,
    ///          "type": "boolean"
    ///        },
    ///        "statusCode": {
    ///          "default": 301,
    ///          "type": "integer",
    ///          "maximum": 9007199254740991.0,
    ///          "minimum": -9007199254740991.0
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
    ///          "default": false,
    ///          "type": "boolean"
    ///        },
    ///        "ifNoFile": {
    ///          "default": true,
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
    ///          "default": [],
    ///          "type": "array",
    ///          "items": {
    ///            "type": "string",
    ///            "enum": [
    ///              "geo",
    ///              "asn",
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
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "steps",
    ///        "type"
    ///      ],
    ///      "properties": {
    ///        "inheritGate": {
    ///          "default": true,
    ///          "type": "boolean"
    ///        },
    ///        "override": {
    ///          "default": false,
    ///          "type": "boolean"
    ///        },
    ///        "steps": {
    ///          "type": "array",
    ///          "items": {
    ///            "$ref": "#/definitions/PipelineStep"
    ///          },
    ///          "minItems": 1
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "pipeline"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "limit",
    ///        "type",
    ///        "windowSeconds"
    ///      ],
    ///      "properties": {
    ///        "key": {
    ///          "default": "ip",
    ///          "type": "string",
    ///          "enum": [
    ///            "ip",
    ///            "ip_path",
    ///            "ip_host",
    ///            "host"
    ///          ]
    ///        },
    ///        "limit": {
    ///          "type": "integer",
    ///          "maximum": 100000.0,
    ///          "minimum": 1.0
    ///        },
    ///        "mode": {
    ///          "default": "enforce",
    ///          "type": "string",
    ///          "enum": [
    ///            "shadow",
    ///            "enforce"
    ///          ]
    ///        },
    ///        "type": {
    ///          "type": "string",
    ///          "const": "rate_limit"
    ///        },
    ///        "windowSeconds": {
    ///          "type": "integer",
    ///          "maximum": 600.0,
    ///          "minimum": 10.0
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
    pub enum EdgeRuleAction {
        #[serde(rename = "allow")]
        Allow,
        #[serde(rename = "log")]
        Log,
        #[serde(rename = "deny")]
        Deny {
            #[serde(default = "defaults::edge_rule_action_deny_mode")]
            mode: EdgeRuleActionMode,
            #[serde(rename = "statusCode", default = "defaults::default_u64::<i64, 403>")]
            status_code: i64,
        },
        #[serde(rename = "redirect")]
        Redirect {
            #[serde(rename = "ifNoFile", default = "defaults::default_bool::<true>")]
            if_no_file: bool,
            #[serde(rename = "statusCode", default = "defaults::default_u64::<i64, 301>")]
            status_code: i64,
            target: EdgeRuleActionTarget,
        },
        #[serde(rename = "rewrite")]
        Rewrite {
            #[serde(default)]
            external: bool,
            #[serde(rename = "ifNoFile", default = "defaults::default_bool::<true>")]
            if_no_file: bool,
            target: EdgeRuleActionTarget,
        },
        #[serde(rename = "set_headers")]
        SetHeaders {
            headers: ::std::collections::HashMap<EdgeRuleActionHeadersKey, ::std::string::String>,
        },
        #[serde(rename = "remove_headers")]
        RemoveHeaders {
            headers: ::std::vec::Vec<EdgeRuleActionHeadersItem>,
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
            vary: ::std::vec::Vec<EdgeRuleActionVaryItem>,
        },
        #[serde(rename = "bypass_cache")]
        BypassCache,
        #[serde(rename = "pipeline")]
        Pipeline {
            #[serde(rename = "inheritGate", default = "defaults::default_bool::<true>")]
            inherit_gate: bool,
            #[serde(rename = "override", default)]
            override_: bool,
            steps: ::std::vec::Vec<PipelineStep>,
        },
        #[serde(rename = "rate_limit")]
        RateLimit {
            #[serde(default = "defaults::edge_rule_action_rate_limit_key")]
            key: EdgeRuleActionKey,
            limit: ::std::num::NonZeroU64,
            #[serde(default = "defaults::edge_rule_action_rate_limit_mode")]
            mode: EdgeRuleActionMode,
            #[serde(rename = "windowSeconds")]
            window_seconds: i64,
        },
    }
    ///`EdgeRuleActionHeadersItem`
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
    pub struct EdgeRuleActionHeadersItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionHeadersItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionHeadersItem> for ::std::string::String {
        fn from(value: EdgeRuleActionHeadersItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionHeadersItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionHeadersItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionHeadersItem {
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
    ///`EdgeRuleActionHeadersKey`
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
    pub struct EdgeRuleActionHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleActionHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionHeadersKey {
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
    ///`EdgeRuleActionKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "default": "ip",
    ///  "type": "string",
    ///  "enum": [
    ///    "ip",
    ///    "ip_path",
    ///    "ip_host",
    ///    "host"
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
    pub enum EdgeRuleActionKey {
        #[serde(rename = "ip")]
        Ip,
        #[serde(rename = "ip_path")]
        IpPath,
        #[serde(rename = "ip_host")]
        IpHost,
        #[serde(rename = "host")]
        Host,
    }
    impl ::std::fmt::Display for EdgeRuleActionKey {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Ip => f.write_str("ip"),
                Self::IpPath => f.write_str("ip_path"),
                Self::IpHost => f.write_str("ip_host"),
                Self::Host => f.write_str("host"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "ip" => Ok(Self::Ip),
                "ip_path" => Ok(Self::IpPath),
                "ip_host" => Ok(Self::IpHost),
                "host" => Ok(Self::Host),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::default::Default for EdgeRuleActionKey {
        fn default() -> Self {
            EdgeRuleActionKey::Ip
        }
    }
    ///`EdgeRuleActionMode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "default": "enforce",
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
    pub enum EdgeRuleActionMode {
        #[serde(rename = "shadow")]
        Shadow,
        #[serde(rename = "enforce")]
        Enforce,
    }
    impl ::std::fmt::Display for EdgeRuleActionMode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Shadow => f.write_str("shadow"),
                Self::Enforce => f.write_str("enforce"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionMode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "shadow" => Ok(Self::Shadow),
                "enforce" => Ok(Self::Enforce),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionMode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::default::Default for EdgeRuleActionMode {
        fn default() -> Self {
            EdgeRuleActionMode::Enforce
        }
    }
    ///`EdgeRuleActionTarget`
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
    pub struct EdgeRuleActionTarget(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleActionTarget {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleActionTarget> for ::std::string::String {
        fn from(value: EdgeRuleActionTarget) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionTarget {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionTarget {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionTarget {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionTarget {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleActionTarget {
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
    ///`EdgeRuleActionVaryItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "geo",
    ///    "asn",
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
    pub enum EdgeRuleActionVaryItem {
        #[serde(rename = "geo")]
        Geo,
        #[serde(rename = "asn")]
        Asn,
        #[serde(rename = "device")]
        Device,
        #[serde(rename = "header")]
        Header,
        #[serde(rename = "cookie")]
        Cookie,
        #[serde(rename = "query")]
        Query,
    }
    impl ::std::fmt::Display for EdgeRuleActionVaryItem {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Geo => f.write_str("geo"),
                Self::Asn => f.write_str("asn"),
                Self::Device => f.write_str("device"),
                Self::Header => f.write_str("header"),
                Self::Cookie => f.write_str("cookie"),
                Self::Query => f.write_str("query"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleActionVaryItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "geo" => Ok(Self::Geo),
                "asn" => Ok(Self::Asn),
                "device" => Ok(Self::Device),
                "header" => Ok(Self::Header),
                "cookie" => Ok(Self::Cookie),
                "query" => Ok(Self::Query),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleActionVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleActionVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleActionVaryItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
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
    ///    "any": {
    ///      "type": "array",
    ///      "items": {
    ///        "$ref": "#/definitions/EdgeRuleLeafCondition"
    ///      },
    ///      "minItems": 1
    ///    },
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///    "not": {
    ///      "$ref": "#/definitions/EdgeRuleLeafCondition"
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
    ///            "glob"
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
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub any: ::std::vec::Vec<EdgeRuleLeafCondition>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
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
        pub not: ::std::option::Option<EdgeRuleLeafCondition>,
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
                any: Default::default(),
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                methods: Default::default(),
                not: Default::default(),
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
    ///        "glob"
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
    ///    "glob"
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
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleConditionPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleConditionPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
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
    ///`EdgeRuleId`
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
    pub struct EdgeRuleId(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleId> for ::std::string::String {
        fn from(value: EdgeRuleId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleId {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleId {
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
    ///`EdgeRuleLeafCondition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "asn": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "integer",
    ///        "maximum": 4294967295.0,
    ///        "minimum": 1.0
    ///      }
    ///    },
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
    ///            "glob"
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
    pub struct EdgeRuleLeafCondition {
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub asn: ::std::vec::Vec<::std::num::NonZeroU64>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub cookies:
            ::std::collections::HashMap<EdgeRuleLeafConditionCookiesKey, ::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub device: ::std::option::Option<EdgeRuleLeafConditionDevice>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub geo: ::std::vec::Vec<EdgeRuleLeafConditionGeoItem>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers:
            ::std::collections::HashMap<EdgeRuleLeafConditionHeadersKey, ::std::string::String>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub host: ::std::option::Option<EdgeRuleLeafConditionHost>,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<EdgeRuleLeafConditionMethodsItem>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub path: ::std::option::Option<EdgeRuleLeafConditionPath>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub query:
            ::std::collections::HashMap<EdgeRuleLeafConditionQueryKey, ::std::string::String>,
        #[serde(
            rename = "sourceIpCidrs",
            default,
            skip_serializing_if = "::std::vec::Vec::is_empty"
        )]
        pub source_ip_cidrs: ::std::vec::Vec<EdgeRuleLeafConditionSourceIpCidrsItem>,
    }
    impl ::std::default::Default for EdgeRuleLeafCondition {
        fn default() -> Self {
            Self {
                asn: Default::default(),
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
    ///`EdgeRuleLeafConditionCookiesKey`
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
    pub struct EdgeRuleLeafConditionCookiesKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionCookiesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionCookiesKey> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionCookiesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionCookiesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionCookiesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionCookiesKey {
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
    ///`EdgeRuleLeafConditionDevice`
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
    pub enum EdgeRuleLeafConditionDevice {
        #[serde(rename = "desktop")]
        Desktop,
        #[serde(rename = "mobile")]
        Mobile,
        #[serde(rename = "tablet")]
        Tablet,
        #[serde(rename = "bot")]
        Bot,
    }
    impl ::std::fmt::Display for EdgeRuleLeafConditionDevice {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Desktop => f.write_str("desktop"),
                Self::Mobile => f.write_str("mobile"),
                Self::Tablet => f.write_str("tablet"),
                Self::Bot => f.write_str("bot"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionDevice {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionDevice {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleLeafConditionGeoItem`
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
    pub struct EdgeRuleLeafConditionGeoItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionGeoItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionGeoItem> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionGeoItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionGeoItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionGeoItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionGeoItem {
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
    ///`EdgeRuleLeafConditionHeadersKey`
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
    pub struct EdgeRuleLeafConditionHeadersKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionHeadersKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionHeadersKey> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionHeadersKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionHeadersKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionHeadersKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionHeadersKey {
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
    ///`EdgeRuleLeafConditionHost`
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
    pub struct EdgeRuleLeafConditionHost(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionHost {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionHost> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionHost) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionHost {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionHost {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionHost {
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
    ///`EdgeRuleLeafConditionMethodsItem`
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
    pub enum EdgeRuleLeafConditionMethodsItem {
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
    impl ::std::fmt::Display for EdgeRuleLeafConditionMethodsItem {
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
    impl ::std::str::FromStr for EdgeRuleLeafConditionMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleLeafConditionPath`
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
    ///        "glob"
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
    pub struct EdgeRuleLeafConditionPath {
        #[serde(rename = "type")]
        pub type_: EdgeRuleLeafConditionPathType,
        pub value: EdgeRuleLeafConditionPathValue,
    }
    ///`EdgeRuleLeafConditionPathType`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "exact",
    ///    "prefix",
    ///    "glob"
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
    pub enum EdgeRuleLeafConditionPathType {
        #[serde(rename = "exact")]
        Exact,
        #[serde(rename = "prefix")]
        Prefix,
        #[serde(rename = "glob")]
        Glob,
    }
    impl ::std::fmt::Display for EdgeRuleLeafConditionPathType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Exact => f.write_str("exact"),
                Self::Prefix => f.write_str("prefix"),
                Self::Glob => f.write_str("glob"),
            }
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionPathType {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "exact" => Ok(Self::Exact),
                "prefix" => Ok(Self::Prefix),
                "glob" => Ok(Self::Glob),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionPathType {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`EdgeRuleLeafConditionPathValue`
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
    pub struct EdgeRuleLeafConditionPathValue(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionPathValue {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionPathValue> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionPathValue) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionPathValue {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionPathValue {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionPathValue {
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
    ///`EdgeRuleLeafConditionQueryKey`
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
    pub struct EdgeRuleLeafConditionQueryKey(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionQueryKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionQueryKey> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionQueryKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionQueryKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionQueryKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionQueryKey {
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
    ///`EdgeRuleLeafConditionSourceIpCidrsItem`
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
    pub struct EdgeRuleLeafConditionSourceIpCidrsItem(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleLeafConditionSourceIpCidrsItem {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleLeafConditionSourceIpCidrsItem> for ::std::string::String {
        fn from(value: EdgeRuleLeafConditionSourceIpCidrsItem) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleLeafConditionSourceIpCidrsItem {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleLeafConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleLeafConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleLeafConditionSourceIpCidrsItem {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleLeafConditionSourceIpCidrsItem {
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
    ///`EdgeRuleName`
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
    pub struct EdgeRuleName(::std::string::String);
    impl ::std::ops::Deref for EdgeRuleName {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<EdgeRuleName> for ::std::string::String {
        fn from(value: EdgeRuleName) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for EdgeRuleName {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for EdgeRuleName {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for EdgeRuleName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for EdgeRuleName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for EdgeRuleName {
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
    ///Build output manifest (manifest.json) — the contract between builder/adapter/CLI and the platform. Generated from the Zod source of truth used by the API server and consumed by the Rust CLI contract crate. Cross-field rules (layer/route references, regex compatibility, meta size) are enforced in code, not in this schema.
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "$id": "https://docs.onreza.ru/schemas/manifest-v1.schema.json",
    ///  "title": "ONREZA Build Output Manifest",
    ///  "description": "Build output manifest (manifest.json) — the contract between builder/adapter/CLI and the platform. Generated from the Zod source of truth used by the API server and consumed by the Rust CLI contract crate. Cross-field rules (layer/route references, regex compatibility, meta size) are enforced in code, not in this schema.",
    ///  "type": "object",
    ///  "required": [
    ///    "layers",
    ///    "routes",
    ///    "version"
    ///  ],
    ///  "properties": {
    ///    "edgeRules": {
    ///      "type": "object",
    ///      "required": [
    ///        "rules",
    ///        "schemaVersion",
    ///        "source"
    ///      ],
    ///      "properties": {
    ///        "rules": {
    ///          "type": "array",
    ///          "items": {
    ///            "$ref": "#/definitions/EdgeRule"
    ///          }
    ///        },
    ///        "schemaVersion": {
    ///          "type": "string",
    ///          "const": "EDGE_RULE_SET_V1"
    ///        },
    ///        "source": {
    ///          "type": "object",
    ///          "required": [
    ///            "origin"
    ///          ],
    ///          "properties": {
    ///            "origin": {
    ///              "type": "string",
    ///              "enum": [
    ///                "build",
    ///                "ui"
    ///              ]
    ///            },
    ///            "revisionId": {
    ///              "type": "string",
    ///              "minLength": 1
    ///            }
    ///          },
    ///          "additionalProperties": false
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    "layers": {
    ///      "type": "array",
    ///      "items": {
    ///        "oneOf": [
    ///          {
    ///            "type": "object",
    ///            "required": [
    ///              "directory",
    ///              "name",
    ///              "target"
    ///            ],
    ///            "properties": {
    ///              "directory": {
    ///                "type": "string",
    ///                "maxLength": 256,
    ///                "minLength": 1
    ///              },
    ///              "name": {
    ///                "type": "string",
    ///                "maxLength": 64,
    ///                "minLength": 1
    ///              },
    ///              "target": {
    ///                "type": "string",
    ///                "const": "STATIC"
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          },
    ///          {
    ///            "type": "object",
    ///            "required": [
    ///              "directory",
    ///              "entry",
    ///              "name",
    ///              "target"
    ///            ],
    ///            "properties": {
    ///              "directory": {
    ///                "type": "string",
    ///                "maxLength": 256,
    ///                "minLength": 1
    ///              },
    ///              "entry": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              },
    ///              "name": {
    ///                "type": "string",
    ///                "maxLength": 64,
    ///                "minLength": 1
    ///              },
    ///              "runtime": {
    ///                "type": "object",
    ///                "properties": {
    ///                  "maxConcurrency": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "exclusiveMinimum": 0.0
    ///                  },
    ///                  "memoryMb": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "exclusiveMinimum": 0.0
    ///                  },
    ///                  "timeoutMs": {
    ///                    "type": "integer",
    ///                    "maximum": 9007199254740991.0,
    ///                    "exclusiveMinimum": 0.0
    ///                  }
    ///                }
    ///              },
    ///              "target": {
    ///                "type": "string",
    ///                "const": "COMPUTE"
    ///              }
    ///            },
    ///            "additionalProperties": false
    ///          }
    ///        ]
    ///      },
    ///      "maxItems": 10,
    ///      "minItems": 1
    ///    },
    ///    "meta": {
    ///      "type": "object",
    ///      "additionalProperties": {},
    ///      "propertyNames": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "prerender": {
    ///      "type": "object",
    ///      "required": [
    ///        "layer",
    ///        "pages"
    ///      ],
    ///      "properties": {
    ///        "layer": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "pages": {
    ///          "type": "object",
    ///          "additionalProperties": {
    ///            "type": "object",
    ///            "required": [
    ///              "html"
    ///            ],
    ///            "properties": {
    ///              "data": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              },
    ///              "html": {
    ///                "type": "string",
    ///                "minLength": 1
    ///              }
    ///            }
    ///          },
    ///          "propertyNames": {
    ///            "type": "string",
    ///            "pattern": "^\\/.*"
    ///          }
    ///        }
    ///      }
    ///    },
    ///    "routes": {
    ///      "type": "array",
    ///      "items": {
    ///        "type": "object",
    ///        "required": [
    ///          "layer",
    ///          "pattern"
    ///        ],
    ///        "properties": {
    ///          "fallthrough": {
    ///            "type": "boolean"
    ///          },
    ///          "headers": {
    ///            "type": "object",
    ///            "additionalProperties": {
    ///              "type": "string"
    ///            },
    ///            "propertyNames": {
    ///              "type": "string"
    ///            }
    ///          },
    ///          "layer": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          },
    ///          "methods": {
    ///            "type": "array",
    ///            "items": {
    ///              "type": "string",
    ///              "enum": [
    ///                "GET",
    ///                "POST",
    ///                "PUT",
    ///                "DELETE",
    ///                "PATCH",
    ///                "HEAD",
    ///                "OPTIONS"
    ///              ]
    ///            }
    ///          },
    ///          "pattern": {
    ///            "type": "string",
    ///            "maxLength": 500,
    ///            "minLength": 1
    ///          },
    ///          "priority": {
    ///            "default": 0,
    ///            "type": "integer",
    ///            "maximum": 9007199254740991.0,
    ///            "minimum": -9007199254740991.0
    ///          },
    ///          "revalidate": {
    ///            "type": "integer",
    ///            "maximum": 31536000.0,
    ///            "exclusiveMinimum": 0.0
    ///          }
    ///        }
    ///      },
    ///      "maxItems": 200,
    ///      "minItems": 1
    ///    },
    ///    "version": {
    ///      "type": "number",
    ///      "const": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct OnrezaBuildOutputManifest {
        #[serde(
            rename = "edgeRules",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub edge_rules: ::std::option::Option<OnrezaBuildOutputManifestEdgeRules>,
        pub layers: ::std::vec::Vec<OnrezaBuildOutputManifestLayersItem>,
        #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
        pub meta: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub prerender: ::std::option::Option<OnrezaBuildOutputManifestPrerender>,
        pub routes: ::std::vec::Vec<OnrezaBuildOutputManifestRoutesItem>,
        pub version: f64,
    }
    ///`OnrezaBuildOutputManifestEdgeRules`
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
    ///        "$ref": "#/definitions/EdgeRule"
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
    pub struct OnrezaBuildOutputManifestEdgeRules {
        pub rules: ::std::vec::Vec<EdgeRule>,
        #[serde(rename = "schemaVersion")]
        pub schema_version: ::std::string::String,
        pub source: OnrezaBuildOutputManifestEdgeRulesSource,
    }
    ///`OnrezaBuildOutputManifestEdgeRulesSource`
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
    pub struct OnrezaBuildOutputManifestEdgeRulesSource {
        pub origin: OnrezaBuildOutputManifestEdgeRulesSourceOrigin,
        #[serde(
            rename = "revisionId",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub revision_id: ::std::option::Option<OnrezaBuildOutputManifestEdgeRulesSourceRevisionId>,
    }
    ///`OnrezaBuildOutputManifestEdgeRulesSourceOrigin`
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
    pub enum OnrezaBuildOutputManifestEdgeRulesSourceOrigin {
        #[serde(rename = "build")]
        Build,
        #[serde(rename = "ui")]
        Ui,
    }
    impl ::std::fmt::Display for OnrezaBuildOutputManifestEdgeRulesSourceOrigin {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Build => f.write_str("build"),
                Self::Ui => f.write_str("ui"),
            }
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestEdgeRulesSourceOrigin {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "build" => Ok(Self::Build),
                "ui" => Ok(Self::Ui),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestEdgeRulesSourceOrigin {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestEdgeRulesSourceOrigin
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaBuildOutputManifestEdgeRulesSourceOrigin
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaBuildOutputManifestEdgeRulesSourceRevisionId`
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
    pub struct OnrezaBuildOutputManifestEdgeRulesSourceRevisionId(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestEdgeRulesSourceRevisionId {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestEdgeRulesSourceRevisionId>
        for ::std::string::String
    {
        fn from(value: OnrezaBuildOutputManifestEdgeRulesSourceRevisionId) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestEdgeRulesSourceRevisionId {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestEdgeRulesSourceRevisionId {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestEdgeRulesSourceRevisionId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaBuildOutputManifestEdgeRulesSourceRevisionId
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestEdgeRulesSourceRevisionId {
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
    ///`OnrezaBuildOutputManifestLayersItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "oneOf": [
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "directory",
    ///        "name",
    ///        "target"
    ///      ],
    ///      "properties": {
    ///        "directory": {
    ///          "type": "string",
    ///          "maxLength": 256,
    ///          "minLength": 1
    ///        },
    ///        "name": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1
    ///        },
    ///        "target": {
    ///          "type": "string",
    ///          "const": "STATIC"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    },
    ///    {
    ///      "type": "object",
    ///      "required": [
    ///        "directory",
    ///        "entry",
    ///        "name",
    ///        "target"
    ///      ],
    ///      "properties": {
    ///        "directory": {
    ///          "type": "string",
    ///          "maxLength": 256,
    ///          "minLength": 1
    ///        },
    ///        "entry": {
    ///          "type": "string",
    ///          "minLength": 1
    ///        },
    ///        "name": {
    ///          "type": "string",
    ///          "maxLength": 64,
    ///          "minLength": 1
    ///        },
    ///        "runtime": {
    ///          "type": "object",
    ///          "properties": {
    ///            "maxConcurrency": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "exclusiveMinimum": 0.0
    ///            },
    ///            "memoryMb": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "exclusiveMinimum": 0.0
    ///            },
    ///            "timeoutMs": {
    ///              "type": "integer",
    ///              "maximum": 9007199254740991.0,
    ///              "exclusiveMinimum": 0.0
    ///            }
    ///          }
    ///        },
    ///        "target": {
    ///          "type": "string",
    ///          "const": "COMPUTE"
    ///        }
    ///      },
    ///      "additionalProperties": false
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(tag = "target", deny_unknown_fields)]
    pub enum OnrezaBuildOutputManifestLayersItem {
        #[serde(rename = "STATIC")]
        Static {
            directory: OnrezaBuildOutputManifestLayersItemDirectory,
            name: OnrezaBuildOutputManifestLayersItemName,
        },
        #[serde(rename = "COMPUTE")]
        Compute {
            directory: OnrezaBuildOutputManifestLayersItemDirectory,
            entry: OnrezaBuildOutputManifestLayersItemEntry,
            name: OnrezaBuildOutputManifestLayersItemName,
            #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
            runtime: ::std::option::Option<OnrezaBuildOutputManifestLayersItemRuntime>,
        },
    }
    ///`OnrezaBuildOutputManifestLayersItemDirectory`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 256,
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaBuildOutputManifestLayersItemDirectory(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestLayersItemDirectory {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestLayersItemDirectory> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestLayersItemDirectory) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestLayersItemDirectory {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 256usize {
                return Err("longer than 256 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestLayersItemDirectory {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestLayersItemDirectory
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaBuildOutputManifestLayersItemDirectory
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestLayersItemDirectory {
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
    ///`OnrezaBuildOutputManifestLayersItemEntry`
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
    pub struct OnrezaBuildOutputManifestLayersItemEntry(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestLayersItemEntry {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestLayersItemEntry> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestLayersItemEntry) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestLayersItemEntry {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestLayersItemEntry {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaBuildOutputManifestLayersItemEntry {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaBuildOutputManifestLayersItemEntry {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestLayersItemEntry {
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
    ///`OnrezaBuildOutputManifestLayersItemName`
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
    pub struct OnrezaBuildOutputManifestLayersItemName(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestLayersItemName {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestLayersItemName> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestLayersItemName) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestLayersItemName {
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
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestLayersItemName {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaBuildOutputManifestLayersItemName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaBuildOutputManifestLayersItemName {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestLayersItemName {
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
    ///`OnrezaBuildOutputManifestLayersItemRuntime`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "properties": {
    ///    "maxConcurrency": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "exclusiveMinimum": 0.0
    ///    },
    ///    "memoryMb": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "exclusiveMinimum": 0.0
    ///    },
    ///    "timeoutMs": {
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "exclusiveMinimum": 0.0
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    pub struct OnrezaBuildOutputManifestLayersItemRuntime {
        #[serde(
            rename = "maxConcurrency",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub max_concurrency: ::std::option::Option<::std::num::NonZeroU64>,
        #[serde(
            rename = "memoryMb",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub memory_mb: ::std::option::Option<::std::num::NonZeroU64>,
        #[serde(
            rename = "timeoutMs",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub timeout_ms: ::std::option::Option<::std::num::NonZeroU64>,
    }
    impl ::std::default::Default for OnrezaBuildOutputManifestLayersItemRuntime {
        fn default() -> Self {
            Self {
                max_concurrency: Default::default(),
                memory_mb: Default::default(),
                timeout_ms: Default::default(),
            }
        }
    }
    ///`OnrezaBuildOutputManifestPrerender`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "layer",
    ///    "pages"
    ///  ],
    ///  "properties": {
    ///    "layer": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "pages": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "object",
    ///        "required": [
    ///          "html"
    ///        ],
    ///        "properties": {
    ///          "data": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          },
    ///          "html": {
    ///            "type": "string",
    ///            "minLength": 1
    ///          }
    ///        }
    ///      },
    ///      "propertyNames": {
    ///        "type": "string",
    ///        "pattern": "^\\/.*"
    ///      }
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    pub struct OnrezaBuildOutputManifestPrerender {
        pub layer: OnrezaBuildOutputManifestPrerenderLayer,
        pub pages: ::std::collections::HashMap<
            OnrezaBuildOutputManifestPrerenderPagesKey,
            OnrezaBuildOutputManifestPrerenderPagesValue,
        >,
    }
    ///`OnrezaBuildOutputManifestPrerenderLayer`
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
    pub struct OnrezaBuildOutputManifestPrerenderLayer(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestPrerenderLayer {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestPrerenderLayer> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestPrerenderLayer) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestPrerenderLayer {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestPrerenderLayer {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaBuildOutputManifestPrerenderLayer {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaBuildOutputManifestPrerenderLayer {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestPrerenderLayer {
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
    ///`OnrezaBuildOutputManifestPrerenderPagesKey`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "pattern": "^\\/.*"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaBuildOutputManifestPrerenderPagesKey(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestPrerenderPagesKey {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestPrerenderPagesKey> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestPrerenderPagesKey) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestPrerenderPagesKey {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| ::regress::Regex::new("^\\/.*").unwrap());
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^\\/.*\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestPrerenderPagesKey {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestPrerenderPagesKey
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaBuildOutputManifestPrerenderPagesKey {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestPrerenderPagesKey {
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
    ///`OnrezaBuildOutputManifestPrerenderPagesValue`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "html"
    ///  ],
    ///  "properties": {
    ///    "data": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    },
    ///    "html": {
    ///      "type": "string",
    ///      "minLength": 1
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    pub struct OnrezaBuildOutputManifestPrerenderPagesValue {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub data: ::std::option::Option<OnrezaBuildOutputManifestPrerenderPagesValueData>,
        pub html: OnrezaBuildOutputManifestPrerenderPagesValueHtml,
    }
    ///`OnrezaBuildOutputManifestPrerenderPagesValueData`
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
    pub struct OnrezaBuildOutputManifestPrerenderPagesValueData(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestPrerenderPagesValueData {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestPrerenderPagesValueData>
        for ::std::string::String
    {
        fn from(value: OnrezaBuildOutputManifestPrerenderPagesValueData) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestPrerenderPagesValueData {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestPrerenderPagesValueData {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestPrerenderPagesValueData
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaBuildOutputManifestPrerenderPagesValueData
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestPrerenderPagesValueData {
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
    ///`OnrezaBuildOutputManifestPrerenderPagesValueHtml`
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
    pub struct OnrezaBuildOutputManifestPrerenderPagesValueHtml(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestPrerenderPagesValueHtml {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestPrerenderPagesValueHtml>
        for ::std::string::String
    {
        fn from(value: OnrezaBuildOutputManifestPrerenderPagesValueHtml) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestPrerenderPagesValueHtml {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestPrerenderPagesValueHtml {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestPrerenderPagesValueHtml
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaBuildOutputManifestPrerenderPagesValueHtml
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestPrerenderPagesValueHtml {
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
    ///`OnrezaBuildOutputManifestRoutesItem`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "layer",
    ///    "pattern"
    ///  ],
    ///  "properties": {
    ///    "fallthrough": {
    ///      "type": "boolean"
    ///    },
    ///    "headers": {
    ///      "type": "object",
    ///      "additionalProperties": {
    ///        "type": "string"
    ///      },
    ///      "propertyNames": {
    ///        "type": "string"
    ///      }
    ///    },
    ///    "layer": {
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
    ///    "pattern": {
    ///      "type": "string",
    ///      "maxLength": 500,
    ///      "minLength": 1
    ///    },
    ///    "priority": {
    ///      "default": 0,
    ///      "type": "integer",
    ///      "maximum": 9007199254740991.0,
    ///      "minimum": -9007199254740991.0
    ///    },
    ///    "revalidate": {
    ///      "type": "integer",
    ///      "maximum": 31536000.0,
    ///      "exclusiveMinimum": 0.0
    ///    }
    ///  }
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    pub struct OnrezaBuildOutputManifestRoutesItem {
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub fallthrough: ::std::option::Option<bool>,
        #[serde(
            default,
            skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
        )]
        pub headers: ::std::collections::HashMap<::std::string::String, ::std::string::String>,
        pub layer: OnrezaBuildOutputManifestRoutesItemLayer,
        #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
        pub methods: ::std::vec::Vec<OnrezaBuildOutputManifestRoutesItemMethodsItem>,
        pub pattern: OnrezaBuildOutputManifestRoutesItemPattern,
        #[serde(default)]
        pub priority: i64,
        #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
        pub revalidate: ::std::option::Option<::std::num::NonZeroU64>,
    }
    ///`OnrezaBuildOutputManifestRoutesItemLayer`
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
    pub struct OnrezaBuildOutputManifestRoutesItemLayer(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestRoutesItemLayer {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestRoutesItemLayer> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestRoutesItemLayer) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestRoutesItemLayer {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestRoutesItemLayer {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for OnrezaBuildOutputManifestRoutesItemLayer {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaBuildOutputManifestRoutesItemLayer {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestRoutesItemLayer {
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
    ///`OnrezaBuildOutputManifestRoutesItemMethodsItem`
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
    pub enum OnrezaBuildOutputManifestRoutesItemMethodsItem {
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
    impl ::std::fmt::Display for OnrezaBuildOutputManifestRoutesItemMethodsItem {
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
    impl ::std::str::FromStr for OnrezaBuildOutputManifestRoutesItemMethodsItem {
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
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestRoutesItemMethodsItem {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestRoutesItemMethodsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String>
        for OnrezaBuildOutputManifestRoutesItemMethodsItem
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`OnrezaBuildOutputManifestRoutesItemPattern`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 500,
    ///  "minLength": 1
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct OnrezaBuildOutputManifestRoutesItemPattern(::std::string::String);
    impl ::std::ops::Deref for OnrezaBuildOutputManifestRoutesItemPattern {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<OnrezaBuildOutputManifestRoutesItemPattern> for ::std::string::String {
        fn from(value: OnrezaBuildOutputManifestRoutesItemPattern) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for OnrezaBuildOutputManifestRoutesItemPattern {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 500usize {
                return Err("longer than 500 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for OnrezaBuildOutputManifestRoutesItemPattern {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String>
        for OnrezaBuildOutputManifestRoutesItemPattern
    {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for OnrezaBuildOutputManifestRoutesItemPattern {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for OnrezaBuildOutputManifestRoutesItemPattern {
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
    ///`PipelineHandleStep`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "handle"
    ///  ],
    ///  "properties": {
    ///    "handle": {
    ///      "type": "string",
    ///      "maxLength": 64,
    ///      "minLength": 1
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct PipelineHandleStep {
        pub handle: PipelineHandleStepHandle,
    }
    ///`PipelineHandleStepHandle`
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
    pub struct PipelineHandleStepHandle(::std::string::String);
    impl ::std::ops::Deref for PipelineHandleStepHandle {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<PipelineHandleStepHandle> for ::std::string::String {
        fn from(value: PipelineHandleStepHandle) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for PipelineHandleStepHandle {
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
    impl ::std::convert::TryFrom<&str> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineHandleStepHandle {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for PipelineHandleStepHandle {
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
    ///`PipelineStep`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "anyOf": [
    ///    {
    ///      "$ref": "#/definitions/PipelineUseStep"
    ///    },
    ///    {
    ///      "$ref": "#/definitions/PipelineHandleStep"
    ///    }
    ///  ]
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(untagged)]
    pub enum PipelineStep {
        UseStep(PipelineUseStep),
        HandleStep(PipelineHandleStep),
    }
    impl ::std::convert::From<PipelineUseStep> for PipelineStep {
        fn from(value: PipelineUseStep) -> Self {
            Self::UseStep(value)
        }
    }
    impl ::std::convert::From<PipelineHandleStep> for PipelineStep {
        fn from(value: PipelineHandleStep) -> Self {
            Self::HandleStep(value)
        }
    }
    ///`PipelineUseStep`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "object",
    ///  "required": [
    ///    "failure",
    ///    "mode",
    ///    "use"
    ///  ],
    ///  "properties": {
    ///    "as": {
    ///      "type": "string",
    ///      "maxLength": 64,
    ///      "minLength": 1
    ///    },
    ///    "cachePosition": {
    ///      "type": "string",
    ///      "enum": [
    ///        "before",
    ///        "after"
    ///      ]
    ///    },
    ///    "failure": {
    ///      "type": "string",
    ///      "enum": [
    ///        "closed",
    ///        "open"
    ///      ]
    ///    },
    ///    "mode": {
    ///      "type": "string",
    ///      "enum": [
    ///        "request",
    ///        "response",
    ///        "observe"
    ///      ]
    ///    },
    ///    "use": {
    ///      "type": "string",
    ///      "maxLength": 64,
    ///      "minLength": 1,
    ///      "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///    }
    ///  },
    ///  "additionalProperties": false
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct PipelineUseStep {
        #[serde(
            rename = "as",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub as_: ::std::option::Option<PipelineUseStepAs>,
        #[serde(
            rename = "cachePosition",
            default,
            skip_serializing_if = "::std::option::Option::is_none"
        )]
        pub cache_position: ::std::option::Option<PipelineUseStepCachePosition>,
        pub failure: PipelineUseStepFailure,
        pub mode: PipelineUseStepMode,
        #[serde(rename = "use")]
        pub use_: PipelineUseStepUse,
    }
    ///`PipelineUseStepAs`
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
    pub struct PipelineUseStepAs(::std::string::String);
    impl ::std::ops::Deref for PipelineUseStepAs {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<PipelineUseStepAs> for ::std::string::String {
        fn from(value: PipelineUseStepAs) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for PipelineUseStepAs {
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
    impl ::std::convert::TryFrom<&str> for PipelineUseStepAs {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineUseStepAs {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineUseStepAs {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for PipelineUseStepAs {
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
    ///`PipelineUseStepCachePosition`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "before",
    ///    "after"
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
    pub enum PipelineUseStepCachePosition {
        #[serde(rename = "before")]
        Before,
        #[serde(rename = "after")]
        After,
    }
    impl ::std::fmt::Display for PipelineUseStepCachePosition {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Before => f.write_str("before"),
                Self::After => f.write_str("after"),
            }
        }
    }
    impl ::std::str::FromStr for PipelineUseStepCachePosition {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "before" => Ok(Self::Before),
                "after" => Ok(Self::After),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for PipelineUseStepCachePosition {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineUseStepCachePosition {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineUseStepCachePosition {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`PipelineUseStepFailure`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "closed",
    ///    "open"
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
    pub enum PipelineUseStepFailure {
        #[serde(rename = "closed")]
        Closed,
        #[serde(rename = "open")]
        Open,
    }
    impl ::std::fmt::Display for PipelineUseStepFailure {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Closed => f.write_str("closed"),
                Self::Open => f.write_str("open"),
            }
        }
    }
    impl ::std::str::FromStr for PipelineUseStepFailure {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "closed" => Ok(Self::Closed),
                "open" => Ok(Self::Open),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for PipelineUseStepFailure {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineUseStepFailure {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineUseStepFailure {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`PipelineUseStepMode`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "enum": [
    ///    "request",
    ///    "response",
    ///    "observe"
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
    pub enum PipelineUseStepMode {
        #[serde(rename = "request")]
        Request,
        #[serde(rename = "response")]
        Response,
        #[serde(rename = "observe")]
        Observe,
    }
    impl ::std::fmt::Display for PipelineUseStepMode {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match *self {
                Self::Request => f.write_str("request"),
                Self::Response => f.write_str("response"),
                Self::Observe => f.write_str("observe"),
            }
        }
    }
    impl ::std::str::FromStr for PipelineUseStepMode {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            match value {
                "request" => Ok(Self::Request),
                "response" => Ok(Self::Response),
                "observe" => Ok(Self::Observe),
                _ => Err("invalid value".into()),
            }
        }
    }
    impl ::std::convert::TryFrom<&str> for PipelineUseStepMode {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineUseStepMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineUseStepMode {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    ///`PipelineUseStepUse`
    ///
    /// <details><summary>JSON schema</summary>
    ///
    /// ```json
    ///{
    ///  "type": "string",
    ///  "maxLength": 64,
    ///  "minLength": 1,
    ///  "pattern": "^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$"
    ///}
    /// ```
    /// </details>
    #[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[serde(transparent)]
    pub struct PipelineUseStepUse(::std::string::String);
    impl ::std::ops::Deref for PipelineUseStepUse {
        type Target = ::std::string::String;
        fn deref(&self) -> &::std::string::String {
            &self.0
        }
    }
    impl ::std::convert::From<PipelineUseStepUse> for ::std::string::String {
        fn from(value: PipelineUseStepUse) -> Self {
            value.0
        }
    }
    impl ::std::str::FromStr for PipelineUseStepUse {
        type Err = self::error::ConversionError;
        fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            if value.chars().count() > 64usize {
                return Err("longer than 64 characters".into());
            }
            if value.chars().count() < 1usize {
                return Err("shorter than 1 characters".into());
            }
            static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
                ::std::sync::LazyLock::new(|| {
                    ::regress::Regex::new("^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$").unwrap()
                });
            if PATTERN.find(value).is_none() {
                return Err("doesn't match pattern \"^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$\"".into());
            }
            Ok(Self(value.to_string()))
        }
    }
    impl ::std::convert::TryFrom<&str> for PipelineUseStepUse {
        type Error = self::error::ConversionError;
        fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<&::std::string::String> for PipelineUseStepUse {
        type Error = self::error::ConversionError;
        fn try_from(
            value: &::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl ::std::convert::TryFrom<::std::string::String> for PipelineUseStepUse {
        type Error = self::error::ConversionError;
        fn try_from(
            value: ::std::string::String,
        ) -> ::std::result::Result<Self, self::error::ConversionError> {
            value.parse()
        }
    }
    impl<'de> ::serde::Deserialize<'de> for PipelineUseStepUse {
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
    /// Generation of default values for serde.
    pub mod defaults {
        pub(super) fn default_bool<const V: bool>() -> bool {
            V
        }
        pub(super) fn default_u64<T, const V: u64>() -> T
        where
            T: ::std::convert::TryFrom<u64>,
            <T as ::std::convert::TryFrom<u64>>::Error: ::std::fmt::Debug,
        {
            T::try_from(V).unwrap()
        }
        pub(super) fn edge_rule_condition() -> super::EdgeRuleCondition {
            super::EdgeRuleCondition {
                any: Default::default(),
                asn: Default::default(),
                cookies: Default::default(),
                device: Default::default(),
                geo: Default::default(),
                headers: Default::default(),
                host: Default::default(),
                methods: Default::default(),
                not: Default::default(),
                path: Default::default(),
                query: Default::default(),
                source_ip_cidrs: Default::default(),
            }
        }
        pub(super) fn edge_rule_action_deny_mode() -> super::EdgeRuleActionMode {
            super::EdgeRuleActionMode::Enforce
        }
        pub(super) fn edge_rule_action_rate_limit_key() -> super::EdgeRuleActionKey {
            super::EdgeRuleActionKey::Ip
        }
        pub(super) fn edge_rule_action_rate_limit_mode() -> super::EdgeRuleActionMode {
            super::EdgeRuleActionMode::Enforce
        }
    }
}
