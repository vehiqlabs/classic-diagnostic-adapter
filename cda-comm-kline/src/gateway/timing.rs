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

use std::time::Duration;

use crate::config::KLineTimingConfig;

/// Resolved ISO 14230-2 timing parameters of one K-Line link that the
/// configuration can override.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KLineTiming {
    /// P2max: latest start of the ECU response after a request.
    pub p2_max: Duration,
    /// P3min: earliest next request after the end of a response.
    pub p3_min: Duration,
    /// P4min: minimum gap between the bytes of a tester request.
    pub p4_min: Duration,
}

impl Default for KLineTiming {
    fn default() -> Self {
        // ISO 14230-2 default timing values.
        Self {
            p2_max: Duration::from_millis(50),
            p3_min: Duration::from_millis(55),
            p4_min: Duration::from_millis(5),
        }
    }
}

impl KLineTiming {
    /// Applies the configured overrides to the standard defaults.
    #[must_use]
    pub fn from_config(config: &KLineTimingConfig) -> Self {
        let defaults = Self::default();
        Self {
            p2_max: config
                .p2_max_ms
                .map_or(defaults.p2_max, Duration::from_millis),
            p3_min: config
                .p3_min_ms
                .map_or(defaults.p3_min, Duration::from_millis),
            p4_min: config
                .p4_min_ms
                .map_or(defaults.p4_min, Duration::from_millis),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::KLineTiming;
    use crate::config::KLineTimingConfig;

    #[test]
    fn from_config_uses_defaults_when_unset() {
        let timing = KLineTiming::from_config(&KLineTimingConfig::default());
        assert_eq!(timing, KLineTiming::default());
    }

    #[test]
    fn from_config_applies_overrides() {
        let timing = KLineTiming::from_config(&KLineTimingConfig {
            p2_max_ms: Some(100),
            p3_min_ms: None,
            p4_min_ms: Some(10),
        });
        assert_eq!(timing.p2_max, Duration::from_millis(100));
        assert_eq!(timing.p3_min, KLineTiming::default().p3_min);
        assert_eq!(timing.p4_min, Duration::from_millis(10));
    }
}
