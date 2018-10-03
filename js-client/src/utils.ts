export function fromHex(str: string): string {
    return Buffer.from(str, 'hex').toString();
}

export function toHex(str: string): string {
    return Buffer.from(str).toString('hex').toUpperCase();
}