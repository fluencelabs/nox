import { ThunkAction } from 'redux-thunk';
import { getNode, Node, NodeId } from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import { DELETE_NODE } from './delete-node';
import { ReduxState } from '../../app';

export type NodesState = {
    [key: string]: Node;
};

const initialState: NodesState = {};

export const GET_NODE_RECEIVE = 'GET_NODE_RECEIVE';

export const retrieveNode = (nodeId: NodeId): ThunkAction<void, ReduxState, void, Action<string>> => {
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
export default (state = initialState, action: any): NodesState => {
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
