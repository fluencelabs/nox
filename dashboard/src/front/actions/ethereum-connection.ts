import { getEthConnectionState, EthConnectionState } from '../../fluence/contract';
import { AnyAction } from 'redux';

export const UPDATE_ETH_CONNECTION_STATE = 'UPDATE_ETH_CONNECTION_STATE';
export const updateEthereumConnectionState = (ethConnectionState: EthConnectionState): AnyAction => {
    return {
        type: UPDATE_ETH_CONNECTION_STATE,
        ethConnectionState
    };
};

/*
 * Reducer
 */
export default (state: EthConnectionState = getEthConnectionState(), action: any): EthConnectionState => {
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
