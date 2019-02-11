// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { padAddress } from './pad';

/**
 * Slice data.
 *
 * @param data - Data to slice.
 */
export const sliceData = (data?: string): string[] | null => {
  if (!data || !data.length) {
    return [];
  }

  let dataWithout0x = data.substr(0, 2) === '0x' ? data.substr(2) : data;

  if (!dataWithout0x.length) {
    dataWithout0x = padAddress('');
  }

  return dataWithout0x.match(/.{1,64}/g);
};
