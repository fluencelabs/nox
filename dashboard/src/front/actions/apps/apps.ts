import { Dispatch, Action } from 'redux';
import { ThunkAction } from 'redux-thunk';
import { getApp, AppId, App } from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { DELETE_APP } from './delete-app';
import { ReduxState } from '../../app';

export type AppsState = {
    [key: string]: App;
};

const initialState: AppsState = {};

export const GET_APP_RECEIVE = 'GET_APP_RECEIVE';

export const retrieveApp = (appId: AppId): ThunkAction<void, ReduxState, void, Action<string>> => {
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
export default (state = initialState, action: any): AppsState => {
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
