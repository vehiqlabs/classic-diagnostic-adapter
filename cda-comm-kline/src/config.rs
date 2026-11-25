/*
 * SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 */

use cda_interfaces::TransportType;
use serde::{Deserialize, Serialize};

/// Configuration for K-Line (ISO 14230 / ISO 9141-2) communication.
// Every field has a serde default: `[kline]` is optional, so it deserializes
// without the figment defaults tree behind it.
#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct KLineConfig {
    /// Serial device the K-Line adapter is reached through (e.g.
    /// `/dev/ttyUSB0`).
    pub interface: String,

    #[serde(default)]
    pub ecu_mappings: Vec<KLineEcuMapping>,

    /// Per-ECU transport pins; a value of `kline` binds the ECU here.
    #[serde(default)]
    pub transport_overrides: Vec<TransportOverride>,

    #[serde(default = "default_init_mode")]
    pub init_mode: KLineInitMode,

    #[serde(default = "default_response_timeout_ms")]
    pub response_timeout_ms: u64,

    /// Timeout for the init handshake when probing an ECU, in milliseconds.
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,

    /// Keep-alive interval in milliseconds; must stay below P3max. `0` disables.
    #[serde(default = "default_keepalive_interval_ms")]
    pub keepalive_interval_ms: u64,

    #[serde(default)]
    pub timing: KLineTimingConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct TransportOverride {
    pub ecu_name: String,
    pub transport: TransportType,
}

#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct KLineEcuMapping {
    pub ecu_name: String,
    /// Physical target address byte carried in the ISO 14230-2 header.
    pub physical_address: u8,
    #[serde(default)]
    pub functional_address: Option<u8>,
    /// Link baud rate; `None` uses the transport default (10400 baud).
    #[serde(default)]
    pub baud_rate: Option<u32>,
}

/// Link initialization mode (ISO 14230-2 / ISO 9141-2).
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KLineInitMode {
    /// Fast initialization followed by a `StartCommunication` request.
    FastInit,
    /// 5-baud address followed by the key-byte exchange.
    FiveBaudInit,
}

/// Optional ISO 14230-2 timing overrides, in milliseconds. An unset value
/// defers to the MDD com-param, then to the standard default.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, schemars::JsonSchema)]
pub struct KLineTimingConfig {
    /// P2max: request to start of response.
    #[serde(default)]
    pub p2_max_ms: Option<u64>,
    /// P3min: end of response to next request.
    #[serde(default)]
    pub p3_min_ms: Option<u64>,
    /// P4min: inter-byte time of a tester request.
    #[serde(default)]
    pub p4_min_ms: Option<u64>,
}

impl Default for KLineConfig {
    fn default() -> Self {
        Self {
            interface: "/dev/ttyUSB0".to_owned(),
            ecu_mappings: Vec::new(),
            transport_overrides: Vec::new(),
            init_mode: KLineInitMode::FastInit,
            response_timeout_ms: 5000,
            probe_timeout_ms: 500,
            keepalive_interval_ms: 2000,
            timing: KLineTimingConfig::default(),
        }
    }
}

fn default_init_mode() -> KLineInitMode {
    KLineConfig::default().init_mode
}

fn default_response_timeout_ms() -> u64 {
    KLineConfig::default().response_timeout_ms
}

fn default_probe_timeout_ms() -> u64 {
    KLineConfig::default().probe_timeout_ms
}

fn default_keepalive_interval_ms() -> u64 {
    KLineConfig::default().keepalive_interval_ms
}

#[cfg(test)]
mod tests {
    use super::{KLineConfig, KLineInitMode};

    #[test]
    fn minimal_kline_section_deserializes_with_defaults() {
        let config: KLineConfig =
            toml::from_str(r#"interface = "/dev/ttyUSB0""#).expect("minimal config");

        assert_eq!(config.interface, "/dev/ttyUSB0");
        assert_eq!(config.init_mode, KLineInitMode::FastInit);
        assert_eq!(
            config.response_timeout_ms,
            KLineConfig::default().response_timeout_ms
        );
        assert_eq!(
            config.keepalive_interval_ms,
            KLineConfig::default().keepalive_interval_ms
        );
        assert!(config.ecu_mappings.is_empty());
        assert!(config.transport_overrides.is_empty());
    }

    #[test]
    fn init_mode_parses_snake_case() {
        let config: KLineConfig =
            toml::from_str("interface = \"/dev/ttyUSB0\"\ninit_mode = \"five_baud_init\"")
                .expect("config with init mode");

        assert_eq!(config.init_mode, KLineInitMode::FiveBaudInit);
    }
}
