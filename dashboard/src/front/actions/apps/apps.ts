import {getApp, AppId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';

export const GET_APP_RECEIVE = 'GET_APP_RECEIVE';

export const retrieveApp = (appId: AppId) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const app = await getApp(contract, appId);
        return dispatch({
            type: GET_APP_RECEIVE,
            app,
        });
    };
};

/*
 * Reducer
 */
export default (state = {}, action: any) => {
    switch (action.type) {
        case GET_APP_RECEIVE: {
            return {
                ...state,
                [action.app.app_id]: action.app
            };
        }
        default: {
            return state;
        }
    }
};
