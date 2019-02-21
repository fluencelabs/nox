import {getNode, NodeId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';

export const GET_NODE_RECEIVE = 'GET_NODE_RECEIVE';

export const retrieveNode = (nodeId: NodeId) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const node = await getNode(contract, nodeId);

        return dispatch({
            type: GET_NODE_RECEIVE,
            node,
        });
    };
};

/*
 * Reducer
 */
export default (state = {}, action: any) => {
    switch (action.type) {
        case GET_NODE_RECEIVE: {
            return {
                ...state,
                [action.node.id]: action.node
            };
        }
        default: {
            return state;
        }
    }
};
