import { getNodeIds, NodeId } from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { Dispatch, Action } from 'redux';
import { ThunkAction } from 'redux-thunk';
import { ReduxState } from '../../app';

export type NodeIdsState = NodeId[];

const initialState: NodeIdsState = [];

export const GET_NODES_IDS_RECEIVE = 'GET_NODES_IDS_RECEIVE';

export const retrieveNodeIds = (): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const nodeIds = await getNodeIds(getContract());

        return dispatch({
            type: GET_NODES_IDS_RECEIVE,
            nodeIds,
        });
    };
};

/*
 * Reducer
 */
export default (state = initialState, action: any): NodeIdsState  => {
    switch (action.type) {
        case GET_NODES_IDS_RECEIVE: {
            return action.nodeIds;
        }
        default: {
            return state;
        }
    }
};
