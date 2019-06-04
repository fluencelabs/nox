import {getNode, NodeId} from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import {DELETE_NODE} from "./delete-node";

export const GET_NODE_RECEIVE = 'GET_NODE_RECEIVE';

export const retrieveNode = (nodeId: NodeId) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const node = await getNode(getContract(), nodeId);

        return dispatch({
            type: GET_NODE_RECEIVE,
            node,
        });
    };
};

/*
 * Reducer
 */
export default (state: {[key: string]: Node;} = {}, action: any) => {
    switch (action.type) {
        case GET_NODE_RECEIVE: {
            return {
                ...state,
                [action.node.id]: action.node
            };
        }
        case DELETE_NODE: {
            delete state[action.nodeId];
            return {
                ...state
            };
        }
        default: {
            return state;
        }
    }
};
