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

import {printQuotations, QuotationClient} from "./QuotationClient";
import {Condition, Field} from "./DbClient";

/**
 * Client for the sqldb example https://github.com/fluencelabs/fluence/tree/master/vm/examples/sqldb
 */
class DbOnPointers {

    client: QuotationClient;

    constructor(host: string, port: number) {
        this.client = new QuotationClient(host, port)
    }

    /**
     * Tests all requests for DB on pointers.
     */
    async test() {
        let all = await this.client.queryWildcard();
        console.log("\n=================================================\n");
        console.log("select * from Quotations;");
        printQuotations(all);

        console.log("\n=================================================\n");

        let lessThen500 = await this.client.queryWildCardWhere(Field.Price, Condition.Less, "500");
        console.log("select * from Quotations where price < 500;");
        printQuotations(lessThen500);

        console.log("\n=================================================\n");

        let equals6700 = await this.client.queryWildCardWhere(Field.Price, Condition.Equal, "6700");
        console.log("select * from Quotations where price = 6700;");
        printQuotations(equals6700);

        console.log("\n=================================================\n");

        let countAll = await this.client.count();
        console.log("select count(*) from Quotations;");
        console.log(JSON.stringify(countAll));

        console.log("\n=================================================\n");

        let whereSymbol2 = await this.client.countWhere(Field.Symbol, Condition.Equal, "2");
        console.log("select count(*) from Quotations where symbol = 2;");
        console.log(JSON.stringify(whereSymbol2));

        console.log("\n=================================================\n");

        let averagePrice = await this.client.average(Field.Price);
        console.log("select avg(price) from Quotations;");
        console.log(JSON.stringify(averagePrice));

        console.log("\n=================================================\n");

        let averagePriceWhere = await this.client.averageWhere(Field.Price, Field.Symbol, Condition.Equal, "2");
        console.log("select avg(price) from Quotations where symbol = 2;");
        console.log(JSON.stringify(averagePriceWhere));

        console.log("\n=================================================\n");
    }
}



const _global = (window /* browser */ || global /* node */) as any;
_global.DbOnPointers = DbOnPointers;
