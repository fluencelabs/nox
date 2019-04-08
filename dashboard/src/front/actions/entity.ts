import {AppId, NodeId} from "../../fluence";
import {DeployableAppId} from "../../fluence/deployable";

export enum FluenceEntityType {
    None = -1,
    Stub = 'stub',
    App  = 'app',
    Node = 'node',
    DeployableApp = 'deployableApp',
}

export interface FluenceEntity {
    id: NodeId | AppId | DeployableAppId
    type: FluenceEntityType
}

export const SHOW_ENTITY = 'SHOW_ENTITY';
export const showEntity = (entity: FluenceEntity) => {
    return {
        type: SHOW_ENTITY,
        entity: entity
    };
};

/*
 * Reducer
 */
export default (state = { type: FluenceEntityType.Stub }, action: any) => {
    switch (action.type) {
        case SHOW_ENTITY: {
            return {
                ...state,
                ...action.entity
            };
        }
        default: {
            return state;
        }
    }
};
