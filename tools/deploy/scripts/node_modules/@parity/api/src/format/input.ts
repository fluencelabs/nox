// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';

import {
  BlockNumber,
  Condition,
  DeriveObject,
  Derive,
  FilterObject,
  FilterOptions,
  Options,
  Topic
} from '../types';
import { isArray, isHex, isInstanceOf, isString } from '../util/types';
import { padLeft, toHex } from '../util/format';
import { SerializedCondition, SerializedTransaction } from './types.serialized';

/**
 * Validate input address.
 *
 * @param address - Input address to validate.
 */
export const inAddress = (address?: string) => {
  // TODO: address validation if we have upper-lower addresses
  return inHex(address);
};

/**
 * Validate input addresses.
 *
 * @param addresses - Input addresses to validate.
 */
export const inAddresses = (addresses?: (string | undefined)[]) => {
  return (addresses || []).map(inAddress);
};

export const inBlockNumber = (blockNumber?: BlockNumber) => {
  if (isString(blockNumber)) {
    switch (blockNumber) {
      case 'earliest':
      case 'latest':
      case 'pending':
        return blockNumber;
    }
  }

  return inNumber16(blockNumber);
};

export const inData = (data: string) => {
  if (data && data.length && !isHex(data)) {
    data = data
      .split('')
      .map(chr => `0${chr.charCodeAt(0).toString(16)}`.slice(-2))
      .join('');
  }

  return inHex(data);
};

export const inHash = (hash: string) => {
  return inHex(hash);
};

export const inTopics = (topics?: (Topic | Topic[])[]): (string | null)[] => {
  if (!topics) {
    return [] as string[];
  }

  // @ts-ignore I don't want to deal with resursive nested arrays
  // TODO https://stackoverflow.com/questions/52638149/recursive-nested-array-type-can-it-be-done
  return topics
    .filter((topic: Topic | Topic[]) => topic === null || topic)
    .map((topic: Topic | Topic[]) => {
      if (topic === null) {
        return null;
      }

      if (Array.isArray(topic)) {
        return inTopics(topic);
      }

      return padLeft(topic, 32);
    });
};

export const inFilter = (options: FilterOptions) => {
  const result: {
    [key in keyof FilterOptions]: number | string | string[] | Topic[]
  } = {};

  if (options) {
    Object.keys(options).forEach(key => {
      switch (key) {
        case 'address':
          if (isArray(options[key])) {
            result[key] = (options[key] as string[]).map(inAddress);
          } else {
            result[key] = inAddress(options[key] as string);
          }
          break;

        case 'fromBlock':
        case 'toBlock':
          result[key] = inBlockNumber(options[key]);
          break;

        case 'limit':
          result[key] = inNumber10(options[key]);
          break;

        case 'topics':
          result[key] = inTopics(options[key]);
          break;

        default:
          // @ts-ignore Here, we explicitly pass down extra keys, if they exist
          result[key] = options[key];
      }
    });
  }

  return result;
};

export const inHex = (str?: string) => toHex(str);

export const inNumber10 = (n?: BlockNumber) => {
  if (isInstanceOf(n, BigNumber)) {
    return (n as BigNumber).toNumber();
  }

  return new BigNumber(n || 0).toNumber();
};

export const inNumber16 = (n?: BigNumber | string | number) => {
  const bn = isInstanceOf(n, BigNumber)
    ? (n as BigNumber)
    : new BigNumber(n || 0);

  if (!bn.isInteger()) {
    throw new Error(
      `[format/input::inNumber16] the given number is not an integer: ${bn.toFormat()}`
    );
  }

  return inHex(bn.toString(16));
};

export const inOptionsCondition = (condition?: Condition | null) => {
  if (condition) {
    return {
      block: condition.block ? inNumber10(condition.block) : null,
      time: condition.time
        ? inNumber10(Math.floor(condition.time.getTime() / 1000))
        : null
    } as SerializedCondition;
  }

  return null;
};

export const inOptions = (_options: Options = {}) => {
  const options = Object.assign({}, _options);

  const result: SerializedTransaction = {};

  Object.keys(options).forEach(key => {
    switch (key) {
      case 'to':
        // Don't encode the `to` option if it's empty
        // (eg. contract deployments)
        if (options[key]) {
          result.to = inAddress(options[key]);
        }
        break;

      case 'from':
        result[key] = inAddress(options[key]);
        break;

      case 'condition':
        result[key] = inOptionsCondition(options[key]);
        break;

      case 'gas':
      case 'gasPrice':
        result[key] = inNumber16(
          new BigNumber(options[key] as BigNumber) // TODO Round number
        );
        break;

      case 'value':
      case 'nonce':
        result[key] = inNumber16(options[key]);
        break;

      case 'data':
        result[key] = inData(options[key]);
        break;

      default:
        // @ts-ignore Here, we explicitly pass down extra keys, if they exist
        result[key] = options[key];
    }
  });

  return result;
};

export const inTraceFilter = (filterObject: FilterObject) => {
  const result: { [key in keyof FilterObject]: string | string[] } = {};

  if (filterObject) {
    Object.keys(filterObject).forEach(key => {
      switch (key) {
        case 'fromAddress':
        case 'toAddress':
          result[key] = ([] as string[])
            .concat(filterObject[key] as string)
            .map(address => inAddress(address));
          break;

        case 'toBlock':
        case 'fromBlock':
          result[key] = inBlockNumber(filterObject[key]);
          break;

        default:
          // @ts-ignore Here, we explicitly pass down extra keys, if they exist
          result[key] = options[key];
      }
    });
  }

  return result;
};

export const inTraceType = (whatTrace: string | string[]) => {
  if (isString(whatTrace)) {
    return [whatTrace];
  }

  return whatTrace;
};

export const inDeriveType = (derive?: Derive) => {
  return derive && (derive as DeriveObject).type === 'hard' ? 'hard' : 'soft';
};

export const inDeriveHash = (derive?: Derive | string) => {
  const hash =
    derive && (derive as DeriveObject).hash
      ? (derive as DeriveObject).hash
      : (derive as string);
  const type = inDeriveType(derive as Derive);

  return {
    hash: inHex(hash as string),
    type
  };
};

export const inDeriveIndex = (derive: Derive | Derive[]) => {
  if (!derive) {
    return [];
  }

  const deriveAsArray: Derive[] = isArray(derive) ? derive : [derive];

  return deriveAsArray.map(item => {
    const index = inNumber10(
      item && (item as DeriveObject).index
        ? (item as DeriveObject).index
        : (item as number)
    );

    return {
      index,
      type: inDeriveType(item)
    };
  });
};
