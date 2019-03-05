import {getNode, getNodeAppStatus, NodeId, AppId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import {NodeAppStatusInfo} from "../../../fluence/nodes";

export const GET_NODES_APP_STATUS_RECEIVE = 'GET_NODES_APP_STATUS_RECEIVE';

export const retrieveNodesAppStatus = (nodeIds: NodeId[], appId: AppId) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const nodes = await Promise.all(
            nodeIds.map(nodeId => getNode(contract, nodeId))
        );

        const nodesAppStatus = await Promise.all(
            nodes.map(node => getNodeAppStatus(node, appId))
        );

        return dispatch({
            type: GET_NODES_APP_STATUS_RECEIVE,
            nodesAppStatus,
            appId: appId,
        });
    };
};

/*
 * Reducer
 */
export default (state = {}, action: any) => {
    switch (action.type) {
        case GET_NODES_APP_STATUS_RECEIVE: {
            return {
                ...state,
                [action.appId]: action.nodesAppStatus.reduce((hash: any, nodeAppStatus: NodeAppStatusInfo) => {
                    hash[nodeAppStatus.nodeInfo.id] = nodeAppStatus.status;
                    return hash;
                }, {})
            };
        }
        default: {
            return state;
        }
    }
};
