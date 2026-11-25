.. SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
..
.. See the NOTICE file(s) distributed with this work for additional
.. information regarding copyright ownership.
..
.. This program and the accompanying materials are made available under the
.. terms of the Apache License Version 2.0 which is available at
.. https://www.apache.org/licenses/LICENSE-2.0
..
.. SPDX-License-Identifier: Apache-2.0

.. _architecture_kline_communication:

K-Line Communication
---------------------

The K-Line gateway transports UDS / KWP2000 messages over the single-wire
serial bus of ISO 9141-2 and ISO 14230 (KWP2000). It reaches the bus through a
serial port, typically a USB K-Line adapter. Linux, macOS and Windows all
expose such ports, so the transport is not tied to one operating system. The
``kline`` feature is required; without it the crate builds a stub gateway so
the transport router and ``cda-main`` compile in every feature combination, as
CAN does.

This page describes the first skeleton
(`eclipse-opensovd/classic-diagnostic-adapter#767 <https://github.com/eclipse-opensovd/classic-diagnostic-adapter/issues/767>`_,
moved from ``eclipse-opensovd/opensovd#144``, a follow-up to discussion
`eclipse-opensovd/opensovd#142 <https://github.com/eclipse-opensovd/opensovd/discussions/142>`_,
with CAN as the precedent in
`eclipse-opensovd/classic-diagnostic-adapter#323 <https://github.com/eclipse-opensovd/classic-diagnostic-adapter/pull/323>`_).
The gateway trait surface, the ECU address resolution and the data-link,
initialization and timing modules are present; the serial I/O and the real
framing, init and timing logic are left to follow-up work.

Crate And Configuration
^^^^^^^^^^^^^^^^^^^^^^^^^

The transport lives in the ``cda-comm-kline`` crate. It is configured with a
``[kline]`` section mirroring ``[can]``: a serial ``interface``, an
``init_mode`` (fast init or 5-baud init), response and probe timeouts, a
``TesterPresent`` keep-alive interval that must stay below P3max, optional
per-ECU ``ecu_mappings`` that override the addressing from the MDD and
``transport_overrides`` (``transport = "kline"`` pins an ECU). See
``opensovd-cda-kline.toml`` for a complete example.

``interface`` is required and has no default, because serial port names differ
per operating system:

- Linux: ``/dev/ttyUSB0``
- macOS: ``/dev/cu.usbserial-XXXX``
- Windows: ``COM3``

Addressing
^^^^^^^^^^

``derive_kline_topology`` resolves the K-Line addressing of every ECU at setup,
as ``derive_can_topology`` does for CAN: an explicit ``ecu_mappings`` entry
first, else the MDD com-params of the ECU (``CP_PhysReqTargetAddr``,
``CP_FuncReqTargetAddr``, ``CP_TesterSourceAddress`` and ``CP_Baudrate``,
configurable under ``[com_params.kline]``), which the ECU manager provides
through ``KLineComParamProvider``. A mapping leaves the values it does not set
to the MDD. The gateway looks the target address of a request up by ECU name;
``TransmissionParameters::gateway_address`` keeps its meaning and is not used
for K-Line addressing. An ECU with neither is reported as not configured on
K-Line, so the router does not bind it there.

Data Link, Initialization And Timing
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

The data link follows ISO 14230-2: header formats with and without address
bytes and with or without a separate length byte, the modulo-256 checksum, and
the byte echo of the single-wire bus. Initialization is either fast init (a
25 ms low / 25 ms high wake-up pattern then ``StartCommunication``) or 5-baud
init (the target address clocked out at 5 baud, the key-byte exchange and the
inverted address/key-byte handshake). The timing module holds P2max, P3min and
P4min with their standard defaults, each overridable in ``[kline.timing]``; the
remaining ISO 14230-2 timing parameters come with the serial I/O.

Routing And Gateway Behaviour
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

The ``DiagnosticTransportRouter`` handles K-Line as a third gateway, added as a
third type parameter defaulting to a disabled placeholder and set with
``with_kline``, next to ``with_doip`` and ``with_can``. One gateway owns one
K-Line interface. K-Line is half-duplex with one active ECU per init, so
requests are serialized and the link is (re)initialized on demand.
``route_status`` reports ``ProbeRequired`` for a mapped ECU until the init
handshake succeeds, and ``probe_ecu`` performs the init. Functional addressing
is out of scope for the first skeleton: ``send_functional`` returns
``RequestNotSupported``, as CAN did initially.
