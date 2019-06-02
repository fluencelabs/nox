import {getAppRefs} from '../../../fluence';
import {getDashboardContract} from '../../../fluence/contract';
import {Action, Dispatch} from 'redux';
import {DELETE_APP} from "./delete-app";
import {AppRef} from "../../../fluence/apps";

export const GET_APPS_REFS_RECEIVE = 'GET_APPS_REFS_RECEIVE';

export const retrieveAppRefs = () => {
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
export default (state: AppRef[] = [], action: any) => {
    switch (action.type) {
        case GET_APPS_REFS_RECEIVE: {
            return action.appRefs;
        }
        case DELETE_APP: {
            return state.filter(appRef => appRef.app_id != action.appId);
        }
        default: {
            return state;
        }
    }
};
