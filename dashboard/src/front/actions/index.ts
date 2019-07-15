import loading, { displayLoading, hideLoading } from './loading';
import nodeIds, { retrieveNodeIds } from './nodes/node-ids';
import nodeRefs, { retrieveNodeRefs } from './nodes/node-refs';
import { deleteNode } from './nodes/delete-node';
import nodes, { retrieveNode } from './nodes/nodes';
import appIds, { retrieveAppIds } from './apps/app-ids';
import appRefs, { retrieveAppRefs } from './apps/app-refs';
import { deleteApp } from './apps/delete-app';
import apps, { retrieveApp } from './apps/apps';
import nodesStatus, { retrieveNodeStatus } from './nodes/node-status';
import nodesAppStatus, { retrieveNodesAppStatus } from './nodes/nodes-app-status';
import deployReducer, { deploy, deployUpload } from './deployable/deploy';
import ethereumConnection, { updateEthereumConnectionState } from './ethereum-connection';
import modal, { showModal, closeModal } from './modal';

export {
    displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveNodeRefs,
    deleteNode,
    retrieveAppIds,
    retrieveAppRefs,
    deleteApp,
    retrieveNode,
    retrieveApp,
    retrieveNodeStatus,
    retrieveNodesAppStatus,
    deploy,
    deployUpload,
    updateEthereumConnectionState,
    showModal,
    closeModal,
};

export const reducers = {
    loading,
    nodeIds,
    nodeRefs,
    appIds,
    appRefs,
    nodes,
    apps,
    nodesStatus,
    nodesAppStatus,
    deploy: deployReducer,
    ethereumConnection,
    modal,
};
