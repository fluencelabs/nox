import {getNodes, NodeId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';

export const GET_NODES_RECEIVE = 'GET_NODES_RECEIVE';

export const retrieveNodes = (nodeIds: NodeId[]) => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const nodes = await getNodes(contract, nodeIds);

        return dispatch({
            type: GET_NODES_RECEIVE,
            nodes,
        });
    };
};

/*
 * Reducer
 */
export default (state = [], action: any) => {
    switch (action.type) {
        case GET_NODES_RECEIVE: {
            return action.nodes;
        }
        default: {
            return state;
        }
    }
};
