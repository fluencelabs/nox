import { getContract } from './';
import Web3 from "web3";
import { Network } from "../../types/web3-contracts/Network";

export const contractAddress: string = '0xaa6329b04577faf07672f099fa0ea12e7dd32fa1';

const web3 = (window as any).web3;
const web3js = new Web3(web3.currentProvider);
const contract = getContract(contractAddress, web3js);

export default contract as Network;
