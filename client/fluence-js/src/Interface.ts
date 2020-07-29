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

export interface Interface {
    service_id: string,
    modules: Module[]
}

export function checkFunction(f: any): f is Function {
    if (!f.name) throw Error(`There is no 'name' field in Function struct: ${JSON.stringify(f)}`)

    if (f.input_types) {
        if (!(f.input_types instanceof Array)) {
            throw Error(`'input_types' should be an array: ${JSON.stringify(f)}`)
        }
        f.input_types.forEach((i: any) => {
            if ((typeof i) !== 'string') {
                throw Error(`'input_types' should be a string: ${JSON.stringify(f)}`)
            }
        });
    }

    if (f.output_types) {
        if (!(f.output_types instanceof Array)) {
            throw Error(`'output_types' should be an array: ${JSON.stringify(f)}`)
        }
        f.output_types.forEach((o: any) => {
            if ((typeof o) !== 'string') {
                throw Error(`'output_types' should be a string: ${JSON.stringify(f)}`)
            }
        });
    }

    return true;
}

function checkModule(module: any): module is Module {
    if (!module.name) throw Error(`There is no 'name' field in Module struct: ${JSON.stringify(module)}`)
    if (!module.functions) {
        module.functions.forEach((f: any) => {
            checkFunction(f)
        });
    }

    return true;
}

/**
 * Throws an error if 'i' is not Interface type.
 */
export function checkInterface(i: any): i is Interface {
    if (!i.service_id) throw new Error(`There is no 'service_id' field in Interface struct: ${JSON.stringify(i)}`)
    if (i.modules) {
        i.modules.forEach((module: any) => {
            checkModule(module);
        });
    }

    return true;

}

export interface Module {
    name: string,
    functions: Function[]
}

export interface Function {
    name: string,
    input_types: string[],
    output_types: string[]
}