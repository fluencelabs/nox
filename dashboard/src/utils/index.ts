export function cutId(id: string): string {
    if (id.startsWith('0x')) {
        return id.replace(/^(.{8}).+(.{4})$/, '$1...$2');
    } else {
        return id.replace(/^(.{4}).+(.{4})$/, '$1...$2');
    }
}

export function remove0x(hex: string): string {
    if (hex.startsWith("0x")) {
        return hex.slice(2);
    } else {
        return hex;
    }
}