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

/// The two key bytes returned by the ECU during initialization; together they
/// identify its header and timing conventions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyBytes {
    pub kb1: u8,
    pub kb2: u8,
}

/// Bitwise complement of a byte. The init handshake runs on complements: the
/// tester echoes the complement of KB2, and the ECU replies with the complement
/// of the target address.
#[must_use]
pub const fn complement(byte: u8) -> u8 {
    !byte
}

#[cfg(test)]
mod tests {
    use super::complement;

    #[test]
    fn complement_inverts_every_bit() {
        assert_eq!(complement(0x33), 0xCC);
        assert_eq!(complement(0x00), 0xFF);
        assert_eq!(complement(complement(0xA5)), 0xA5);
    }
}
