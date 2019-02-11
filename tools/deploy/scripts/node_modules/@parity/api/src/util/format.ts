// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { Bytes } from '../types';
import { range } from 'lodash';

/**
 * Convert a bytes array to a hexadecimal string.
 *
 * @param bytes - The bytes array to convert to hexadecimal.
 */
export const bytesToHex = (bytes: Bytes) => {
  return '0x' + Buffer.from(bytes).toString('hex');
};

/**
 * Clean the value if it's cleanable:
 * - ASCII arrays are returned as ASCII strings
 * - non-ASCII arrays are converted to hex
 * - uint (<=48) are converted to numbers
 *
 * @param value - The value to clean.
 * @param type - The type of the value.
 */
export const cleanupValue = (value: string | number | Bytes, type: string) => {
  // TODO: make work with arbitrary depth arrays
  if (value instanceof Array && type.match(/bytes[0-9]+/)) {
    // figure out if it's an ASCII string hiding in there:
    let ascii: string | null = '';
    let ended = false;

    for (let index = 0; index < value.length && ascii !== null; ++index) {
      const val = value[index];

      if (val === 0) {
        ended = true;
      } else {
        ascii += String.fromCharCode(val);
      }

      if ((ended && val !== 0) || (!ended && (val < 32 || val >= 128))) {
        ascii = null;
      }
    }

    value = ascii === null ? bytesToHex(value) : ascii;
  }

  if (type.substr(0, 4) === 'uint' && +type.substr(4) <= 48) {
    value = +value;
  }

  return value;
};

/**
 * Convert a hexadecimal string to a bytes array.
 *
 * @param hex - The hex string to convert.
 */
export const hexToBytes = (hex: string) => {
  const raw = toHex(hex).slice(2);
  const bytes = [];

  for (let i = 0; i < raw.length; i += 2) {
    bytes.push(parseInt(raw.substr(i, 2), 16));
  }

  return bytes;
};

/**
 * Convert a hexadecimal string to an ASCII string.
 *
 * @param hex - The hex string to convert.
 */
export const hexToAscii = (hex: string) => {
  const bytes = hexToBytes(hex);
  const str = bytes.map(byte => String.fromCharCode(byte)).join('');

  return str;
};

/**
 * Convert a bytes array to an ASCII string.
 *
 * @param bytes - The bytes array to convert.
 */
export const bytesToAscii = (bytes: Bytes) =>
  bytes.map(b => String.fromCharCode(b % 512)).join('');

/**
 * Convert an ASCII string to a hexadecimal string.
 *
 * @param string - The ASCII string to convert.
 */
export const asciiToHex = (baseString: string) => {
  let result = '0x';

  for (let i = 0; i < baseString.length; ++i) {
    result += ('0' + baseString.charCodeAt(i).toString(16)).substr(-2);
  }

  return result;
};

/**
 * Pad the input string with `length` zeros on the right.
 *
 * @param input - The input string to pad.
 * @param length - The number of zeros to pad.
 */
export const padRight = (input: string, length: number) => {
  const hexLength = length * 2;
  const value = toHex(input).substr(2, hexLength);

  return (
    '0x' +
    value +
    range(hexLength - value.length)
      .map(() => '0')
      .join('')
  );
};

/**
 * Pad the input string with `length` zeros on the left.
 *
 * @param input - The input string to pad.
 * @param length - The number of zeros to pad.
 */
export const padLeft = (input: string | undefined, length: number) => {
  const hexLength = length * 2;
  const value = toHex(input).substr(2, hexLength);

  return (
    '0x' +
    range(hexLength - value.length)
      .map(() => '0')
      .join('') +
    value
  );
};

/**
 * Convert a string to hexadecimal.
 *
 * @param str - The string to convert.
 */
export const toHex = (str?: string) => {
  if (str && str.toString) {
    // TODO string has no toString(16)
    // @ts-ignore
    str = str.toString(16);
  }

  if (str && str.substr(0, 2) === '0x') {
    return str.toLowerCase();
  }

  return `0x${(str || '').toLowerCase()}`;
};
