import * as React from 'react';
import {connect} from 'react-redux';
import {Link} from "react-router-dom";
import FluenceId from '../fluence-id';
import {
    deleteNode,
    displayLoading,
    hideLoading,
    retrieveNode,
    retrieveNodeStatus
} from '../../actions';
import {NodeId, Node, NodeInfo} from "../../../fluence";
import {Action} from "redux";
import {History} from "history";

interface State {
    deleting: boolean;
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
    deleteNode: (nodeId: NodeId, history: History) => Promise<Action>;
    retrieveNodeStatus: (node: Node) => Promise<Action>,
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    history: History;
    userAddress: string;
}

class FluenceNode extends React.Component<Props, State> {
    state: State = {
        deleting: false,
    };

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

    deleteNode = (): void => {
        if (!confirm('Are you sure you want to delete this node?')) return;

        this.props.displayLoading();
        this.setState({
            deleting: true
        });
        this.props.deleteNode(this.props.nodeId, this.props.history).finally(() => {
            this.props.hideLoading();
            this.setState({
                deleting: false
            });
        });
    };

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
                    <p className="text-muted"><FluenceId entityId={node.tendermint_key}/></p>
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
                    <p className="text-muted">
                        <FluenceId
                            entityId={node.owner}
                            isLink={true}
                            href={'https://rinkeby.etherscan.io/address/' + node.owner}
                            className="etherscan-link"
                            target="_blank"/>
                    </p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Is Private</strong>
                    <p className="text-muted">{node.is_private.toString()}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster</strong>
                    <p className="text-muted">
                        {node.clusters_ids.map((appId, i, a) => {
                            return <span><Link to={`/app/${appId}`}>{appId}</Link>{i+1 < a.length && ', '}</span>;
                        })}
                    </p>

                    {nodeStatus && this.renderNodeStatus(nodeStatus)}

                    <hr/>
                    <button type="button"
                            onClick={this.deleteNode}
                            style={{display: node.owner.toUpperCase() === this.props.userAddress.toUpperCase() ? 'block' : 'none'}}
                            disabled={this.state.deleting}
                            className="btn btn-block btn-danger"
                    >
                        Delete node <i style={{display: (this.state.deleting) ? 'inline-block' : 'none'}} className="fa fa-refresh fa-spin"/>
                    </button>
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const node = this.props.nodes[this.props.nodeId];
        const nodeStatus = this.props.nodesStatus[this.props.nodeId];
        return (
            <div className="box box-widget widget-user-2">
                <div className="widget-user-header bg-fluence-blue-gradient">
                    <div className="widget-user-image">
                        <span className="entity-info-box-icon"><i
                            className={node && nodeStatus ? 'ion ion-android-laptop' : 'fa fa-refresh fa-spin'}></i></span>
                    </div>
                    <h3 className="widget-user-username">Node</h3>
                    <h5 className="widget-user-desc">ID:&nbsp; <FluenceId entityId={this.props.nodeId}/></h5>
                </div>
                {node && this.renderNodeInfo(node, nodeStatus)}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    nodes: state.nodes,
    nodesStatus: state.nodesStatus,
    userAddress: state.ethereumConnection.userAddress,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNode,
    retrieveNodeStatus,
    deleteNode
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceNode);
