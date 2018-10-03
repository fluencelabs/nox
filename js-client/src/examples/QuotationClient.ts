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

import {error, isValue, Result} from "../Result";
import {Condition, DbClient, Field, Query} from "./DbClient";

export class QuotationClient {

    dbClient: DbClient;

    constructor(host: string, port: number) {
        this.dbClient = new DbClient(host, port)
    }

    /**
     * select avg(field) from Quotations;
     * ex: select avg(price) from Quotations;
     */
    async average(field: Field, fetch: number = 2): Promise<number> {
        let q: Query = {query: "set_average_query", args: [field]};

        return await this.dbClient.getResult(q, fetch, 1, singleFloat).then((arr) => arr[0]);
    }

    /**
     * select avg(fieldToCount) from Quotations where field condition quantity;
     * ex: select avg(price) from Quotations where symbol = 2;
     */
    async averageWhere(fieldToCount: Field, field: Field, condition: Condition, quantity: string, fetch: number = 2): Promise<number> {
        let q: Query = {query: "set_average_query_where", args: [fieldToCount, field, quantity, condition]};

        return await this.dbClient.getResult(q, fetch, 1, singleFloat).then((arr) => arr[0]);
    }

    /**
     * select count(*) from Quotations;
     */
    async count(fetch: number = 2): Promise<number> {
        let q: Query = {query: "set_count_query", args: []};

        return await this.dbClient.getResult(q, fetch, 1, singleInt).then((arr) => arr[0]);
    }

    /**
     * select count(*) from Quotations where field condition quantity;
     * ex: select count(*) from Quotations where symbol = 2;
     */
    async countWhere(field: Field, condition: Condition, quantity: string, fetch: number = 2): Promise<number> {
        let q: Query = {query: "set_count_query_where", args: [field, quantity, condition]};

        return await this.dbClient.getResult(q, fetch, 1, singleInt).then((arr) => arr[0]);
    }

    /**
     * select * from Quotations where field condition quantity;
     * ex: select * from Quotations where price = 6700;
     */
    async queryWildCardWhere(field: Field, condition: Condition, quantity: string, fetch: number = 5): Promise<Quotation[]> {
        let q: Query = {query: "set_query_wildcard_where", args: [field, quantity, condition]};

        return await this.dbClient.getResult(q, fetch, 4, toQuotation);
    }

    /**
     * select * from Quotations;
     * @param fetch how many rows to request at a time
     */
    async queryWildcard(fetch: number = 10): Promise<Quotation[]> {
        let q: Query = {query: "set_query_wildcard", args: []};

        return this.dbClient.getResult(q, fetch, 4, toQuotation);
    }
}

export interface Quotation {
    id: number
    symbol: number
    price: number
    month: number
}

/**
 * Print all quotations in console.
 */
export function printQuotations(arr: Quotation[]) {
    arr.forEach((v) =>{
        console.log(JSON.stringify(v))
    })
}

/**
 * Parse array of fields as one integer number.
 *
 * @param fields an array of results of fields requests
 */
function singleInt(fields: Result[]): number {
    if (fields.length != 1) throw error("Wrong format. Length of values must be 1 for a single int response.");
    let v = fields[0];
    if (isValue(v)) {
        return parseInt(v.value)
    } else {
        throw error("Should be non-empty result.")
    }
}

/**
 * Parse array of fields as one float number.
 *
 * @param fields an array of results of fields requests
 */
function singleFloat(fields: Result[]): number {
    if (fields.length != 1) throw error("Wrong format. Length of values must be 1 for a single float response.");
    let v = fields[0];
    if (isValue(v)) {
        return parseFloat(v.value)
    } else {
        throw error("Should be non-empty result.")
    }
}

/**
 * Parse array of fields as a Quotation interfact.
 *
 * @param fields an array of results of fields requests
 */
export function toQuotation(fields: Result[]): Quotation {
    if (fields.length != 4) throw error("Wrong format. Length of values must be 4 for the parsing of quotation.");

    let mapped = fields.map((v) => {
        if (isValue(v)) {
            return v;
        } else {
            throw error("Empty response in result.");
        }
    });
    return {
        id: parseInt(mapped[0].value),
        symbol: parseInt(mapped[1].value),
        price: parseFloat(mapped[2].value),
        month: parseInt(mapped[3].value)
    }
}
