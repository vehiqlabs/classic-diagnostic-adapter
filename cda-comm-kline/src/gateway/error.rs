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

/// Errors while setting up the K-Line gateway.
#[derive(Error, Debug, Clone)]
pub enum KLineGatewaySetupError {
    #[error("Failed to open K-Line interface `{0}`: {1}")]
    InterfaceOpenFailed(String, String),

    #[error("Invalid K-Line configuration: {0}")]
    InvalidConfiguration(String),
}

/// Errors on the K-Line link.
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

#[cfg(test)]
mod tests {
    use super::{KLineError, KLineGatewaySetupError};

    #[test]
    fn errors_name_their_cause() {
        assert_eq!(
            KLineGatewaySetupError::InterfaceOpenFailed("COM3".to_owned(), "busy".to_owned())
                .to_string(),
            "Failed to open K-Line interface `COM3`: busy"
        );
        assert_eq!(
            KLineGatewaySetupError::InvalidConfiguration("empty".to_owned()).to_string(),
            "Invalid K-Line configuration: empty"
        );
        assert_eq!(
            KLineError::ChecksumMismatch {
                expected: 0x26,
                actual: 0x00,
            }
            .to_string(),
            "Checksum mismatch: expected 0x26, got 0x00"
        );
        for (error, text) in [
            (KLineError::NoResponse, "No response from the ECU"),
            (KLineError::Timeout, "Timeout waiting for response"),
            (
                KLineError::EchoMismatch,
                "Echo mismatch on the single-wire bus",
            ),
            (
                KLineError::Framing("short".to_owned()),
                "Framing error: short",
            ),
            (
                KLineError::Init("no key bytes".to_owned()),
                "Initialization failed: no key bytes",
            ),
            (
                KLineError::Serial("closed".to_owned()),
                "Serial I/O error: closed",
            ),
        ] {
            assert_eq!(error.to_string(), text);
        }
    }
}
