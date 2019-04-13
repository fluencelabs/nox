import loading, { displayLoading, hideLoading } from './loading';
import entity, { showEntity } from './entity';
import nodeIds, { retrieveNodeIds } from './nodes/node-ids';
import nodes, { retrieveNode } from './nodes/nodes';
import appIds, { retrieveAppIds } from './apps/app-ids';
import apps, { retrieveApp } from './apps/apps';
import nodesStatus, { retrieveNodeStatus } from './nodes/node-status';
import nodesAppStatus, { retrieveNodesAppStatus } from './nodes/nodes-app-status';
import deployReducer, { restoreDeployed, deploy, deployUpload } from './deployable/deploy';

export {
    displayLoading,
    hideLoading,
    showEntity,
    retrieveNodeIds,
    retrieveAppIds,
    retrieveNode,
    retrieveApp,
    retrieveNodeStatus,
    retrieveNodesAppStatus,
    deploy,
    restoreDeployed,
    deployUpload,
}

export const reducers = {
    loading,
    entity,
    nodeIds,
    appIds,
    nodes,
    apps,
    nodesStatus,
    nodesAppStatus,
    deploy: deployReducer,
};
