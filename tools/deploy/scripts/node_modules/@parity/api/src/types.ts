// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

import BigNumber from 'bignumber.js';

export interface AccountInfo {
  [address: string]: {
    meta?: { [index: string]: any };
    name: string;
    uuid?: string;
  };
}

export type Bytes = number[];

export interface Block {
  author?: string;
  miner?: string;
  difficulty?: BigNumber;
  gasLimit?: BigNumber;
  gasUsed?: BigNumber;
  nonce?: BigNumber;
  number?: BigNumber;
  totalDifficulty?: BigNumber;
  timestamp?: Date;
}

export type BlockGap = BigNumber[];

export type BlockNumber =
  | 'earliest'
  | 'latest'
  | 'pending'
  | number
  | BigNumber
  | string;

export type ChainStatus = {
  blockGap?: BlockGap | null;
};

export type Condition = {
  block?: BlockNumber;
  time?: Date;
};

export interface DeriveObject {
  hash?: number;
  index?: number;
  type?: string;
}
export type Derive = number | string | DeriveObject | null;

export type EtherDenomination =
  | 'wei'
  | 'ada'
  | 'babbage'
  | 'shannon'
  | 'szabo'
  | 'finney'
  | 'ether'
  | 'kether'
  | 'mether'
  | 'gether'
  | 'tether';

export interface FilterObject {
  fromAddress?: string;
  fromBlock?: BlockNumber;
  toAddress?: string;
  toBlock?: BlockNumber;
}

export interface FilterOptions {
  address?: string | string[];
  extraData?: string;
  fromBlock?: BlockNumber;
  limit?: BlockNumber;
  toBlock?: BlockNumber;
  topics?: Topic[];
}

export interface Histogram {
  bucketBounds?: BigNumber[];
  counts?: BigNumber[];
}

export interface HwAccountInfo {
  [address: string]: { [key: string]: any };
}

export interface Log {
  address?: string;
  blockNumber?: BigNumber;
  logIndex?: BigNumber;
  transactionIndex?: BigNumber;
}

export interface NodeKind {
  availability: string;
  capability: string;
}

export interface Options {
  extraData?: string;
  from?: string;
  condition?: Condition | null;
  gas?: BlockNumber;
  gasPrice?: BlockNumber;
  value?: BlockNumber;
  nonce?: BlockNumber;
  to?: string;
  data?: any;
}

export type PeerProtocol = 'par' | 'les';

export interface PeerProtocolItem {
  difficulty: BigNumber;
  head: BigNumber;
  version: BigNumber;
}

export interface Peer {
  caps: string[];
  id: string;
  name: string;
  network: {
    localAddress: string;
    remoteAddress: string;
  };
  protocols?: {
    les?: PeerProtocolItem | null;
    par?: PeerProtocolItem | null;
  };
}

export interface Peers {
  active: BigNumber;
  connected: BigNumber;
  max: BigNumber;
  peers: Peer[];
}

export interface Receipt {
  contractAddress?: string;
  blockNumber?: BigNumber;
  cumulativeGasUsed?: BigNumber;
  gasUsed?: BigNumber;
  transactionIndex?: BigNumber;
  status?: BigNumber;
}

export interface SignerRequest {
  id?: BigNumber;
  origin?: {
    [index: string]: string;
  };
  payload?: {
    decrypt?: SigningPayload;
    sign?: SigningPayload;
    signTransaction?: Transaction;
    sendTransaction?: Transaction;
  };
}

export interface SigningPayload {
  address?: string;
}

export interface Syncing {
  currentBlock?: BigNumber;
  highestBlock?: BigNumber;
  startingBlock?: BigNumber;
  warpChunksAmount?: BigNumber;
  warpChunksProcessed?: BigNumber;
  blockGap?: BlockGap | null;
}

export interface Trace {
  action?: {
    gas?: BigNumber;
    value?: BigNumber;
    balance?: BigNumber;
    from?: string;
    to?: string;
    address?: string;
    refundAddress?: string;
  };
  result?: {
    address?: string;
    gasUsed?: BigNumber;
  };
  traceAddress?: BigNumber[];
  subtraces?: BigNumber;
  transactionPosition?: BigNumber;
  blockNumber?: BigNumber;
}

export interface TraceReplay {
  trace?: Trace[];
}

export interface Transaction {
  blockNumber?: BigNumber;
  creates?: string;
  extraData?: string;
  to?: string;
  from?: string;
  condition?: Condition | null;
  gas?: BigNumber;
  gasPrice?: BigNumber;
  transactionIndex?: BigNumber;
  value?: BigNumber;
  nonce?: BigNumber;
  data?: string;
}

export type Topic = string | null;

export interface VaultMeta {
  [index: string]: any;
}

export type AnyType =
  | AccountInfo
  | Block
  | BlockGap
  | BlockNumber
  | ChainStatus
  | Condition
  | DeriveObject
  | Derive
  | FilterObject
  | FilterOptions
  | Histogram
  | HwAccountInfo
  | Log
  | NodeKind
  | Options
  | Peer
  | Peers
  | Receipt
  | SignerRequest
  | SigningPayload
  | Syncing
  | Trace
  | TraceReplay
  | Transaction
  | Topic
  | VaultMeta;
