export function cutId(id: string): string {
    if (id.startsWith('0x')) {
        return id.replace(/^(.{8}).+(.{4})$/, '$1...$2');
    } else {
        return id.replace(/^(.{4}).+(.{4})$/, '$1...$2');
    }
}