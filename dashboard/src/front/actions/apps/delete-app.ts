import {deleteApp as deleteAppMethod, AppId} from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import {History} from "history";

export const DELETE_APP = 'DELETE_APP';
export const DELETE_APP_FAILED = 'DELETE_APP_FAILED';

export const deleteApp = (appId: AppId, history: History) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const deleteResult = await deleteAppMethod(getContract(), appId);

        if (deleteResult) {
            history.push(`/account`);
            return dispatch({
                type: DELETE_APP,
                appId,
            });
        }

        return dispatch({
            type: DELETE_APP_FAILED,
            appId,
        });
    };
};

