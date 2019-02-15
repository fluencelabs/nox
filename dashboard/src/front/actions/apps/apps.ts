import {getApps, AppId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';

export const GET_APPS_RECEIVE = 'GET_APPS_RECEIVE';

export const retrieveApps = (appIds: AppId[]) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const apps = await getApps(contract, appIds);
        return dispatch({
            type: GET_APPS_RECEIVE,
            apps,
        });
    };
};

/*
 * Reducer
 */
export default (state = [], action: any) => {
    switch (action.type) {
        case GET_APPS_RECEIVE: {
            return action.apps;
        }
        default: {
            return state;
        }
    }
};
