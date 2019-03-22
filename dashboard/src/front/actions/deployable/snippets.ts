import {APP_DEPLOYED, APP_ENQUEUED} from "./deploy";

export default (state = {}, action: any) => {
    switch (action.type) {
        case APP_DEPLOYED:
        case APP_ENQUEUED: {
            return {
                ...state,
                app: action.app,
                appId: action.appId,
            };
        }
        default: {
            return state;
        }
    }
};