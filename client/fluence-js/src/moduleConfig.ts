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

export interface ModuleConfig {
    name: string,
    mem_pages_count?: number,
    logger_enabled?: boolean,
    wasi?: Wasi,
    mounted_binaries?: object
}

export interface Wasi {
    envs?: object,
    preopened_files?: string[],
    mapped_dirs?: object,
}