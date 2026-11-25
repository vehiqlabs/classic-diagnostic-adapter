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

use thiserror::Error;

/// Errors that can occur during K-Line gateway setup.
#[derive(Error, Debug, Clone)]
pub enum KLineGatewaySetupError {
    #[error("Failed to open K-Line interface `{0}`: {1}")]
    InterfaceOpenFailed(String, String),

    #[error("Invalid K-Line configuration: {0}")]
    InvalidConfiguration(String),
}

/// Errors that can occur during K-Line communication.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum KLineError {
    #[error("No response from the ECU")]
    NoResponse,

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Checksum mismatch: expected {expected:#04X}, got {actual:#04X}")]
    ChecksumMismatch { expected: u8, actual: u8 },

    #[error("Echo mismatch on the single-wire bus")]
    EchoMismatch,

    #[error("Framing error: {0}")]
    Framing(String),

    #[error("Initialization failed: {0}")]
    Init(String),

    #[error("Serial I/O error: {0}")]
    Serial(String),
}
