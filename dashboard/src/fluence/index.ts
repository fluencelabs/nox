export function getContract(address: string, abi: any, web3js: any) {
    return new web3js.eth.Contract(abi, address);
}

export * from './apps';
export * from './nodes';