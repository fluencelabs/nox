// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { keccak_256 } from 'js-sha3';

/**
 * Verify that an address has a valid checksum.
 *
 * @param address - The Ethereum address to verify.
 */
export const isChecksumValid = (address: string) => {
  const _address = address.replace('0x', '');
  const hash = keccak_256(_address.toLowerCase());

  for (let n = 0; n < 40; n++) {
    const char = _address[n];
    const isLower = char !== char.toUpperCase();
    const isUpper = char !== char.toLowerCase();
    const hashval = parseInt(hash[n], 16);

    if ((hashval > 7 && isLower) || (hashval <= 7 && isUpper)) {
      return false;
    }
  }

  return true;
};

/**
 * Verify that an address is a valid Ethereum address.
 *
 * @param address - The address to verify.
 */
export const isAddress = (address?: string) => {
  if (address && address.length === 42) {
    if (!/^(0x)?[0-9a-f]{40}$/i.test(address)) {
      return false;
    } else if (
      /^(0x)?[0-9a-f]{40}$/.test(address) ||
      /^(0x)?[0-9A-F]{40}$/.test(address)
    ) {
      return true;
    }

    return isChecksumValid(address);
  }

  return false;
};

/**
 * Convert an Ethereum address to its checksum-valid version.
 *
 * @param address - The address to convert.
 */
export const toChecksumAddress = (address?: string | null) => {
  const _address = (address || '').toLowerCase();

  if (!isAddress(_address)) {
    return '';
  }

  const hash = keccak_256(_address.slice(-40));
  let result = '0x';

  for (let n = 0; n < 40; n++) {
    result = `${result}${
      parseInt(hash[n], 16) > 7
        ? _address[n + 2].toUpperCase()
        : _address[n + 2]
    }`;
  }

  return result;
};
