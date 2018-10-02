export function fromHex(str: string) {
    return Buffer.from(str, 'hex').toString();
}

export function toHex(str: string) {
    return Buffer.from(str).toString('hex').toUpperCase();
}