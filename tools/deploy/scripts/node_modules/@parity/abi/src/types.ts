// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';

import Token from './token';

export interface AbiInput {
  indexed?: boolean;
  name?: string;
  type: TokenTypeEnum;
}

export type AbiItemType = 'function' | 'event' | 'constructor' | 'fallback';

export type MediateType = 'raw' | 'prefixed' | 'fixedArray' | 'array';

export type Slices = string[] | null | undefined;

// Elementary types
export type TokenTypeEnum =
  | 'address'
  | 'bool'
  | 'bytes'
  | 'bytes1'
  | 'bytes2'
  | 'bytes3'
  | 'bytes4'
  | 'bytes5'
  | 'bytes6'
  | 'bytes7'
  | 'bytes8'
  | 'bytes9'
  | 'bytes10'
  | 'bytes11'
  | 'bytes12'
  | 'bytes13'
  | 'bytes14'
  | 'bytes15'
  | 'bytes16'
  | 'bytes17'
  | 'bytes18'
  | 'bytes19'
  | 'bytes20'
  | 'bytes21'
  | 'bytes22'
  | 'bytes23'
  | 'bytes24'
  | 'bytes25'
  | 'bytes26'
  | 'bytes27'
  | 'bytes28'
  | 'bytes29'
  | 'bytes30'
  | 'bytes31'
  | 'bytes32'
  | 'string'
  | 'int'
  | 'int8'
  | 'int16'
  | 'int32'
  | 'int64'
  | 'int128'
  | 'int256'
  | 'uint'
  | 'uint8'
  | 'uint16'
  | 'uint32'
  | 'uint64'
  | 'uint128'
  | 'uint256'
  | 'fixedBytes'
  | 'fixedArray'
  | 'array';

// TS types for each of the above type's values
export type AddressValue = string; // '0x123...'
export type BoolValue = boolean | string; // true or '1'
export type BytesValue = string | number[]; // '0x123' or [34, 35, 36]
export type StringValue = string; // 'foo'
export type IntValue = number | string | BigNumber; // -1
export type UintValue = IntValue; // 1
export type FixedBytesValue = BytesValue; // '0x123'
export type FixedArrayValue = (boolean | string | number | BigNumber | Token)[];
export type ArrayValue = FixedArrayValue[];

export type TokenValue =
  | AddressValue
  | Boolean
  | BytesValue
  | StringValue
  | IntValue
  | UintValue
  | FixedBytesValue
  | FixedArrayValue
  | ArrayValue;

export interface AbiItem {
  anonymous?: boolean;
  constant?: boolean;
  inputs?: AbiInput[];
  name?: string;
  payable?: boolean;
  outputs?: AbiInput[];
  type?: AbiItemType;
}

export type AbiObject = AbiItem[];
