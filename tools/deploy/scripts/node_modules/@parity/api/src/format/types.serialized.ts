// Copyright 2015-2018 Parity Technologies (UK) Ltd.
// This file is part of Parity.
//
// SPDX-License-Identifier: MIT

export type SerializedNumber = number | string;

export interface SerializedAccountInfo {
  [address: string]: { meta?: string; name: string; uuid?: string };
}

export interface SerializedBlock {
  author?: string;
  miner?: string;
  difficulty?: string;
  extraData?: string;
  gasLimit?: string;
  gasUsed?: string;
  nonce?: string;
  number?: string;
  totalDifficulty?: string;
  timestamp?: number | string;
}

export type SerializedBlockGap = (string | number)[];

export interface SerializedChainStatus {
  blockGap?: SerializedBlockGap | null;
}

export type SerializedCondition = {
  block?: SerializedNumber;
  time?: SerializedNumber;
};

export interface SerializedHistogram {
  bucketBounds?: number[];
  counts?: number[];
}

export interface SerializedHwAccountInfo {
  [address: string]: { [key: string]: any };
}

export interface SerializedLog {
  address: string;
  blockNumber: SerializedNumber;
  logIndex: SerializedNumber;
  transactionIndex: SerializedNumber;
}

export interface SerializedPeer {
  caps?: string[];
  id?: string;
  name?: string;
  network?: {
    localAddress: string;
    remoteAddress: string;
  };
  protocols: {
    les?: {
      difficulty: SerializedNumber;
      head: SerializedNumber;
      version: SerializedNumber;
    } | null;
    par?: {
      difficulty: SerializedNumber;
      head: SerializedNumber;
      version: SerializedNumber;
    } | null;
  };
}

export interface SerializedPeers {
  active: SerializedNumber;
  connected: SerializedNumber;
  max: SerializedNumber;
  peers: SerializedPeer[];
}

export interface SerializedReceipt {
  blockNumber?: SerializedNumber;
  contractAddress?: string;
  cumulativeGasUsed?: SerializedNumber;
  extraData?: string;
  gasUsed?: SerializedNumber;
  transactionIndex?: SerializedNumber;
  status?: SerializedNumber;
}

export interface SerializedSignerRequest {
  id?: SerializedNumber;
  origin?: {
    [index: string]: string;
  };
  payload?: {
    decrypt?: SerializedSigningPayload;
    sign?: SerializedSigningPayload;
    signTransaction?: SerializedTransaction;
    sendTransaction?: SerializedTransaction;
  };
}

export interface SerializedSigningPayload {
  address?: string;
}

export interface SerializedTrace {
  action?: {
    callType?: string;
    gas?: SerializedNumber;
    value?: SerializedNumber;
    balance?: SerializedNumber;
    from?: string;
    input?: string;
    to?: string;
    address?: string;
    refundAddress?: string;
  };
  blockHash?: string;
  blockNumber?: SerializedNumber;
  result?: {
    address?: string;
    gasUsed?: SerializedNumber;
    output?: string;
  };
  traceAddress?: SerializedNumber[];
  subtraces?: SerializedNumber;
  transactionHash?: string;
  transactionPosition?: SerializedNumber;
  type?: string;
}

export interface SerializedTraceReplay {
  trace?: SerializedTrace[];
}

export interface SerializedTransaction {
  blockNumber?: SerializedNumber;
  creates?: string;
  extraData?: string;
  to?: string;
  from?: string;
  condition?: SerializedCondition | null;
  gas?: string;
  gasPrice?: string;
  transactionIndex?: SerializedNumber;
  value?: string;
  nonce?: string;
  data?: string;
}

export interface SerializedSyncing {
  currentBlock?: SerializedNumber;
  highestBlock?: SerializedNumber;
  startingBlock?: SerializedNumber;
  warpChunksAmount?: SerializedNumber;
  warpChunksProcessed?: SerializedNumber;
  blockGap?: SerializedBlockGap | null;
}

export type SerializedVaultMeta = string | { [key: string]: any } | null;
