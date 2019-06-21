import {getAppIds} from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import {Action, Dispatch} from 'redux';
import {DELETE_APP} from "./delete-app";

export const GET_APPS_IDS_RECEIVE = 'GET_APPS_IDS_RECEIVE';

export const retrieveAppIds = () => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const appIds = await getAppIds(getContract());
        return dispatch({
            type: GET_APPS_IDS_RECEIVE,
            appIds,
        });
    };
};

/*
 * Reducer
 */
export default (state = [], action: any) => {
    switch (action.type) {
        case GET_APPS_IDS_RECEIVE: {
            return action.appIds;
        }
        case DELETE_APP: {
            return state.filter(appId => appId != action.appId);
        }
        default: {
            return state;
        }
    }
};
