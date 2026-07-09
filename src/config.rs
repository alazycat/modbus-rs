//! Configuration file support for Modbus clients and servers.
//!
//! This module is enabled by the `config` feature. It provides serde-
//! deserializable structs for JSON/TOML/YAML configuration files and conversion
//! helpers that map them onto the runtime types used by the client and server.

#![cfg(feature = "config")]

use std::time::Duration;

use serde::Deserialize;

/// Errors that can occur while loading configuration files.
#[derive(Debug)]
pub enum ConfigError {
    /// The configuration file could not be parsed.
    Parse(String),
    /// A configuration value is invalid or unsupported.
    InvalidValue(String),
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "config parse error: {msg}"),
            Self::InvalidValue(msg) => write!(f, "config invalid value: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Client configuration as it appears in a configuration file.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ClientConfigFile {
    /// Response timeout in milliseconds.
    pub timeout_ms: u64,
    /// Optional retry policy.
    pub retry_policy: Option<RetryPolicyFile>,
    /// Byte order used by typed register helpers (requires the `helpers` feature).
    #[cfg(feature = "helpers")]
    pub endian: Option<String>,
    /// Word order used by multi-register typed helpers (requires the `helpers` feature).
    #[cfg(feature = "helpers")]
    pub word_order: Option<String>,
}

impl Default for ClientConfigFile {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            retry_policy: None,
            #[cfg(feature = "helpers")]
            endian: None,
            #[cfg(feature = "helpers")]
            word_order: None,
        }
    }
}

/// Retry policy configuration as it appears in a configuration file.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RetryPolicyFile {
    /// Maximum number of retry attempts before giving up.
    pub max_retries: u32,
    /// Initial delay before the first retry, in milliseconds.
    pub initial_backoff_ms: u64,
    /// Maximum delay between retries, in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicyFile {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
        }
    }
}

impl ClientConfigFile {
    /// Convert this file configuration into the runtime client config and retry
    /// policy.
    #[cfg(any(feature = "sync", feature = "async"))]
    pub fn into_parts(self) -> Result<(crate::client::ClientConfig, crate::client::RetryPolicy), ConfigError> {
        use crate::client::{ClientConfig, RetryPolicy, default_retryable};

        #[cfg(feature = "helpers")]
        use crate::helpers::{Endian, WordOrder};

        let timeout = Duration::from_millis(self.timeout_ms);

        let retry = match self.retry_policy {
            Some(policy) => RetryPolicy {
                max_retries: policy.max_retries,
                initial_backoff: Duration::from_millis(policy.initial_backoff_ms),
                max_backoff: Duration::from_millis(policy.max_backoff_ms),
                retryable: default_retryable,
            },
            None => RetryPolicy::default(),
        };

        #[cfg(feature = "helpers")]
        let endian = match self.endian.as_deref() {
            None | Some("big") => Endian::Big,
            Some("little") => Endian::Little,
            Some(other) => {
                return Err(ConfigError::InvalidValue(format!(
                    "endian must be 'big' or 'little', got '{other}'"
                )))
            }
        };

        #[cfg(feature = "helpers")]
        let word_order = match self.word_order.as_deref() {
            None | Some("msf") => WordOrder::MostSignificantFirst,
            Some("lsf") => WordOrder::LeastSignificantFirst,
            Some(other) => {
                return Err(ConfigError::InvalidValue(format!(
                    "word_order must be 'msf' or 'lsf', got '{other}'"
                )))
            }
        };

        let config = ClientConfig {
            timeout,
            #[cfg(feature = "helpers")]
            endian,
            #[cfg(feature = "helpers")]
            word_order,
        };

        Ok((config, retry))
    }
}

/// Load a [`ClientConfigFile`] from a JSON string.
pub fn client_from_json(text: &str) -> Result<ClientConfigFile, ConfigError> {
    serde_json::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Load a [`ClientConfigFile`] from a TOML string.
pub fn client_from_toml(text: &str) -> Result<ClientConfigFile, ConfigError> {
    toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Load a [`ClientConfigFile`] from a YAML string.
pub fn client_from_yaml(text: &str) -> Result<ClientConfigFile, ConfigError> {
    serde_yaml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Server transport kind as it appears in a configuration file.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerTransport {
    /// Plain TCP transport.
    #[default]
    Tcp,
    /// UDP transport.
    Udp,
    /// RTU serial transport.
    Rtu,
    /// ASCII serial transport.
    Ascii,
    /// RTU framing over a TCP stream.
    #[serde(rename = "rtu_over_tcp")]
    RtuOverTcp,
    /// TLS-wrapped TCP transport.
    #[serde(rename = "tls")]
    Tls,
    /// Direct serial-port RTU transport.
    #[serde(rename = "serial")]
    Serial,
}

/// Server configuration as it appears in a configuration file.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServerConfigFile {
    /// Address the server should bind to (e.g. `127.0.0.1:502` or a serial port).
    pub bind_address: String,
    /// Transport protocol the server should use.
    pub transport: ServerTransport,
    /// Optional path to a persistent backing store.
    pub store_path: Option<String>,
}

impl Default for ServerConfigFile {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:502".to_string(),
            transport: ServerTransport::Tcp,
            store_path: None,
        }
    }
}

/// Load a [`ServerConfigFile`] from a JSON string.
pub fn server_from_json(text: &str) -> Result<ServerConfigFile, ConfigError> {
    serde_json::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Load a [`ServerConfigFile`] from a TOML string.
pub fn server_from_toml(text: &str) -> Result<ServerConfigFile, ConfigError> {
    toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Load a [`ServerConfigFile`] from a YAML string.
pub fn server_from_yaml(text: &str) -> Result<ServerConfigFile, ConfigError> {
    serde_yaml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_defaults_are_applied() {
        let file: ClientConfigFile = client_from_json("{}").unwrap();
        assert_eq!(file.timeout_ms, 5000);
        let retry = file.retry_policy.unwrap_or_default();
        assert_eq!(retry.max_retries, 3);
        assert_eq!(retry.initial_backoff_ms, 100);
        assert_eq!(retry.max_backoff_ms, 5000);
    }

    #[test]
    fn client_json_roundtrip_with_custom_values() {
        let text = r#"{
            "timeout_ms": 2000,
            "retry_policy": {
                "max_retries": 5,
                "initial_backoff_ms": 250,
                "max_backoff_ms": 10000
            }
        }"#;
        let file: ClientConfigFile = client_from_json(text).unwrap();
        assert_eq!(file.timeout_ms, 2000);
        let retry = file.retry_policy.unwrap();
        assert_eq!(retry.max_retries, 5);
        assert_eq!(retry.initial_backoff_ms, 250);
        assert_eq!(retry.max_backoff_ms, 10000);
    }

    #[test]
    fn client_toml_roundtrip_with_custom_values() {
        let text = r#"
timeout_ms = 3000

[retry_policy]
max_retries = 2
initial_backoff_ms = 50
max_backoff_ms = 1000
"#;
        let file: ClientConfigFile = client_from_toml(text).unwrap();
        assert_eq!(file.timeout_ms, 3000);
        let retry = file.retry_policy.unwrap();
        assert_eq!(retry.max_retries, 2);
        assert_eq!(retry.initial_backoff_ms, 50);
        assert_eq!(retry.max_backoff_ms, 1000);
    }

    #[test]
    fn client_yaml_roundtrip_with_custom_values() {
        let text = r#"
timeout_ms: 1500
retry_policy:
  max_retries: 1
  initial_backoff_ms: 10
  max_backoff_ms: 500
"#;
        let file: ClientConfigFile = client_from_yaml(text).unwrap();
        assert_eq!(file.timeout_ms, 1500);
        let retry = file.retry_policy.unwrap();
        assert_eq!(retry.max_retries, 1);
        assert_eq!(retry.initial_backoff_ms, 10);
        assert_eq!(retry.max_backoff_ms, 500);
    }

    #[cfg(any(feature = "sync", feature = "async"))]
    #[test]
    fn into_parts_applies_timeout_and_retry() {
        let text = r#"{"timeout_ms": 1000, "retry_policy": {"max_retries": 2}}"#;
        let file: ClientConfigFile = client_from_json(text).unwrap();
        let (config, retry) = file.into_parts().unwrap();
        assert_eq!(config.timeout, Duration::from_millis(1000));
        assert_eq!(retry.max_retries, 2);
    }

    #[cfg(all(feature = "helpers", any(feature = "sync", feature = "async")))]
    #[test]
    fn into_parts_parses_endian_and_word_order() {
        let text = r#"{"endian": "little", "word_order": "lsf"}"#;
        let file: ClientConfigFile = client_from_json(text).unwrap();
        let (config, _retry) = file.into_parts().unwrap();
        assert_eq!(config.endian, crate::helpers::Endian::Little);
        assert_eq!(
            config.word_order,
            crate::helpers::WordOrder::LeastSignificantFirst
        );
    }

    #[cfg(all(feature = "helpers", any(feature = "sync", feature = "async")))]
    #[test]
    fn into_parts_rejects_invalid_endian() {
        let text = r#"{"endian": "mixed"}"#;
        let file: ClientConfigFile = client_from_json(text).unwrap();
        let err = file.into_parts().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue(_)),
            "expected InvalidValue, got {err:?}"
        );
    }

    #[test]
    fn server_defaults_are_applied() {
        let file: ServerConfigFile = server_from_json("{}").unwrap();
        assert_eq!(file.bind_address, "127.0.0.1:502");
        assert_eq!(file.transport, ServerTransport::Tcp);
        assert!(file.store_path.is_none());
    }

    #[test]
    fn server_json_roundtrip_with_custom_values() {
        let text = r#"{
            "bind_address": "0.0.0.0:1502",
            "transport": "rtu_over_tcp",
            "store_path": "/var/lib/modbus/store.json"
        }"#;
        let file: ServerConfigFile = server_from_json(text).unwrap();
        assert_eq!(file.bind_address, "0.0.0.0:1502");
        assert_eq!(file.transport, ServerTransport::RtuOverTcp);
        assert_eq!(file.store_path.as_deref(), Some("/var/lib/modbus/store.json"));
    }

    #[test]
    fn server_toml_roundtrip_with_custom_values() {
        let text = r#"
bind_address = "0.0.0.0:502"
transport = "udp"
store_path = "/tmp/store.json"
"#;
        let file: ServerConfigFile = server_from_toml(text).unwrap();
        assert_eq!(file.bind_address, "0.0.0.0:502");
        assert_eq!(file.transport, ServerTransport::Udp);
        assert_eq!(file.store_path.as_deref(), Some("/tmp/store.json"));
    }

    #[test]
    fn server_yaml_roundtrip_with_custom_values() {
        let text = r#"
bind_address: 0.0.0.0:1502
transport: serial
store_path: /var/lib/modbus/store.json
"#;
        let file: ServerConfigFile = server_from_yaml(text).unwrap();
        assert_eq!(file.bind_address, "0.0.0.0:1502");
        assert_eq!(file.transport, ServerTransport::Serial);
        assert_eq!(file.store_path.as_deref(), Some("/var/lib/modbus/store.json"));
    }

    #[test]
    fn server_invalid_transport_is_parse_error() {
        let text = r#"{"transport": "infrared"}"#;
        let err = server_from_json(text).unwrap_err();
        assert!(
            matches!(err, ConfigError::Parse(_)),
            "expected Parse error, got {err:?}"
        );
    }
}
