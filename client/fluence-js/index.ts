import * as stepper from './stepper.js';

console.log("doing smth")

stepper.loadWasm().then((wasm) => {
    console.log(wasm.invoke("123", "456", "Uint8Array.from([8,8,8])"))
})
