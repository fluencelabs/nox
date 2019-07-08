import { Action, Dispatch } from 'redux';
import { ThunkAction } from 'redux-thunk';
import { getAppRefs } from '../../../fluence';
import { getDashboardContract} from '../../../fluence/contract';
import { DELETE_APP } from './delete-app';
import { AppRef } from '../../../fluence/apps';
import { ReduxState } from '../../app';

export type AppRefsState = AppRef[];

const initialState: AppRefsState = [];

export const GET_APPS_REFS_RECEIVE = 'GET_APPS_REFS_RECEIVE';

export const retrieveAppRefs = (): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const appRefs = await getAppRefs(getDashboardContract());

        return dispatch({
            type: GET_APPS_REFS_RECEIVE,
            appRefs,
        });
    };
};

/*
 * Reducer
 */
export default (state = initialState, action: any): AppRefsState => {
    switch (action.type) {
        case GET_APPS_REFS_RECEIVE: {
            return action.appRefs;
        }
        case DELETE_APP: {
            return state.filter(appRef => appRef.app_id !== action.appId);
        }
        default: {
            return state;
        }
    }
};
