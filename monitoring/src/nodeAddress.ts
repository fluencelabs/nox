/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

import hexToArrayBuffer = require("hex-to-array-buffer");
import arrayBufferToHex = require("array-buffer-to-hex");

const IP_LEN = 4;

const TENDERMINT_KEY_LEN = 20;

/**
 * Address of a node in Fluence contract. Contains tendermint key and IP address of the node.
 */
interface NodeAddress {
    tendermint_key: string,
    ip_addr: string
}

/**
 * Decode node address to tendermint key and IP address.
 */
export function decodeNodeAddress(nodeAddress: string): NodeAddress {
    let buf = hexToArrayBuffer(nodeAddress.replace("0x", ""));

    let tendermint_key = arrayBufferToHex(buf.slice(0, TENDERMINT_KEY_LEN));

    let ip_buf = new DataView(buf.slice(TENDERMINT_KEY_LEN, TENDERMINT_KEY_LEN + IP_LEN));

    let ip_addr = `${ip_buf.getUint8(0)}.${ip_buf.getUint8(1)}.${ip_buf.getUint8(2)}.${ip_buf.getUint8(3)}`;

    return {tendermint_key: tendermint_key, ip_addr: ip_addr};
}