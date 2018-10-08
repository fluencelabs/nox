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
 * Possible results from the real-time cluster.
 */
import {fromHex} from "./utils";

export type Result = Empty | Value

/**
 * The empty result, if there is no value in response.
 */
export class Empty {}

/**
 * The result with value as a string from the real-time cluster.
 */
export class Value {

    private readonly value: string;

    /**
     * @param v hex string
     */
    constructor(v: string) {
        this.value = v
    }

    hex(): string {
        return this.value;
    }

}

/**
 * Returns if some error occurred on request in the real-time cluster.
 */
export class ErrorResult extends Error {
    constructor(err: string) {
        super(err);
        this.error = err;
    }

    readonly error: string
}

export const empty: Result = new Empty();

export function isValue(r: Result): r is Value {
    return r instanceof Value;
}

export function value(v: string): Value {
    return new Value(v)
}

export function error(err: string) {
    return new ErrorResult(err)
}

export function parseObject(obj: any): Result {
    if (obj.Error !== undefined) {
        throw error(obj.Error.message)
    } else if (obj.Empty !== undefined) {
        return empty;
    } else if (obj.Computed) {
        return value(obj.Computed.value);
    } else {
        throw error("Could not parse the response: " + JSON.stringify(obj))
    }
}
