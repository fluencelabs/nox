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

export interface Result {}

class Timeout implements Result {}

class Empty implements Result {}

class Value implements Result {

    constructor(v: string) {
        this.value = v
    }

    readonly value: string
}

class Error implements Result {
    constructor(err: string) {
        this.error = err
    }

    readonly error: string
}

export const timeout = new Timeout();

export const empty = new Empty();

export function value(v: string) {
    return new Value(v)
}

export function error(err: string) {
    return new Error(err)
}