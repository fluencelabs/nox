import { Dispatch, Action } from 'redux';
import { ThunkAction } from 'redux-thunk';
import { History } from 'history';
import { deleteNode as deleteNodeMethod, NodeId } from '../../../fluence';
import { getContract } from '../../../fluence/contract';
import { ReduxState } from '../../app';

export const DELETE_NODE = 'DELETE_NODE';
export const DELETE_NODE_FAILED = 'DELETE_NODE_FAILED';

export const deleteNode = (nodeId: NodeId, history: History): ThunkAction<void, ReduxState, void, Action<string>> => {
    return async (dispatch: Dispatch): Promise<Action> => {
        const deleteResult = await deleteNodeMethod(getContract(), nodeId);

        if (deleteResult) {
            history.push(`/account`);

            return dispatch({
                type: DELETE_NODE,
                nodeId,
            });
        }

        return dispatch({
            type: DELETE_NODE_FAILED,
            nodeId,
        });
    };
};

