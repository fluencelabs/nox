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

import {TendermintClient} from "../TendermintClient";
import {Engine} from "../Engine";
import {Signer} from "../Signer";
import {Client} from "../Client";
import {Session} from "../Session";
import {empty, isValue, Result} from "../Result";
import {none, Option, some} from "ts-option";

export enum Condition {
    Less = "-1",
    More = "1",
    Equal = "0"
}

export enum Field {
    Id = "0",
    Symbol = "1",
    Price = "2",
    Month = "3"
}

export interface Query {
    query: string,
    args: string[]
}

export class DbClient {

    private readonly session: Session;

    constructor(host: string, port: number) {
        let tm = new TendermintClient(host, port);

        let engine = new Engine(tm);

        // default signing key for now
        let signingKey = "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxT3hAtSkX+Lrl4lN5OEsXjD7GGG7iEewSod472HudrkrA==";
        let signer = new Signer(signingKey);

        // `client002` is a default client for now
        let client = new Client("client002", signer);

        this.session = engine.genSession(client);
    }

    /**
     * Get an array of rows as a result of the request.
     *
     * @param request query for VM
     * @param fetch how many rows requests at once
     * @param fields how many fields in a row
     * @param parser parse row to T
     */
    async getResult<T>(request: Query, fetch: number, fields: number, parser: (arr: Result[]) => T): Promise<T[]> {
        this.session.invoke(request.query, request.args);
        return this.getRows(fetch, fields, parser);
    }

    /**
     * Gets all rows after the query.
     *
     * @param fetch how many rows requests at once
     * @param fields how many fields in a row
     * @param parser parse row to T
     */
    private async getRows<T>(fetch: number, fields: number, parser: (arr: Result[]) => T): Promise<T[]> {
        let all: Promise<Option<T>>[] = await this.getRows0(fetch, fields, parser, []);

        return Promise.all(all)
            .then((arr) =>
                arr.filter((v) => v.nonEmpty)
                    .map((v) => v.get)
            );
    }


    private async getRows0<T>(fetch: number, fields: number, parser: (arr: Result[]) => T, acc: Promise<Option<T>>[]): Promise<Promise<Option<T>>[]> {
        var fetched: [boolean, Promise<Option<T>>[]] = await this.fetchRows(fetch, fields, parser);

        var cont: boolean = fetched[0];

        let allRows: Promise<Option<T>>[] = fetched[1];

        if (cont) {
            return this.getRows0(fetch, fields, parser, allRows);
        } else {
            return acc.concat(allRows);
        }
    }

    /**
     * Requests multiple rows at once.
     *
     * @param fetch how many rows requests at once
     * @param fields how many fields in a row
     * @param parser parse row to T
     */
    private async fetchRows<T>(fetch: number, fields: number, parser: (arr: Result[]) => T): Promise<[boolean, Promise<Option<T>>[]]> {
        let listOfRows: Promise<Option<T>>[] = [];
        let lastRow: Promise<Result> = Promise.resolve(empty);
        for(var _i = 0; _i < fetch; _i++) {
            let row = this.session.invoke("next_row").result();
            let res = this.getRow(fields, parser);
            listOfRows.push(res);
            lastRow = row;
        }

        let lastRowResult: Result = await lastRow;
        if (isValue(lastRowResult) && parseInt(lastRowResult.value) === -1) {
            return [false, listOfRows];
        }

        return [true, listOfRows];
    }

    /**
     * Gets one row.
     * @param fields how many fields in a row
     * @param parser parse row to T
     */
    private async getRow<T>(fields: number, parser: (arr: Result[]) => T): Promise<Option<T>> {

        let listOfFields: Promise<any>[] = [];
        for(var _i = 0; _i < fields; _i++) {
            let pr: Promise<Result> = this.session.invoke("next_field").result();

            listOfFields.push(pr);
        }
        return Promise.all(listOfFields).then((arr) => {
            if (!arr.every((v) => {
                return isValue(v) && parseInt(v.value) !== -1
            })) {
                return none
            }
            return some(parser(arr))
        });
    }
}