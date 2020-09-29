

const rust = import('../../../aquamarine/pkg');

let arr = Uint8Array.from([5,5,5])

export async function run() {
    return rust
        .then(m => console.log(m.invoke("123", "123", arr)))
        .catch(console.error);
}
