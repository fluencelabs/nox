import loading, { displayLoading, hideLoading } from './loading';
import nodeIds, { retrieveNodeIds } from './nodes/node-ids';
import nodes, { retrieveNode } from './nodes/nodes';
import appIds, { retrieveAppIds } from './apps/app-ids';
import apps, { retrieveApp } from './apps/apps';
import nodesStatus, { retrieveNodeStatus } from './nodes/node-status';

export {
    displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
    retrieveNode,
    retrieveApp,
    retrieveNodeStatus,
}

export const reducers = {
    loading,
    nodeIds,
    appIds,
    nodes,
    apps,
    nodesStatus
};
