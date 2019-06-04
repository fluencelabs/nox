import {getApp, AppId, App} from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import {DELETE_APP} from "./delete-app";

export const GET_APP_RECEIVE = 'GET_APP_RECEIVE';

export const retrieveApp = (appId: AppId) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const app = await getApp(getContract(), appId);
        return dispatch({
            type: GET_APP_RECEIVE,
            app,
        });
    };
};

/*
 * Reducer
 */
export default (state: {[key: string]: App;} = {}, action: any) => {
    switch (action.type) {
        case GET_APP_RECEIVE: {
            return {
                ...state,
                [action.app.app_id]: action.app
            };
        }
        case DELETE_APP: {
            delete state[action.appId];
            return {
                ...state
            };
        }
        default: {
            return state;
        }
    }
};
