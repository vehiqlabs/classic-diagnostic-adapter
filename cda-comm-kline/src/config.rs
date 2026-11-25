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
#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct KLineConfig {
    /// Serial port of the K-Line adapter as the host names it, e.g.
    /// "/dev/ttyUSB0" (Linux), "/dev/cu.usbserial-XXXX" (macOS) or "COM3"
    /// (Windows). Required, there is no default.
    pub interface: String,

    /// Explicit per-ECU K-Line addressing; an entry takes precedence over
    /// the MDD com-params of that ECU.
    #[serde(default)]
    pub ecu_mappings: Vec<KLineEcuMapping>,

    /// Per-ECU transport pins; a value of `kline` binds the ECU to K-Line.
    #[serde(default)]
    pub transport_overrides: Vec<TransportOverride>,

    /// Link initialization: `fast_init` (default) or `five_baud_init`.
    #[serde(default)]
    pub init_mode: KLineInitMode,

    /// Timeout for waiting for a response in milliseconds.
    #[serde(default = "default_response_timeout_ms")]
    pub response_timeout_ms: u64,

    /// Timeout for the init handshake when probing an ECU, in milliseconds.
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,

    /// `TesterPresent` keep-alive interval in milliseconds; must stay below
    /// P3max. `0` disables it.
    #[serde(default = "default_keepalive_interval_ms")]
    pub keepalive_interval_ms: u64,

    #[serde(default)]
    pub timing: KLineTimingConfig,
}

/// Pins an ECU to a transport.
#[derive(Deserialize, Serialize, Clone, Debug, schemars::JsonSchema)]
pub struct TransportOverride {
    pub ecu_name: String,
    pub transport: TransportType,
}

/// Explicit K-Line addressing of one ECU.
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

/// How the link to an ECU is initialized.
#[derive(
    Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, Default, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum KLineInitMode {
    /// Fast initialization followed by a `StartCommunication` request.
    #[default]
    FastInit,
    /// 5-baud address followed by the key-byte exchange.
    FiveBaudInit,
}

/// Optional ISO 14230-2 timing overrides, in milliseconds. An unset value
/// uses the standard default.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, schemars::JsonSchema)]
pub struct KLineTimingConfig {
    /// P2max: request to start of response, in milliseconds.
    #[serde(default)]
    pub p2_max_ms: Option<u64>,
    /// P3min: end of response to next request, in milliseconds.
    #[serde(default)]
    pub p3_min_ms: Option<u64>,
    /// P4min: inter-byte time of a tester request, in milliseconds.
    #[serde(default)]
    pub p4_min_ms: Option<u64>,
}

const DEFAULT_RESPONSE_TIMEOUT_MS: u64 = 5000;
const DEFAULT_PROBE_TIMEOUT_MS: u64 = 500;
const DEFAULT_KEEPALIVE_INTERVAL_MS: u64 = 2000;

fn default_response_timeout_ms() -> u64 {
    DEFAULT_RESPONSE_TIMEOUT_MS
}

fn default_probe_timeout_ms() -> u64 {
    DEFAULT_PROBE_TIMEOUT_MS
}

fn default_keepalive_interval_ms() -> u64 {
    DEFAULT_KEEPALIVE_INTERVAL_MS
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_KEEPALIVE_INTERVAL_MS, DEFAULT_PROBE_TIMEOUT_MS, DEFAULT_RESPONSE_TIMEOUT_MS,
        KLineConfig, KLineInitMode,
    };

    #[test]
    fn minimal_kline_section_deserializes_with_defaults() {
        let config: KLineConfig = toml::from_str(r#"interface = "COM3""#).expect("minimal config");

        assert_eq!(config.interface, "COM3");
        assert_eq!(config.init_mode, KLineInitMode::FastInit);
        assert_eq!(config.response_timeout_ms, DEFAULT_RESPONSE_TIMEOUT_MS);
        assert_eq!(config.probe_timeout_ms, DEFAULT_PROBE_TIMEOUT_MS);
        assert_eq!(config.keepalive_interval_ms, DEFAULT_KEEPALIVE_INTERVAL_MS);
        assert!(config.ecu_mappings.is_empty());
        assert!(config.transport_overrides.is_empty());
    }

    #[test]
    fn interface_is_required() {
        assert!(toml::from_str::<KLineConfig>(r#"init_mode = "fast_init""#).is_err());
    }

    #[test]
    fn interface_takes_every_platform_port_name_verbatim() {
        for port in [
            "/dev/ttyUSB0",
            "/dev/cu.usbserial-A10KZ3Y1",
            "COM3",
            r"\\.\COM12",
        ] {
            let config: KLineConfig = toml::from_str(&format!("interface = '{port}'"))
                .expect("config with a serial port name");
            assert_eq!(config.interface, port);
        }
    }

    #[test]
    fn init_mode_parses_snake_case() {
        let config: KLineConfig =
            toml::from_str("interface = \"COM3\"\ninit_mode = \"five_baud_init\"")
                .expect("config with init mode");

        assert_eq!(config.init_mode, KLineInitMode::FiveBaudInit);
    }
}
