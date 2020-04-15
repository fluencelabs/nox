const ipfsClient = require('ipfs-http-client');
const Hash = require('ipfs-only-hash');

export async function calcHash(data: Buffer): Promise<string> {
    return await Hash.of(data);
}

export async function ipfsAdd(multiaddr: string, file: Uint8Array): Promise<void> {
    const ipfs = ipfsClient(multiaddr);
    const source = ipfs.add(
        [file]
    );

    try {
        for await (const file of source) {
            console.log("uploaded:");
            console.log(file);
        }
    } catch (err) {
        console.error(err)
    }

    return Promise.resolve();
}

export async function ipfsGet(multiaddr: string, path: string): Promise<Uint8Array> {
    const ipfs = ipfsClient(multiaddr);
    const source = ipfs.cat(path);

    let bytes = new Uint8Array();

    try {
        for await (const chunk of source) {
            const newArray = new Uint8Array(bytes.length + chunk.length);
            newArray.set(bytes, 0);
            newArray.set(chunk, bytes.length);

            bytes = newArray;
        }
    } catch (err) {
        console.error(err)
    }

    return Promise.resolve(bytes);
}
