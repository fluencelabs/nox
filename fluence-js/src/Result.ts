/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * The result with value as a string from the real-time cluster.
 */
export interface QueryResponse {
    height: string;
    value?: string;
    code?: number;
    info: string;
}

export enum ErrorType {
    TendermintError,
    TransportError,
    MalformedError,
    ParsingError,
    SessionClosed,
    InternalError
}

/**
 * Returns if some error occurred on request in the real-time cluster.
 */
export class ErrorResponse extends Error {
    constructor(errorType: ErrorType, err: string) {
        super(err);
        this.error = err;
        this.errorType = errorType;
    }

    readonly error: string;
    readonly errorType: ErrorType;
}

export function error(errorType: ErrorType, err: string) {
    return new ErrorResponse(errorType, err)
}

/**
 * The result with value as a bytes array from the real-time cluster.
 */
export class Result {

    private readonly value: Uint8Array;

    /**
     * @param v hex string
     */
    constructor(v: Uint8Array) {
        this.value = v
    }

    bytes(): Uint8Array {
        return this.value;
    }

    asString(): string {
        return new TextDecoder("utf-8").decode(this.value);
    }
}
