import {NodeAppStatusInfo} from "../../../fluence/nodes";
import {APP_DEPLOYED, APP_ENQUEUED} from "./deploy";

export default (state = {}, action: any) => {
    switch (action.type) {
        case APP_DEPLOYED:
        case APP_ENQUEUED: {
            let appId: number | undefined;
            if (action.appStatus != undefined)
                appId = action.appStatus.app_id;
            else
                appId = undefined;

            return {
                ...state,
                app: action.app,
                appId: appId,
            };
        }
        default: {
            return state;
        }
    }
};