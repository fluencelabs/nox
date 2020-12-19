import 'mocha';
import Fluence from "../fluence";

describe("== AST parsing suite", () => {
    it("parse simple script and return ast", async function () {
        let ast = await Fluence.parseAIR(`
            (call node ("service" "function") [1 2 3 arg] output)
        `);

        console.log(ast);
    })
})

