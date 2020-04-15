declare module 'ipfs-only-hash' {
    export function of(data: Buffer): Promise<string>
}
