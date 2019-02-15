import { Network } from '../../types/web3-contracts/Network';
import abi from '../abi/Network.json';

export function getContract(address: string, web3js: any): Network {
    return new web3js.eth.Contract(abi, address) as Network;
}

export * from './apps';
export * from './nodes';