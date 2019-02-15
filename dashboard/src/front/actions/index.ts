import loading, { displayLoading, hideLoading } from './loading';
import nodeIds, { retrieveNodeIds } from './nodes/node-ids';
import nodes, { retrieveNodes } from './nodes/nodes';
import appIds, { retrieveAppIds } from './apps/app-ids';
import apps, { retrieveApps } from './apps/apps';

export {
    displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
    retrieveNodes,
    retrieveApps,
}

export const reducers = {
    loading,
    nodeIds,
    appIds,
    nodes,
    apps,
};
