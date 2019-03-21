import * as React from 'react';
import {connect} from 'react-redux';
import {cutId} from '../../../utils';
import {
    displayLoading,
    hideLoading,
    retrieveNode,
    retrieveNodeStatus
} from '../../actions';
import {NodeId, Node, NodeInfo} from "../../../fluence";
import {Action} from "redux";

interface State {
}

interface Props {
    nodeId: NodeId,
    nodes: {
        [key: string]: Node
    },
    nodesStatus: {
        [key: string]: NodeInfo
    },
    retrieveNode: (nodeId: NodeId) => Promise<Action>,
    retrieveNodeStatus: (node: Node) => Promise<Action>,
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
}

class FluenceNode extends React.Component<Props, State> {
    state: State = {};

    loadData(): void {
        this.props.displayLoading();
        this.props.retrieveNode(this.props.nodeId)
            .then(() => {
                const node = this.props.nodes[this.props.nodeId];
                return this.props.retrieveNodeStatus(node);
            })
            .then(this.props.hideLoading)
            .catch(this.props.hideLoading);

    }

    componentDidMount(): void {
        this.loadData();
    }

    componentDidUpdate(prevProps: Props): void {
        if (prevProps.nodeId != this.props.nodeId) {
            this.loadData();
        }
    }

    renderNodeStatus(nodeStatus: NodeInfo): React.ReactNode {
        if (nodeStatus.status) {
            return (
                <div>
                    <hr/>
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Node status</strong>
                    {/*<p className="text-muted">IP: {nodeStatus.status.ip}</p>*/}
                    {/*<p className="text-muted">Uptime: {nodeStatus.status.uptime}</p>*/}
                    <p className="text-muted">Workers count: {nodeStatus.status.numberOfWorkers}</p>
                </div>
            );
        } else if (nodeStatus.causeBy) {
            return (
                <div>
                    <hr/>
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Node status: error</strong>
                    <p className="text-muted">{nodeStatus.causeBy.toString()}</p>
                </div>
            );
        }
    }

    renderNodeInfo(node: Node, nodeStatus: NodeInfo): React.ReactNode {
        return (
            <div className="box-footer no-padding">
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Tendermint Key</strong>
                    <p className="text-muted" title={node.tendermint_key}>{cutId(node.tendermint_key)}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Ip Address</strong>
                    <p className="text-muted">{node.ip_addr}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Api Port</strong>
                    <p className="text-muted">{node.api_port}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Capacity</strong>
                    <p className="text-muted">{node.capacity}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Owner</strong>
                    <p className="text-muted" title={node.owner}><a
                        href={'https://rinkeby.etherscan.io/address/' + node.owner} title={node.owner}
                        className="etherscan-link" target="_blank">{cutId(node.owner)}</a></p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Is Private</strong>
                    <p className="text-muted">{node.is_private.toString()}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster</strong>
                    <p className="text-muted">{node.clusters_ids.join(', ')}</p>

                    {nodeStatus && this.renderNodeStatus(nodeStatus)}
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const node = this.props.nodes[this.props.nodeId];
        const nodeStatus = this.props.nodesStatus[this.props.nodeId];
        return (
            <div className="col-md-4 col-xs-12">
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-fluence-blue-gradient">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon"><i
                                className={node && nodeStatus ? 'ion ion-android-laptop' : 'fa fa-refresh fa-spin'}></i></span>
                        </div>
                        <h3 className="widget-user-username">Node</h3>
                        <h5 className="widget-user-desc"
                            title={this.props.nodeId}>ID:&nbsp;{cutId(this.props.nodeId)}</h5>
                    </div>
                    {node && this.renderNodeInfo(node, nodeStatus)}
                </div>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    nodes: state.nodes,
    nodesStatus: state.nodesStatus,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNode,
    retrieveNodeStatus
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceNode);
