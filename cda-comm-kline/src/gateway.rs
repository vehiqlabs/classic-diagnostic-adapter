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

/// ISO 14230-2 framing: header, checksum and echo check.
pub mod data_link;
/// Errors of the K-Line gateway and link.
pub mod error;
/// Link initialization helpers.
pub mod init;
/// ISO 14230-2 timing parameters.
pub mod timing;
mod topology;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use cda_interfaces::{
    DiagServiceError, EcuAddresses, EcuConnectivityHandler, FunctionalTransport, HashMap,
    NetworkTopology, PhysicalTransport, RouteStatus, ServicePayload, Shutdown,
    TransmissionParameters, TransportProbe, TransportResponse, VariantDetectionSender,
    communication_control::{
        TransportControl, TransportState, TransportStateTracker, error::CommControlError,
    },
};
use tokio::sync::mpsc;

pub use self::topology::{KLineEcuAddressing, KLineTopology, derive_kline_topology};
use self::{error::KLineGatewaySetupError, timing::KLineTiming};
use crate::config::{KLineConfig, KLineInitMode};

/// K-Line diagnostic gateway owning one serial interface. Half-duplex with one
/// active ECU per init: requests are serialized and the link is (re)initialized
/// on demand.
pub struct KLineDiagGateway {
    interface: String,
    topology: Arc<KLineTopology>,
    init_mode: KLineInitMode,
    response_timeout: Duration,
    probe_timeout: Duration,
    keepalive_interval: Duration,
    timing: KLineTiming,
    variant_detection: VariantDetectionSender,
    connectivity_handler: Arc<dyn EcuConnectivityHandler>,
    lifecycle: Arc<TransportStateTracker>,
}

impl KLineDiagGateway {
    /// Builds the passive gateway over the addressing from
    /// [`derive_kline_topology`]; no serial port is opened until `enable()`.
    ///
    /// # Errors
    /// Returns [`KLineGatewaySetupError::InvalidConfiguration`] for an empty
    /// interface.
    pub fn build(
        config: &KLineConfig,
        topology: KLineTopology,
        variant_detection: VariantDetectionSender,
        connectivity_handler: Arc<dyn EcuConnectivityHandler>,
    ) -> Result<KLineDiagGateway, KLineGatewaySetupError> {
        if config.interface.trim().is_empty() {
            return Err(KLineGatewaySetupError::InvalidConfiguration(
                "kline.interface must name the serial port of the K-Line adapter".to_owned(),
            ));
        }
        tracing::info!(interface = %config.interface, "Initializing KLineDiagGateway");
        Ok(Self {
            interface: config.interface.clone(),
            topology: Arc::new(topology),
            init_mode: config.init_mode,
            response_timeout: Duration::from_millis(config.response_timeout_ms),
            probe_timeout: Duration::from_millis(config.probe_timeout_ms),
            keepalive_interval: Duration::from_millis(config.keepalive_interval_ms),
            timing: KLineTiming::from_config(&config.timing),
            variant_detection,
            connectivity_handler,
            lifecycle: Arc::new(TransportStateTracker::new(TransportState::Disabled)),
        })
    }

    /// The K-Line addressing of the ECU, looked up by its name
    /// (case-insensitive).
    #[must_use]
    pub fn addressing(&self, ecu_name: &str) -> Option<&KLineEcuAddressing> {
        self.topology.get(&ecu_name.to_lowercase())
    }

    /// The KWP target address of the ECU, looked up by its name
    /// (case-insensitive).
    #[must_use]
    pub fn target_address(&self, ecu_name: &str) -> Option<u8> {
        self.addressing(ecu_name)
            .map(|addressing| addressing.target)
    }

    /// The serial port of the K-Line adapter.
    #[must_use]
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// How the link is initialized.
    #[must_use]
    pub fn init_mode(&self) -> KLineInitMode {
        self.init_mode
    }

    /// How long to wait for a response.
    #[must_use]
    pub fn response_timeout(&self) -> Duration {
        self.response_timeout
    }

    /// How long to wait for the init handshake when probing an ECU.
    #[must_use]
    pub fn probe_timeout(&self) -> Duration {
        self.probe_timeout
    }

    /// Interval of the `TesterPresent` keep-alive, zero when disabled.
    #[must_use]
    pub fn keepalive_interval(&self) -> Duration {
        self.keepalive_interval
    }

    /// The resolved link timing.
    #[must_use]
    pub fn timing(&self) -> &KLineTiming {
        &self.timing
    }

    /// The channel that requests variant detection for an ECU.
    #[must_use]
    pub fn variant_detection(&self) -> &VariantDetectionSender {
        &self.variant_detection
    }

    /// The handler told about gateway connects and disconnects and ECU replies.
    #[must_use]
    pub fn connectivity_handler(&self) -> &Arc<dyn EcuConnectivityHandler> {
        &self.connectivity_handler
    }
}

impl Clone for KLineDiagGateway {
    fn clone(&self) -> Self {
        Self {
            interface: self.interface.clone(),
            topology: Arc::clone(&self.topology),
            init_mode: self.init_mode,
            response_timeout: self.response_timeout,
            probe_timeout: self.probe_timeout,
            keepalive_interval: self.keepalive_interval,
            timing: self.timing,
            variant_detection: self.variant_detection.clone(),
            connectivity_handler: Arc::clone(&self.connectivity_handler),
            lifecycle: Arc::clone(&self.lifecycle),
        }
    }
}

impl PhysicalTransport for KLineDiagGateway {
    fn send(
        &self,
        transmission_params: TransmissionParameters,
        _message: ServicePayload,
        _response_sender: mpsc::Sender<Result<Option<TransportResponse>, DiagServiceError>>,
        _expect_uds_reply: bool,
    ) -> impl Future<Output = Result<tokio::task::JoinHandle<()>, DiagServiceError>> + Send {
        std::future::ready(
            if self.target_address(&transmission_params.ecu_name).is_none() {
                Err(DiagServiceError::EcuOffline(transmission_params.ecu_name))
            } else {
                Err(DiagServiceError::RequestNotSupported(
                    "K-Line physical send is not implemented yet".to_owned(),
                ))
            },
        )
    }

    fn ecu_online<E: EcuAddresses>(
        &self,
        ecu_name: &str,
        _ecu_db: &tokio::sync::RwLock<E>,
    ) -> impl Future<Output = Result<(), DiagServiceError>> + Send {
        std::future::ready(Err(DiagServiceError::EcuOffline(ecu_name.to_owned())))
    }
}

impl FunctionalTransport for KLineDiagGateway {
    fn send_functional(
        &self,
        _transmission_params: TransmissionParameters,
        _message: ServicePayload,
        _expected_ecu_logical_addrs: HashMap<u16, String>,
        _timeout: Duration,
        _expect_positive_response: bool,
    ) -> impl Future<
        Output = Result<
            HashMap<String, Result<ServicePayload, DiagServiceError>>,
            DiagServiceError,
        >,
    > + Send {
        std::future::ready(Err(DiagServiceError::RequestNotSupported(
            "K-Line functional addressing is not supported".to_owned(),
        )))
    }
}

impl NetworkTopology for KLineDiagGateway {
    fn get_gateway_network_address(
        &self,
        _logical_address: u16,
    ) -> impl Future<Output = Option<String>> + Send {
        std::future::ready(None)
    }

    fn get_ecu_network_address(
        &self,
        ecu_name: &str,
    ) -> impl Future<Output = Option<String>> + Send {
        std::future::ready(
            self.target_address(ecu_name)
                .map(|target| format!("{}:{target}", self.interface)),
        )
    }
}

impl TransportProbe for KLineDiagGateway {
    fn route_status(&self, ecu_name: &str) -> impl Future<Output = RouteStatus> + Send {
        std::future::ready(if self.target_address(ecu_name).is_some() {
            RouteStatus::ProbeRequired
        } else {
            RouteStatus::NotConfigured
        })
    }

    fn probe_ecu(&self, _ecu_name: &str) -> impl Future<Output = bool> + Send {
        std::future::ready(false)
    }
}

#[async_trait]
impl Shutdown for KLineDiagGateway {
    async fn shutdown(&self) {
        self.lifecycle.transition(TransportState::Disabled).await;
    }
}

#[async_trait]
impl TransportControl for KLineDiagGateway {
    async fn enable(&self) -> Result<(), CommControlError> {
        self.lifecycle.transition(TransportState::Enabled).await;
        Ok(())
    }

    async fn disable(&self) -> Result<(), CommControlError> {
        self.lifecycle.transition(TransportState::Disabled).await;
        Ok(())
    }

    async fn state(&self) -> TransportState {
        self.lifecycle.state().await
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use cda_interfaces::{
        DiagServiceError, EcuAddresses, EcuConnectivityHandler, FunctionalTransport,
        GatewayConnectReport, HashMap, NetworkTopology, PhysicalTransport, RouteStatus,
        ServicePayload, Shutdown, TransmissionParameters, TransportProbe, VariantDetectionRequest,
        VariantDetectionSender,
        communication_control::{TransportControl, TransportState},
    };

    use super::{
        KLineDiagGateway, KLineEcuAddressing, KLineTopology, error::KLineGatewaySetupError,
    };
    use crate::config::KLineConfig;

    struct NoConnectivity;

    #[async_trait::async_trait]
    impl EcuConnectivityHandler for NoConnectivity {
        async fn on_gateway_connected(&self, _ecu_names: &[String]) -> GatewayConnectReport {
            GatewayConnectReport::default()
        }

        async fn on_gateway_disconnected(&self, _ecu_names: &[String]) {}

        async fn on_ecu_replied(&self, _ecu_name: &str) {}
    }

    fn config(toml_text: &str) -> KLineConfig {
        toml::from_str(toml_text).expect("valid K-Line config")
    }

    fn build(
        config: &KLineConfig,
        topology: KLineTopology,
    ) -> Result<KLineDiagGateway, KLineGatewaySetupError> {
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        KLineDiagGateway::build(
            config,
            topology,
            VariantDetectionSender::new(sender),
            Arc::new(NoConnectivity),
        )
    }

    fn mapped_gateway() -> KLineDiagGateway {
        build(
            &config(r#"interface = "COM3""#),
            KLineTopology::from_iter([(
                "enginekwp".to_owned(),
                KLineEcuAddressing {
                    target: 0x10,
                    functional: None,
                    tester: Some(0xF1),
                    baud_rate: None,
                },
            )]),
        )
        .expect("gateway with one ECU")
    }

    fn params(ecu_name: &str) -> TransmissionParameters {
        TransmissionParameters {
            gateway_address: 0x0E80,
            timeout_ack: Duration::from_millis(100),
            ecu_name: ecu_name.to_owned(),
            repeat_request_count_transmission: 0,
        }
    }

    fn tester_present() -> ServicePayload {
        ServicePayload {
            data: vec![0x3E, 0x00],
            source_address: 0x0E80,
            target_address: 0x0E80,
            new_session: None,
            new_security: None,
        }
    }

    #[test]
    fn build_rejects_an_empty_interface() {
        assert!(matches!(
            build(&config(r#"interface = "  ""#), KLineTopology::default()),
            Err(KLineGatewaySetupError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn target_address_resolves_from_the_ecu_name() {
        let gateway = mapped_gateway();
        assert_eq!(gateway.target_address("ENGINEKWP"), Some(0x10));
        assert_eq!(
            gateway
                .addressing("EngineKwp")
                .and_then(|addressing| addressing.tester),
            Some(0xF1)
        );
        assert_eq!(gateway.target_address("Unmapped"), None);
    }

    #[tokio::test]
    async fn only_ecus_with_addressing_are_routable() {
        let gateway = mapped_gateway();
        assert_eq!(
            gateway.route_status("EngineKwp").await,
            RouteStatus::ProbeRequired
        );
        assert_eq!(
            gateway.route_status("Unmapped").await,
            RouteStatus::NotConfigured
        );
        assert_eq!(
            gateway.get_ecu_network_address("EngineKwp").await,
            Some("COM3:16".to_owned())
        );
        assert_eq!(gateway.get_ecu_network_address("Unmapped").await, None);
    }

    #[tokio::test]
    async fn send_resolves_the_target_from_the_ecu_name_not_the_gateway_address() {
        let gateway = mapped_gateway();
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        assert!(matches!(
            gateway
                .send(params("EngineKwp"), tester_present(), sender.clone(), true)
                .await,
            Err(DiagServiceError::RequestNotSupported(_))
        ));
        assert!(matches!(
            gateway
                .send(params("Unmapped"), tester_present(), sender, true)
                .await,
            Err(DiagServiceError::EcuOffline(_))
        ));
    }

    struct FakeEcu;

    impl EcuAddresses for FakeEcu {
        fn tester_address(&self) -> u16 {
            0x00F1
        }

        fn logical_address(&self) -> u16 {
            0x0010
        }

        fn logical_gateway_address(&self) -> u16 {
            0x0010
        }

        fn logical_functional_address(&self) -> u16 {
            0x0033
        }

        fn ecu_name(&self) -> String {
            "EngineKwp".to_owned()
        }

        fn logical_address_eq<T: EcuAddresses>(&self, other: &T) -> bool {
            self.logical_address() == other.logical_address()
        }
    }

    #[tokio::test]
    async fn the_gateway_keeps_the_configured_link_settings_and_channels() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let connectivity: Arc<dyn EcuConnectivityHandler> = Arc::new(NoConnectivity);
        let gateway = KLineDiagGateway::build(
            &config(
                r#"
interface = "/dev/ttyUSB0"
init_mode = "five_baud_init"
response_timeout_ms = 2000
probe_timeout_ms = 300
keepalive_interval_ms = 1500

[timing]
p2_max_ms = 80
"#,
            ),
            KLineTopology::default(),
            VariantDetectionSender::new(sender),
            Arc::clone(&connectivity),
        )
        .expect("gateway");

        assert_eq!(gateway.interface(), "/dev/ttyUSB0");
        assert_eq!(
            gateway.init_mode(),
            crate::config::KLineInitMode::FiveBaudInit
        );
        assert_eq!(gateway.response_timeout(), Duration::from_millis(2000));
        assert_eq!(gateway.probe_timeout(), Duration::from_millis(300));
        assert_eq!(gateway.keepalive_interval(), Duration::from_millis(1500));
        assert_eq!(gateway.timing().p2_max, Duration::from_millis(80));
        assert!(Arc::ptr_eq(gateway.connectivity_handler(), &connectivity));

        gateway
            .variant_detection()
            .send(VariantDetectionRequest::new(vec!["EngineKwp".to_owned()]))
            .await
            .expect("send through the gateway's variant-detection channel");
        assert!(receiver.recv().await.is_some());
    }

    #[tokio::test]
    async fn the_skeleton_answers_without_serial_io() {
        let gateway = mapped_gateway();

        assert!(matches!(
            gateway
                .ecu_online("EngineKwp", &tokio::sync::RwLock::new(FakeEcu))
                .await,
            Err(DiagServiceError::EcuOffline(_))
        ));
        assert!(matches!(
            gateway
                .send_functional(
                    params("EngineKwp"),
                    tester_present(),
                    HashMap::default(),
                    Duration::from_millis(100),
                    true,
                )
                .await,
            Err(DiagServiceError::RequestNotSupported(_))
        ));
        assert_eq!(gateway.get_gateway_network_address(0x0010).await, None);
        assert!(!gateway.probe_ecu("EngineKwp").await);
    }

    #[tokio::test]
    async fn clones_share_the_transport_lifecycle() {
        let gateway = mapped_gateway();
        let clone = gateway.clone();
        assert_eq!(clone.target_address("EngineKwp"), Some(0x10));
        assert_eq!(gateway.state().await, TransportState::Disabled);

        clone.enable().await.expect("enable");
        assert_eq!(gateway.state().await, TransportState::Enabled);

        gateway.disable().await.expect("disable");
        assert_eq!(clone.state().await, TransportState::Disabled);

        clone.enable().await.expect("enable");
        gateway.shutdown().await;
        assert_eq!(clone.state().await, TransportState::Disabled);
    }
}
