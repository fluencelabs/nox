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

export interface Blueprint {
    dependencies: string[],
    id: string,
    name: string
}

export function checkBlueprint(b: any): b is Blueprint {
    if (!b.id) throw new Error(`There is no 'id' field in Blueprint struct: ${JSON.stringify(b)}`)
    if (b.dependencies) {
        b.dependencies.forEach((dep: any) => {
            if ((typeof dep) !== 'string') {
                throw Error(`'dependencies' should be an array of strings: ${JSON.stringify(b)}`)
            }
        });
    }

    return true;

}