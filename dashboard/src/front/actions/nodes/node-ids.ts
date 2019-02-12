import {getNodeIds, NodeId} from '../../../fluence';
import contract from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';

export const GET_NODES_IDS_RECEIVE = 'GET_NODES_IDS_RECEIVE';

export const retrieveNodeIds = () => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const nodeIds = await getNodeIds(contract);
        return dispatch({
            type: GET_NODES_IDS_RECEIVE,
            nodeIds,
        });
    };
};

/*
 * Reducer
 */
export default (state = [], action: any) => {
    switch (action.type) {
        case GET_NODES_IDS_RECEIVE: {
            return action.nodeIds;
        }
        default: {
            return state;
        }
    }
};
