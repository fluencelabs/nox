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

import {Session} from "../Session";
import {TendermintClient} from "../TendermintClient";
import {Engine} from "../Engine";
import {Signer} from "../Signer";
import {Client} from "../Client";
import {empty, error, isValue, Result} from "../Result";
import {none, Option, some} from "ts-option";

enum Condition {
    Less = "-1",
    More = "1",
    Equal = "0"
}

enum Field {
    Id = "0",
    Symbol = "1",
    Price = "2",
    Month = "3"
}

class DbOnPointers {

    private session: Session;

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

    async test() {
        let all = await this.setQueryWildcard();
        console.log("\n=================================================\n");
        console.log("select * from Quotations;");
        printQuantations(all);

        console.log("\n=================================================\n");

        let lessThen500 = await this.setQueryWildCardWhere(Field.Price, Condition.Less, "500");
        console.log("select * from Quotations where price < 500;");
        printQuantations(lessThen500);

        console.log("\n=================================================\n");

        let equals6700 = await this.setQueryWildCardWhere(Field.Price, Condition.Equal, "6700");
        console.log("select * from Quotations where price = 6700;");
        printQuantations(equals6700);

        console.log("\n=================================================\n");

        let countAll = await this.setCount();
        console.log("select count(*) from Quotations;");
        console.log(JSON.stringify(countAll));

        console.log("\n=================================================\n");

        let whereSymbol2 = await this.setCountWhere(Field.Symbol, Condition.Equal, "2");
        console.log("select count(*) from Quotations where symbol = 2;");
        console.log(JSON.stringify(whereSymbol2));

        console.log("\n=================================================\n");

        let averagePrice = await this.setAverage(Field.Price);
        console.log("select avg(price) from Quotations;");
        console.log(JSON.stringify(averagePrice));

        console.log("\n=================================================\n");

        let averagePriceWhere = await this.setAverageWhere(Field.Price, Field.Symbol, Condition.Equal, "2");
        console.log("select avg(price) from Quotations where symbol = 2;");
        console.log(JSON.stringify(averagePriceWhere));

        console.log("\n=================================================\n");
    }

    async setAverage(field: Field, fetch: number = 2): Promise<number> {
        this.session.invoke("set_average_query", [field]);

        return await this.getAllRows(fetch, 1, singleFloat).then((arr) => arr[0]);
    }

    async setAverageWhere(fieldToCount: Field, field: Field, condition: Condition, quantity: string, fetch: number = 2): Promise<number> {
        this.session.invoke("set_average_query_where", [fieldToCount, field, quantity, condition]);

        return await this.getAllRows(fetch, 1, singleFloat).then((arr) => arr[0]);
    }

    async setCount(fetch: number = 2): Promise<number> {
        this.session.invoke("set_count_query");

        return await this.getAllRows(fetch, 1, singleInt).then((arr) => arr[0]);
    }

    async setCountWhere(field: Field, condition: Condition, quantity: string, fetch: number = 2): Promise<number> {
        this.session.invoke("set_count_query_where", [field, quantity, condition]);

        return await this.getAllRows(fetch, 1, singleInt).then((arr) => arr[0]);
    }

    async setQueryWildCardWhere(field: Field, condition: Condition, quantity: string, fetch: number = 5): Promise<Quantation[]> {
        this.session.invoke("set_query_wildcard_where", [field, quantity, condition]);

        return await this.getAllRows(fetch, 4, toQuantation);
    }

    async setQueryWildcard(fetch: number = 10): Promise<Quantation[]> {
        this.session.invoke("set_query_wildcard");

        return this.getAllRows(fetch, 4, toQuantation);
    }

    async getAllRows<T>(fetch: number, fields: number, parser: (arr: Result[]) => T): Promise<T[]> {
        return this.getRows(fetch, fields, parser);
    }

    async getRows<T>(fetch: number, fields: number, parser: (arr: Result[]) => T): Promise<T[]> {
        let all: Promise<Option<T>>[] = await this.getRows0(fetch, fields, parser, []);

        return Promise.all(all)
            .then((arr) =>
                arr.filter((v) => v.nonEmpty)
                    .map((v) => v.get)
            );
    }

    async getRows0<T>(fetch: number, fields: number, parser: (arr: Result[]) => T, acc: Promise<Option<T>>[]): Promise<Promise<Option<T>>[]> {
        var fetched: [boolean, Promise<Option<T>>[]] = await this.fetchRows(fetch, fields, parser);

        var cont: boolean = fetched[0];

        let allRows: Promise<Option<T>>[] = fetched[1];

        if (cont) {
            return this.getRows0(fetch, fields, parser, allRows);
        } else {
            return acc.concat(allRows);
        }
    }

    async fetchRows<T>(fetch: number, fields: number, parser: (arr: Result[]) => T): Promise<[boolean, Promise<Option<T>>[]]> {
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

    async getRow<T>(fields: number, parser: (arr: Result[]) => T): Promise<Option<T>> {

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

interface Quantation {
    id: number
    symbol: number
    price: number
    month: number
}

function printQuantations(arr: Quantation[]) {
    arr.forEach((v) =>{
        console.log(JSON.stringify(v))
    })
}

function singleInt(arr: Result[]): number {
    if (arr.length != 1) throw error("Wrong format. Length of values mast be 4");
    let v = arr[0];
    if (isValue(v)) {
        return parseInt(v.value)
    } else {
        throw error("Should be non-empty resul.")
    }
}

function singleFloat(arr: Result[]): number {
    if (arr.length != 1) throw error("Wrong format. Length of values mast be 4");
    let v = arr[0];
    if (isValue(v)) {
        return parseFloat(v.value)
    } else {
        throw error("Should be non-empty resul.")
    }
}

function toQuantation(arr: Result[]): Quantation {
    if (arr.length != 4) throw error("Wrong format. Length of values mast be 4");
    if (!arr.every((v) => isValue(v))) throw error("Empty response in result.");
    let mapped = arr.map((v) => {
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



const _global = (window /* browser */ || global /* node */) as any;
_global.DbOnPointers = DbOnPointers;