import {NodeAppStatusInfo} from "../../../fluence/nodes";
import {APP_DEPLOYED, APP_ENQUEUED} from "./deploy";

export default (state = {}, action: any) => {
    console.log("Reducer: " + action.type);
    switch (action.type) {
        case APP_DEPLOYED:
        case APP_ENQUEUED: {
            let appId: number | undefined;
            if (action.appStatus != undefined)
                appId = action.appStatus.appId;
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