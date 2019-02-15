import { getContract } from './';
import Web3 from "web3";
import { Network } from "../../types/web3-contracts/Network";

//const contractAddress: string = '0xd62ab1bd71068a252bb20fa94b2f536d7525cfeb';
const contractAddress: string = '0x5faa7b8d290407460e0ec8585b2712acf27290f9';

const web3 = (window as any).web3;
const web3js = new Web3(web3.currentProvider);
const contract = getContract(contractAddress, web3js);

export default contract as Network;
