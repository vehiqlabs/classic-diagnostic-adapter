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

pub mod data_link;
pub mod error;
pub mod init;
pub mod timing;

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

use self::{error::KLineGatewaySetupError, timing::KLineTiming};
use crate::config::{KLineConfig, KLineInitMode};

/// K-Line diagnostic gateway owning one serial interface. Half-duplex with one
/// active ECU per init: requests are serialized and the link is (re)initialized
/// on demand.
pub struct KLineDiagGateway {
    interface: String,
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
    /// Builds the passive gateway; no serial port is opened until `enable()`.
    ///
    /// # Errors
    /// Returns [`KLineGatewaySetupError`] if the configuration is unusable.
    pub fn build(
        config: &KLineConfig,
        variant_detection: VariantDetectionSender,
        connectivity_handler: Arc<dyn EcuConnectivityHandler>,
    ) -> Result<KLineDiagGateway, KLineGatewaySetupError> {
        if config.interface.trim().is_empty() {
            return Err(KLineGatewaySetupError::InvalidConfiguration(
                "kline.interface must not be empty".to_owned(),
            ));
        }
        tracing::info!(interface = %config.interface, "Initializing KLineDiagGateway");
        Ok(Self {
            interface: config.interface.clone(),
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

    #[must_use]
    pub fn interface(&self) -> &str {
        &self.interface
    }

    #[must_use]
    pub fn init_mode(&self) -> KLineInitMode {
        self.init_mode
    }

    #[must_use]
    pub fn response_timeout(&self) -> Duration {
        self.response_timeout
    }

    #[must_use]
    pub fn probe_timeout(&self) -> Duration {
        self.probe_timeout
    }

    #[must_use]
    pub fn keepalive_interval(&self) -> Duration {
        self.keepalive_interval
    }

    #[must_use]
    pub fn timing(&self) -> &KLineTiming {
        &self.timing
    }

    #[must_use]
    pub fn variant_detection(&self) -> &VariantDetectionSender {
        &self.variant_detection
    }

    #[must_use]
    pub fn connectivity_handler(&self) -> &Arc<dyn EcuConnectivityHandler> {
        &self.connectivity_handler
    }
}

impl Clone for KLineDiagGateway {
    fn clone(&self) -> Self {
        Self {
            interface: self.interface.clone(),
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
    async fn send(
        &self,
        _transmission_params: TransmissionParameters,
        _message: ServicePayload,
        _response_sender: mpsc::Sender<Result<Option<TransportResponse>, DiagServiceError>>,
        _expect_uds_reply: bool,
    ) -> Result<tokio::task::JoinHandle<()>, DiagServiceError> {
        Err(DiagServiceError::RequestNotSupported(
            "K-Line physical send is not implemented yet".to_owned(),
        ))
    }

    async fn ecu_online<E: EcuAddresses>(
        &self,
        ecu_name: &str,
        _ecu_db: &tokio::sync::RwLock<E>,
    ) -> Result<(), DiagServiceError> {
        Err(DiagServiceError::EcuOffline(ecu_name.to_owned()))
    }
}

impl FunctionalTransport for KLineDiagGateway {
    async fn send_functional(
        &self,
        _transmission_params: TransmissionParameters,
        _message: ServicePayload,
        _expected_ecu_logical_addrs: HashMap<u16, String>,
        _timeout: Duration,
        _expect_positive_response: bool,
    ) -> Result<HashMap<String, Result<ServicePayload, DiagServiceError>>, DiagServiceError> {
        Err(DiagServiceError::RequestNotSupported(
            "K-Line functional addressing is not supported".to_owned(),
        ))
    }
}

impl NetworkTopology for KLineDiagGateway {
    async fn get_gateway_network_address(&self, _logical_address: u16) -> Option<String> {
        None
    }

    async fn get_ecu_network_address(&self, _ecu_name: &str) -> Option<String> {
        None
    }
}

impl TransportProbe for KLineDiagGateway {
    async fn route_status(&self, _ecu_name: &str) -> RouteStatus {
        RouteStatus::ProbeRequired
    }

    async fn probe_ecu(&self, _ecu_name: &str) -> bool {
        // TODO: perform the init handshake; until then no ECU binds to K-Line.
        false
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
