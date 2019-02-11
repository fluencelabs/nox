// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';

import { toChecksumAddress } from './address';

/**
 * Convert slice to u32.
 *
 * @param slice - Slice to convert.
 */
export const asU32 = (slice: string) => {
  // TODO: validation

  return new BigNumber(slice.toLowerCase(), 16);
};

/**
 * Convert slice to i32.
 *
 * @param slice - Slice to convert.
 */
export const asI32 = (slice: string) => {
  if (new BigNumber(slice.substr(0, 1), 16).toString(2)[0] === '1') {
    return new BigNumber(slice, 16)
      .minus(
        new BigNumber(
          'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff',
          16
        )
      )
      .minus(1);
  }

  return new BigNumber(slice, 16);
};

/**
 * Convert slice to checksum address.
 *
 * @param slice - Slice to convert.
 */
export const asAddress = (slice: string) => {
  // TODO: address validation?

  return toChecksumAddress(`0x${slice.slice(-40)}`);
};

/**
 * Convert slice to boolean.
 *
 * @param slice - Slice to convert.
 */
export const asBool = (slice: string) => {
  // TODO: everything else should be 0

  return new BigNumber(slice[63]).eq(1);
};
