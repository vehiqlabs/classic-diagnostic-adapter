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

use super::error::KLineError;

/// ISO 14230-2 message header format. The format byte carries the addressing
/// mode and, in its low six bits, the length - or zero, when a separate length
/// byte follows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderFormat {
    NoAddressLengthInFormat,
    NoAddressSeparateLength,
    WithAddressLengthInFormat,
    WithAddressSeparateLength,
}

/// Header of an ISO 14230-2 message.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KLineHeader {
    pub format: HeaderFormat,
    /// Present only with the `WithAddress*` formats.
    pub target: Option<u8>,
    /// Present only with the `WithAddress*` formats.
    pub source: Option<u8>,
    pub length: u8,
}

impl KLineHeader {
    /// Header of a physically addressed message with the length in the
    /// format byte.
    #[must_use]
    pub fn physical(source: u8, target: u8, length: u8) -> Self {
        Self {
            format: HeaderFormat::WithAddressLengthInFormat,
            target: Some(target),
            source: Some(source),
            length,
        }
    }
}

/// ISO 14230-2 checksum: the 8-bit sum of every preceding byte, modulo 256.
#[must_use]
pub fn checksum(message: &[u8]) -> u8 {
    message
        .iter()
        .fold(0u8, |acc, &byte| acc.wrapping_add(byte))
}

/// # Errors
/// [`KLineError::Framing`] for an empty frame, [`KLineError::ChecksumMismatch`]
/// for a bad trailer.
pub fn verify_checksum(frame: &[u8]) -> Result<(), KLineError> {
    let (actual, body) = frame
        .split_last()
        .ok_or_else(|| KLineError::Framing("empty frame".to_owned()))?;
    let expected = checksum(body);
    if expected == *actual {
        Ok(())
    } else {
        Err(KLineError::ChecksumMismatch {
            expected,
            actual: *actual,
        })
    }
}

/// K-Line echoes every transmitted byte on the single wire; the sender reads
/// and checks that echo against what it sent.
///
/// # Errors
/// [`KLineError::EchoMismatch`] when the echo differs.
pub fn verify_echo(sent: &[u8], echo: &[u8]) -> Result<(), KLineError> {
    if sent == echo {
        Ok(())
    } else {
        Err(KLineError::EchoMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::{HeaderFormat, KLineError, KLineHeader, checksum, verify_checksum, verify_echo};

    #[test]
    fn a_physical_header_carries_both_addresses_and_the_length_in_the_format_byte() {
        assert_eq!(
            KLineHeader::physical(0xF1, 0x10, 2),
            KLineHeader {
                format: HeaderFormat::WithAddressLengthInFormat,
                target: Some(0x10),
                source: Some(0xF1),
                length: 2,
            }
        );
    }

    #[test]
    fn checksum_is_the_modulo_256_sum() {
        assert_eq!(checksum(&[0xC4, 0x33, 0xF1, 0x3E]), 0x26);
        assert_eq!(checksum(&[]), 0x00);
    }

    #[test]
    fn verify_checksum_accepts_a_matching_trailer() {
        let body = [0xC4, 0x33, 0xF1, 0x3E];
        let mut frame = body.to_vec();
        frame.push(checksum(&body));
        assert_eq!(verify_checksum(&frame), Ok(()));
    }

    #[test]
    fn verify_checksum_rejects_a_bad_trailer() {
        assert_eq!(
            verify_checksum(&[0x01, 0x02, 0x00]),
            Err(KLineError::ChecksumMismatch {
                expected: 0x03,
                actual: 0x00,
            })
        );
    }

    #[test]
    fn verify_checksum_rejects_an_empty_frame() {
        assert!(matches!(verify_checksum(&[]), Err(KLineError::Framing(_))));
    }

    #[test]
    fn verify_echo_matches_identical_bytes() {
        assert_eq!(verify_echo(&[0x81, 0x10], &[0x81, 0x10]), Ok(()));
        assert_eq!(
            verify_echo(&[0x81, 0x10], &[0x81, 0x11]),
            Err(KLineError::EchoMismatch)
        );
    }
}
