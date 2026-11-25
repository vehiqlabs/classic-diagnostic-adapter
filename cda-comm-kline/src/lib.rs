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

//! K-Line (ISO 14230 / ISO 9141-2) communication layer for the CDA.

/// The `[kline]` configuration section.
pub mod config;

#[cfg(feature = "kline")]
mod gateway;
#[cfg(not(feature = "kline"))]
use cda_interfaces::{
    DiagServiceError, EcuAddresses, FunctionalTransport, HashMap, NetworkTopology,
    PhysicalTransport, RouteStatus, ServicePayload, Shutdown, TransmissionParameters,
    TransportProbe, TransportResponse,
    communication_control::{TransportControl, TransportState, error::CommControlError},
};
#[cfg(feature = "kline")]
pub use gateway::{
    KLineDiagGateway, KLineEcuAddressing, KLineTopology, data_link, derive_kline_topology, error,
    init, timing,
};

/// Uninhabited stub for builds without the `kline` feature; its trait impls
/// only satisfy the router's bounds.
#[cfg(not(feature = "kline"))]
#[derive(Clone)]
pub enum KLineDiagGateway {}

#[cfg(not(feature = "kline"))]
impl PhysicalTransport for KLineDiagGateway {
    fn send(
        &self,
        _transmission_params: TransmissionParameters,
        _message: ServicePayload,
        _response_sender: tokio::sync::mpsc::Sender<
            Result<Option<TransportResponse>, DiagServiceError>,
        >,
        _expect_uds_reply: bool,
    ) -> impl Future<Output = Result<tokio::task::JoinHandle<()>, DiagServiceError>> + Send {
        std::future::ready(Err(DiagServiceError::EcuOffline(
            "K-Line support is not enabled. Compile with the `kline` feature.".to_owned(),
        )))
    }

    fn ecu_online<E: EcuAddresses>(
        &self,
        _ecu_name: &str,
        _ecu_db: &tokio::sync::RwLock<E>,
    ) -> impl Future<Output = Result<(), DiagServiceError>> + Send {
        std::future::ready(Err(DiagServiceError::EcuOffline(
            "K-Line support is not enabled. Compile with the `kline` feature.".to_owned(),
        )))
    }
}

#[cfg(not(feature = "kline"))]
impl NetworkTopology for KLineDiagGateway {
    fn get_gateway_network_address(
        &self,
        _logical_address: u16,
    ) -> impl Future<Output = Option<String>> + Send {
        std::future::ready(None)
    }

    fn get_ecu_network_address(
        &self,
        _ecu_name: &str,
    ) -> impl Future<Output = Option<String>> + Send {
        std::future::ready(None)
    }
}

#[cfg(not(feature = "kline"))]
impl FunctionalTransport for KLineDiagGateway {
    fn send_functional(
        &self,
        _transmission_params: TransmissionParameters,
        _message: ServicePayload,
        _expected_ecu_logical_addrs: HashMap<u16, String>,
        _timeout: std::time::Duration,
        _expect_positive_response: bool,
    ) -> impl Future<
        Output = Result<
            HashMap<String, Result<ServicePayload, DiagServiceError>>,
            DiagServiceError,
        >,
    > + Send {
        std::future::ready(Err(DiagServiceError::RequestNotSupported(
            "K-Line functional addressing is not available because K-Line support is disabled."
                .to_owned(),
        )))
    }
}

#[cfg(not(feature = "kline"))]
#[async_trait::async_trait]
impl TransportControl for KLineDiagGateway {
    async fn enable(&self) -> Result<(), CommControlError> {
        match *self {}
    }

    async fn disable(&self) -> Result<(), CommControlError> {
        match *self {}
    }

    async fn state(&self) -> TransportState {
        match *self {}
    }
}

#[cfg(not(feature = "kline"))]
impl TransportProbe for KLineDiagGateway {
    fn route_status(&self, _ecu_name: &str) -> impl Future<Output = RouteStatus> + Send {
        std::future::ready(RouteStatus::NotConfigured)
    }

    fn probe_ecu(&self, _ecu_name: &str) -> impl Future<Output = bool> + Send {
        std::future::ready(false)
    }
}

#[cfg(not(feature = "kline"))]
#[async_trait::async_trait]
impl Shutdown for KLineDiagGateway {
    async fn shutdown(&self) {}
}
