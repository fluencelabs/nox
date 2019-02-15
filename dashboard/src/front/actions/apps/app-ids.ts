import {getAppIds, AppId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';

export const GET_APPS_IDS_RECEIVE = 'GET_APPS_IDS_RECEIVE';

export const retrieveAppIds = () => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const appIds = await getAppIds(contract);
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
        default: {
            return state;
        }
    }
};
