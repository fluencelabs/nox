// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import { AbiItem } from '../../types';
import { asAddress } from '../../util/sliceAs';
import Decoder from '../../decoder/decoder';
import DecodedLog from './decodedLog';
import DecodedLogParam from './decodedLogParam';
import EventParam from './eventParam';
import { eventSignature } from '../../util/signature';
import Token from '../../token';

class Event {
  private _anonymous: boolean;
  private _id: string;
  private _inputs: EventParam[];
  private _name: string | undefined;
  private _signature: string;

  constructor (abi: AbiItem) {
    this._inputs = EventParam.toEventParams(abi.inputs || []);
    this._anonymous = !!abi.anonymous;

    const { id, name, signature } = eventSignature(
      abi.name,
      this.inputParamTypes()
    );

    this._id = id;
    this._name = name;
    this._signature = signature;
  }

  get anonymous () {
    return this._anonymous;
  }

  get id () {
    return this._id;
  }

  get inputs () {
    return this._inputs;
  }

  get name () {
    return this._name;
  }

  get signature () {
    return this._signature;
  }

  inputParamTypes () {
    return this._inputs.map(input => input.kind);
  }

  inputParamNames () {
    return this._inputs.map(input => input.name);
  }

  indexedParams (indexed: boolean) {
    return this._inputs.filter(input => input.indexed === indexed);
  }

  decodeLog (topics: string[], data: string) {
    const topicParams = this.indexedParams(true);
    const dataParams = this.indexedParams(false);

    let address = '';
    let toSkip: number;

    if (!this.anonymous) {
      address = asAddress(topics[0]);
      toSkip = 1;
    } else {
      toSkip = 0;
    }

    const topicTypes = topicParams.map(param => param.kind);
    const flatTopics = topics
      .filter((topic, index) => index >= toSkip)
      .map(topic => {
        return topic.substr(0, 2) === '0x' ? topic.substr(2) : topic;
      })
      .join('');
    const topicTokens = Decoder.decode(topicTypes, flatTopics);

    if (topicTokens.length !== topics.length - toSkip) {
      throw new Error('Invalid topic data');
    }

    const dataTypes = dataParams.map(param => param.kind);
    const dataTokens = Decoder.decode(dataTypes, data);

    const namedTokens: { [key: string]: Token } = {};

    topicParams.forEach((param, index) => {
      namedTokens[param.name || index] = topicTokens[index];
    });
    dataParams.forEach((param, index) => {
      namedTokens[param.name || index] = dataTokens[index];
    });

    const inputParamTypes = this.inputParamTypes();
    const decodedParams = this.inputParamNames().map(
      (name, index) =>
        new DecodedLogParam(
          name,
          inputParamTypes[index],
          namedTokens[name || index]
        )
    );

    return new DecodedLog(decodedParams, address);
  }
}

export default Event;
