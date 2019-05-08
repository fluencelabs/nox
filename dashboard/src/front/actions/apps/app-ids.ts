import {getAppIds, getAppsNew} from '../../../fluence';
import contract, {dashboardContract} from '../../../fluence/contract';
import {Action, Dispatch} from 'redux';

export const GET_APPS_IDS_RECEIVE = 'GET_APPS_IDS_RECEIVE';

export const retrieveAppIds = () => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const appIds = await getAppIds(contract);
        const smallApps = await getAppsNew(dashboardContract);
        console.log("smallApps: " + JSON.stringify(smallApps));
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
