// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { keccak_256 } from 'js-sha3';

import { hexToBytes } from './format';
import { isHex } from './types';
import { Bytes } from '../types';

type Encoding = 'hex' | 'raw';

/**
 *
 * @param value - The value to hash
 * @param options - Set the encoding in the options, encoding can be `'hex'`
 * or `'raw'`.
 */
export const sha3 = (
  value: string | Bytes,
  options?: { encoding: Encoding }
): string => {
  const forceHex = options && options.encoding === 'hex';

  if (forceHex || (!options && isHex(value))) {
    const bytes = hexToBytes(value as string);

    return sha3(bytes);
  }

  const hash = keccak_256(value);

  return `0x${hash}`;
};

export const sha3Text = (val: string) => sha3(val, { encoding: 'raw' });
