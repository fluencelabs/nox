import { getEthConnectionState, EthConnectionState } from '../../fluence/contract';

export const UPDATE_ETH_CONNECTION_STATE = 'UPDATE_ETH_CONNECTION_STATE';
export const updateEthereumConnectionState = (ethConnectionState: EthConnectionState) => {
    return {
        type: UPDATE_ETH_CONNECTION_STATE,
        ethConnectionState
    };
};

/*
 * Reducer
 */
export default (state: EthConnectionState = getEthConnectionState(), action: any) => {
    switch (action.type) {
        case UPDATE_ETH_CONNECTION_STATE: {
            return {
                ...action.ethConnectionState
            };
        }
        default: {
            return state;
        }
    }
};
