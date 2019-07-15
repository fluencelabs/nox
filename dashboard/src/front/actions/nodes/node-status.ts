import { ThunkAction } from 'redux-thunk';
import { getNodeStatus, Node } from '../../../fluence';
import { Dispatch, Action } from 'redux';
import { NodeInfo } from '../../../fluence/nodes';
import { ReduxState } from '../../app';

export type NodesStatusState = {
    [key: string]: NodeInfo;
};

const initialState: NodesStatusState = {};

export const GET_NODE_STATUS_RECEIVE = 'GET_NODE_STATUS_RECEIVE';

export const retrieveNodeStatus = (node: Node): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const nodeStatus = await getNodeStatus(node);

        return dispatch({
            type: GET_NODE_STATUS_RECEIVE,
            nodeStatus,
        });
    };
};

/*
 * Reducer
 */
export default (state = initialState, action: any): NodesStatusState => {
    switch (action.type) {
        case GET_NODE_STATUS_RECEIVE: {
            return {
                ...state,
                [action.nodeStatus.nodeInfo.id]: action.nodeStatus
            };
        }
        default: {
            return state;
        }
    }
};
