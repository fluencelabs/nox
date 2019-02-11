// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

const HEXDIGITS = [
  '0',
  '1',
  '2',
  '3',
  '4',
  '5',
  '6',
  '7',
  '8',
  '9',
  'a',
  'b',
  'c',
  'd',
  'e',
  'f'
];

export function isArray<T> (input?: any): input is Array<T> {
  return Array.isArray(input);
}

export function isError (input?: any): input is Error {
  return input instanceof Error;
}

export function isFunction (input?: any): input is Function {
  return typeof input === 'function';
}

export function isHex (input?: any): boolean {
  if (!isString(input)) {
    return false;
  }

  if (input.substr(0, 2) === '0x') {
    return isHex(input.slice(2));
  }

  const lowerCaseInput = input.toLowerCase();
  let hex = true;

  for (let index = 0; hex && index < input.length; index++) {
    hex = HEXDIGITS.includes(lowerCaseInput[index]);
  }

  return hex;
}
export function isObject (input?: any): input is object {
  return Object.prototype.toString.call(input) === '[object Object]';
}

export function isString (input?: any): input is string {
  return typeof input === 'string';
}

export function isInstanceOf (input: any, clazz: any) {
  return input instanceof clazz;
}
