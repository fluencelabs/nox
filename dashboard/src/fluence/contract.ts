import { getContract } from './';
import Web3 from "web3";
import { Network } from "../../types/web3-contracts/Network";
import { defaultContractAddress, rootTagId } from '../constants';

const search = (window as any).location.search;
const urlParams = new URLSearchParams(search);
const contractFromUrl = urlParams.get('contract');

const rootElement = document.getElementById(rootTagId);
const contractFromTag = rootElement ? rootElement.getAttribute('data-contract') : null;

export const contractAddress: string = contractFromUrl ? contractFromUrl : (contractFromTag ? contractFromTag : defaultContractAddress);

const web3 = (window as any).web3;
const web3js = new Web3(web3.currentProvider);
const contract = getContract(contractAddress, web3js);

export default contract as Network;
