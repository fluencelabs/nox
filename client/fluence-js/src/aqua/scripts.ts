/*
 * Copyright 2020 Fluence Labs Limited
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
 * Experimental attempts to generate aqua code through typescript functions.
 */
export interface Value<T> {
    name: string,
    value: T
}

export function value<T>(name: string, v: T): Value<T> {
    return { name, value: v }
}

function updateData(value: Value<any>, data: Map<string, any>): void {
    if (!data.get(value.name)) {
        data.set(value.name, value.value)
    }
}

function isValue<T>(value: string | Value<T>): value is Value<T> {
    return (value as Value<T>).name !== undefined;
}

/**
 * Generate a script with a call. Data is modified.
 * @param target
 * @param service
 * @param functionV
 * @param args
 * @param returnName
 * @param data
 */
export function call(target: string | Value<string>, service: string | Value<string>,
                     functionV: string | Value<string>, args: (string | Value<any>)[],
                     returnName: string | undefined, data: Map<string, any>): string {

    let targetName = target;
    if (isValue(target)) {
        updateData(target, data)
        targetName = target.name
    }

    let serviceName = service;
    if (isValue(service)) {
        updateData(service, data)
        serviceName = service.name;
    }

    let functionName = functionV;
    if (isValue(functionV)) {
        updateData(functionV, data)
        functionName = functionV.name;
    }

    let argsStr: string[] = []
    args.forEach((v) => {
        if (isValue(v)) {
            updateData(v, data)
            argsStr.push(v.name)
        } else {
            argsStr.push(v)
        }
    })

    if (!returnName) {
        returnName = ""
    }

    return `(call ${targetName} ("${serviceName}" "${functionName}") [${argsStr.join(" ")}] ${returnName})`
}

function wrap(command: string, scripts: string[]): string {
    if (scripts.length === 2) {
        return `(${command}
    ${scripts[0]}
    ${scripts[1]}    
)`
    } else {
        let first = scripts.shift()
        return `(${command}
    ${first}
    ${wrap(command, scripts)}
)`
    }
}

/**
 * Wrap an array of scripts with multiple 'seq' commands
 * @param scripts
 */
export function seq(scripts: string[]): string {
    if (scripts.length < 2) {
        throw new Error("For 'seq' there must be at least 2 scripts")
    }

    return wrap("seq", scripts)
}

/**
 * Wrap a script with 'par' command
 * @param script
 */
export function par(script: string): string {
    return `par(
    ${script}
)
    `
}