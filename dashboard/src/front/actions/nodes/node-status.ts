import {getNodeStatus, Node} from '../../../fluence';
import { Dispatch, Action } from 'redux';

export const GET_NODE_STATUS_RECEIVE = 'GET_NODE_STATUS_RECEIVE';

export const retrieveNodeStatus = (node: Node) => {
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
export default (state = {}, action: any) => {
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
