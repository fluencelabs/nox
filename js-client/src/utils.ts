export function fromHexToView(str: string): DataView {
    let buf = Buffer.from(str, 'hex');

    return new DataView(buf.buffer, 0);
}

export function fromHex(str: string): string {
    return Buffer.from(str, 'hex').toString();
}

export function toHex(str: string): string {
    return Buffer.from(str).toString('hex').toUpperCase();
}
