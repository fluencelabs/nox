// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';
import { encode } from 'utf8';

import {
  AddressValue,
  BoolValue,
  BytesValue,
  IntValue,
  UintValue
} from '../types';
import { isArray } from './types';

const ZERO_64 =
  '0000000000000000000000000000000000000000000000000000000000000000';

/**
 * Pad an address with zeros on the left.
 *
 * @param input - The input address to pad.
 */
export const padAddress = (input: AddressValue) => {
  const inputWithout0x = input.startsWith('0x') ? input.substr(2) : input;

  return `${ZERO_64}${inputWithout0x}`.slice(-64);
};

/**
 * Pad a boolean with zeros on the left.
 *
 * @param input - The input address to pad.
 */
export const padBool = (input: BoolValue) => {
  return `${ZERO_64}${input ? '1' : '0'}`.slice(-64);
};

/**
 * Pad a u32 with zeros on the left.
 *
 * @param input - The input address to pad.
 */
export const padU32 = (input: IntValue | UintValue) => {
  let bn = new BigNumber(input);

  if (bn.isNaN()) {
    throw new Error('Input is not a valid number.');
  }

  if (bn.isLessThan(0)) {
    bn = new BigNumber(
      'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff',
      16
    )
      .plus(bn)
      .plus(1);
  }

  return `${ZERO_64}${bn.toString(16)}`.slice(-64);
};

/**
 * Convert an input string to bytes.
 *
 * @param input - The input string to convert.
 */
export const stringToBytes = (input: BytesValue) => {
  if (isArray(input)) {
    return input as number[];
  } else if ((input as string).startsWith('0x')) {
    const matches =
      (input as string)
        .substr(2)
        .toLowerCase()
        .match(/.{1,2}/g) || [];

    return matches.map(value => parseInt(value, 16));
  } else {
    return (input as string).split('').map(char => char.charCodeAt(0));
  }
};

/**
 * Pad bytes with zeros on the left.
 *
 * @param input - The input bytes to pad.
 */
export const padBytes = (input: BytesValue) => {
  const inputBytes = stringToBytes(input);

  return `${padU32(inputBytes.length)}${padFixedBytes(inputBytes)}`;
};

/**
 * Pad fixed bytes.
 *
 * @param input - Input bytes to pad.
 */
export const padFixedBytes = (input: BytesValue) => {
  const inputBytes = stringToBytes(input);
  const sinput = inputBytes
    .map(code => `0${code.toString(16)}`.slice(-2))
    .join('');
  const max = Math.floor((sinput.length + 63) / 64) * 64;

  return `${sinput}${ZERO_64}`.substr(0, max);
};

/**
 * Pad string.
 *
 * @param input - String to pad.
 */
export const padString = (input: string) => {
  const array = encode(input)
    .split('')
    .map(char => char.charCodeAt(0));

  return padBytes(array);
};
