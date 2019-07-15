import { ThunkAction } from 'redux-thunk';
import { getNode, getNodeAppStatus, NodeId, AppId } from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import { NodeAppStatus, NodeAppStatusInfo } from '../../../fluence/nodes';
import { ReduxState } from '../../app';

export type NodesAppStatusState = {
    [key: string]: {
        [key: string]: NodeAppStatus|null;
    };
};

const initialState: NodesAppStatusState = {};

export const GET_NODES_APP_STATUS_RECEIVE = 'GET_NODES_APP_STATUS_RECEIVE';

export const retrieveNodesAppStatus = (nodeIds: NodeId[], appId: AppId): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const nodes = await Promise.all(
            nodeIds.map(nodeId => getNode(getContract(), nodeId))
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
export default (state = initialState, action: any): NodesAppStatusState => {
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
