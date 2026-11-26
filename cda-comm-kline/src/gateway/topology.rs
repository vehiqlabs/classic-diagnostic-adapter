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

use cda_interfaces::{HashMap, KLineComParamProvider};
use tokio::sync::RwLock;

use super::error::KLineGatewaySetupError;
use crate::config::KLineConfig;

/// K-Line addressing of one ECU, resolved once at setup.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KLineEcuAddressing {
    /// Physical target address byte carried in the ISO 14230-2 header.
    pub target: u8,
    /// Functional target address byte, when known.
    pub functional: Option<u8>,
    /// Source address byte of the tester, when the MDD names one.
    pub tester: Option<u8>,
    /// Link baud rate; `None` uses the transport default.
    pub baud_rate: Option<u32>,
}

/// K-Line addressing of every routable ECU, keyed by lowercase ECU name.
pub type KLineTopology = HashMap<String, KLineEcuAddressing>;

/// Resolves the K-Line addressing of the ECUs: an explicit
/// `[[kline.ecu_mappings]]` entry first, else the MDD com-params of the ECU.
/// A mapping leaves the values it does not set to the MDD. ECUs with neither
/// are not routable over K-Line.
///
/// # Errors
/// Returns [`KLineGatewaySetupError::InvalidConfiguration`] for two mappings
/// of the same ECU name.
#[allow(
    clippy::implicit_hasher,
    reason = "Type alias doesn't allow specifying hasher"
)]
pub async fn derive_kline_topology<T: KLineComParamProvider>(
    config: &KLineConfig,
    ecus: &HashMap<String, RwLock<T>>,
) -> Result<KLineTopology, KLineGatewaySetupError> {
    let mut topology = KLineTopology::default();
    for mapping in &config.ecu_mappings {
        let ecu_name = mapping.ecu_name.to_lowercase();
        let (functional, tester, baud_rate) = match ecus.get(&ecu_name) {
            Some(ecu_lock) => {
                let ecu = ecu_lock.read().await;
                (
                    ecu.kline_functional_address(),
                    ecu.kline_tester_address(),
                    ecu.kline_baud_rate(),
                )
            }
            None => (None, None, None),
        };
        let addressing = KLineEcuAddressing {
            target: mapping.physical_address,
            functional: mapping.functional_address.or(functional),
            tester,
            baud_rate: mapping.baud_rate.or(baud_rate),
        };
        if topology.insert(ecu_name, addressing).is_some() {
            return Err(KLineGatewaySetupError::InvalidConfiguration(format!(
                "Duplicate mapping for ECU '{}' in kline.ecu_mappings",
                mapping.ecu_name
            )));
        }
    }
    for (name, ecu_lock) in ecus {
        let ecu = ecu_lock.read().await;
        let Some(target) = ecu.kline_physical_address() else {
            continue;
        };
        topology
            .entry(name.to_lowercase())
            .or_insert(KLineEcuAddressing {
                target,
                functional: ecu.kline_functional_address(),
                tester: ecu.kline_tester_address(),
                baud_rate: ecu.kline_baud_rate(),
            });
    }
    Ok(topology)
}

#[cfg(test)]
mod tests {
    use cda_interfaces::{HashMap, KLineComParamProvider};
    use tokio::sync::RwLock;

    use super::{KLineEcuAddressing, derive_kline_topology};
    use crate::{config::KLineConfig, gateway::error::KLineGatewaySetupError};

    #[derive(Default)]
    struct FakeEcu {
        physical: Option<u8>,
        functional: Option<u8>,
        tester: Option<u8>,
        baud_rate: Option<u32>,
    }

    impl KLineComParamProvider for FakeEcu {
        fn kline_physical_address(&self) -> Option<u8> {
            self.physical
        }

        fn kline_functional_address(&self) -> Option<u8> {
            self.functional
        }

        fn kline_tester_address(&self) -> Option<u8> {
            self.tester
        }

        fn kline_baud_rate(&self) -> Option<u32> {
            self.baud_rate
        }
    }

    fn config(toml_text: &str) -> KLineConfig {
        toml::from_str(toml_text).expect("valid K-Line config")
    }

    fn ecus(entries: Vec<(&str, FakeEcu)>) -> HashMap<String, RwLock<FakeEcu>> {
        entries
            .into_iter()
            .map(|(name, ecu)| (name.to_owned(), RwLock::new(ecu)))
            .collect()
    }

    fn mdd_ecu() -> FakeEcu {
        FakeEcu {
            physical: Some(0x11),
            functional: Some(0x33),
            tester: Some(0xF1),
            baud_rate: Some(9600),
        }
    }

    #[tokio::test]
    async fn an_ecu_with_kline_com_params_is_routable_without_a_mapping() {
        let topology = derive_kline_topology(
            &config(r#"interface = "COM3""#),
            &ecus(vec![
                ("enginekwp", mdd_ecu()),
                ("doiponly", FakeEcu::default()),
            ]),
        )
        .await
        .expect("topology");

        assert_eq!(
            topology.get("enginekwp"),
            Some(&KLineEcuAddressing {
                target: 0x11,
                functional: Some(0x33),
                tester: Some(0xF1),
                baud_rate: Some(9600),
            })
        );
        assert_eq!(topology.get("doiponly"), None);
    }

    #[tokio::test]
    async fn a_mapping_overrides_the_com_params_it_sets() {
        let topology = derive_kline_topology(
            &config(
                r#"
interface = "/dev/ttyUSB0"

[[ecu_mappings]]
ecu_name = "EngineKwp"
physical_address = 0x10
baud_rate = 10400
"#,
            ),
            &ecus(vec![("enginekwp", mdd_ecu())]),
        )
        .await
        .expect("topology");

        assert_eq!(
            topology.get("enginekwp"),
            Some(&KLineEcuAddressing {
                target: 0x10,
                functional: Some(0x33),
                tester: Some(0xF1),
                baud_rate: Some(10400),
            })
        );
    }

    #[tokio::test]
    async fn a_mapping_without_a_database_is_routable() {
        let topology = derive_kline_topology(
            &config(
                r#"
interface = "COM3"

[[ecu_mappings]]
ecu_name = "EngineKwp"
physical_address = 0x10
"#,
            ),
            &ecus(Vec::new()),
        )
        .await
        .expect("topology");

        assert_eq!(
            topology
                .get("enginekwp")
                .map(|addressing| addressing.target),
            Some(0x10)
        );
    }

    #[tokio::test]
    async fn a_duplicate_ecu_mapping_is_rejected() {
        let duplicate = config(
            r#"
interface = "/dev/cu.usbserial-0001"

[[ecu_mappings]]
ecu_name = "EngineKwp"
physical_address = 0x10

[[ecu_mappings]]
ecu_name = "enginekwp"
physical_address = 0x11
"#,
        );
        assert!(matches!(
            derive_kline_topology(&duplicate, &ecus(Vec::new())).await,
            Err(KLineGatewaySetupError::InvalidConfiguration(_))
        ));
    }
}
