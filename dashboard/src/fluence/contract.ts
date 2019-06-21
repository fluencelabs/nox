import Web3 from "web3";
import {Network} from "../../types/web3-contracts/Network";
import NetworkABI from '../abi/Network.json';
import {Dashboard} from "../../types/web3-contracts/Dashboard";
import DashboardABI from '../abi/Dashboard.json';
import {dashboardContractAddress, defaultContractAddress, fluenceNodeAddr, rootTagId, account as demoAddress} from '../constants';

const injectedWeb3 = (window as any).web3;

const search = (window as any).location.search;
const urlParams = new URLSearchParams(search);
const contractFromUrl = urlParams.get('contract');

const rootElement = document.getElementById(rootTagId);
const contractFromTag = rootElement ? rootElement.getAttribute('data-contract') : null;

export const contractAddress: string = contractFromUrl ? contractFromUrl : (contractFromTag ? contractFromTag : defaultContractAddress);

export enum MetamaskEvent {
    accountsChanged = 'accountsChanged',
    networkChanged = 'networkChanged',
}

export interface EthConnectionState {
    provider: any;
    isMetamaskProviderActive: boolean;
    web3js: any;
    contract: Network;
    dashboardContract: Dashboard;
    userAddress: string;
}

export type MetamaskEventHandler = (eventType: MetamaskEvent, ethConnectionState: EthConnectionState) => void;

const subscribersList: MetamaskEventHandler[] = [];

const ethConnectionState: EthConnectionState = {} as EthConnectionState;
function updateEthConnectionState() {
    if (typeof injectedWeb3 !== 'undefined') {
        if (injectedWeb3.currentProvider.networkVersion == '4' && injectedWeb3.currentProvider.selectedAddress) { // Rinkeby network
            ethConnectionState.provider = injectedWeb3.currentProvider;
            ethConnectionState.isMetamaskProviderActive = true;
            ethConnectionState.userAddress = injectedWeb3.currentProvider.selectedAddress;
            ethConnectionState.web3js = new Web3(ethConnectionState.provider);
            ethConnectionState.contract = new ethConnectionState.web3js.eth.Contract(NetworkABI, contractAddress) as Network;
            ethConnectionState.dashboardContract =  new ethConnectionState.web3js.eth.Contract(DashboardABI, dashboardContractAddress) as Dashboard;

            return ethConnectionState;
        }
    }

    // Demo mode
    ethConnectionState.provider = new Web3.providers.HttpProvider(fluenceNodeAddr);
    ethConnectionState.isMetamaskProviderActive = false;
    ethConnectionState.userAddress = demoAddress;
    ethConnectionState.web3js = new Web3(ethConnectionState.provider);
    ethConnectionState.contract = new ethConnectionState.web3js.eth.Contract(NetworkABI, contractAddress) as Network;
    ethConnectionState.dashboardContract =  new ethConnectionState.web3js.eth.Contract(DashboardABI, dashboardContractAddress) as Dashboard;

    return ethConnectionState;
}

updateEthConnectionState();

if (typeof injectedWeb3 !== 'undefined') { //Subscribe to events
    injectedWeb3.currentProvider.on(MetamaskEvent.accountsChanged, function (accounts: string[]) {
        console.log('metamask accountsChanged', accounts[0]);
        updateEthConnectionState();
        console.log('subscribersList.length', subscribersList.length);
        subscribersList.forEach(s => s(MetamaskEvent.accountsChanged, ethConnectionState));
    });
    injectedWeb3.currentProvider.on(MetamaskEvent.networkChanged, function (network: string) {
        console.log('metamask networkChanged', network);
        updateEthConnectionState();
        subscribersList.forEach(s => s(MetamaskEvent.networkChanged, ethConnectionState));
    });
}

export function getEthConnectionState(): EthConnectionState {
    return ethConnectionState;
}

export function getUserAddress(): string {
    return ethConnectionState.userAddress;
}

export function getContract(): Network {
    return ethConnectionState.contract;
}

export function getDashboardContract(): Dashboard {
    return ethConnectionState.dashboardContract;
}

export function isMetamaskActive(): boolean {
    return ethConnectionState.isMetamaskProviderActive;
}

export function getProvider(): any {
    return ethConnectionState.provider;
}

export function getWeb3Js(): any {
    return ethConnectionState.web3js;
}

export function subscribeToMetamaskEvents(callback: MetamaskEventHandler) {
    subscribersList.push(callback);
}
