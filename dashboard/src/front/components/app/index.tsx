import * as React from 'react';
import { connect } from 'react-redux';
import { displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
} from '../../actions';
import { contractAddress } from '../../../fluence/contract';
import {App, Node, NodeId, AppId} from '../../../fluence';
import FluenceApp from '../fluence-app';
import FluenceNode from '../fluence-node';
import FluenceNodeStatus from '../fluence-node-status';
import {Action} from "redux";

interface State {}

interface Props {
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    retrieveNodeIds: () => Promise<Action>,
    retrieveAppIds: () => Promise<Action>,
    loading: boolean,
    nodeIds: NodeId[],
    appIds: AppId[],
    apps: {
        [key: string]: App
    };
    nodes: {
        [key: string]: Node
    };
}

class DashboardApp extends React.Component<Props, State>{
    state: State = {};

    componentDidMount(): void {

        this.props.displayLoading();

        Promise.all([
            this.props.retrieveNodeIds(),
            this.props.retrieveAppIds(),
        ]).then(() => {
            this.props.hideLoading();
        }).catch((e) => {
            window.console.log(e);
            this.props.hideLoading();
        });
    }

    render(): React.ReactNode {
        return (
            <div>
                <h2>Fluence network status from contract {contractAddress}</h2>
                <img style={{ display: this.props.loading ? 'block' : 'none' }} src="/static/assets/loader.gif"/>

                <p>Apps ID's: {this.props.appIds.join(', ')}</p>
                <p>Nodes ID's: {this.props.nodeIds.join(', ')}</p>

                {this.props.appIds.map(appId => <FluenceApp appId={appId} />)}
                {this.props.nodeIds.map(nodeId => <FluenceNode nodeId={nodeId} />)}
                {this.props.nodeIds.map(nodeId => this.props.nodes[nodeId] && <FluenceNodeStatus node={this.props.nodes[nodeId]} />)}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    loading: state.loading.isLoading,
    nodeIds: state.nodeIds,
    appIds: state.appIds,
    nodes: state.nodes,
    apps: state.apps,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
};

export default connect(mapStateToProps, mapDispatchToProps)(DashboardApp);
