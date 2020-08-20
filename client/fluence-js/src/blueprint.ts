export interface Blueprint {
    dependencies: string[],
    id: string,
    name: string
}

export function checkBlueprint(b: any): b is Blueprint {
    if (!b.id) throw new Error(`There is no 'id' field in Blueprint struct: ${JSON.stringify(b)}`)
    if (b.dependencies) {
        b.dependencies.forEach((dep: any) => {
            if ((typeof dep) !== 'string') {
                throw Error(`'dependencies' should be an array of strings: ${JSON.stringify(b)}`)
            }
        });
    }

    return true;

}