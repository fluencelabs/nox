import * as React from 'react';
import { connect } from 'react-redux';
import {
    displayLoading,
    hideLoading,
    retrieveNode,
} from '../../actions';
import {NodeId, Node} from "../../../fluence";
import {Action} from "redux";

interface State {}

interface Props {
    nodeId: NodeId,
    nodes: {
        [key: string]: Node
    },
    retrieveNode: (nodeId: NodeId) => Promise<Action>,
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
}

class FluenceNode extends React.Component<Props, State> {
    state: State = {};

    componentDidMount(): void {
        this.props.displayLoading();
        this.props.retrieveNode(this.props.nodeId).then(this.props.hideLoading).catch(this.props.hideLoading);
    }

    renderNodeInfo(node: Node): React.ReactNode {
        return (
            <div className="box-footer no-padding">
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Tendermint Key</strong>
                    <p className="text-muted">{node.tendermint_key}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Ip Address</strong>
                    <p className="text-muted">{node.ip_addr}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Next Port</strong>
                    <p className="text-muted">{node.next_port}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Last Port</strong>
                    <p className="text-muted">{node.last_port}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Owner</strong>
                    <p className="text-muted">{node.owner}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Is Private</strong>
                    <p className="text-muted">{node.is_private.toString()}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster</strong>
                    <p className="text-muted">{node.clusters_ids.join(', ')}</p>
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const node = this.props.nodes[this.props.nodeId];

        return (
            <div className="col-md-4">
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-aqua">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon"><i className={node ? 'ion ion-android-laptop' : 'fa fa-refresh fa-spin'}></i></span>
                        </div>
                        <h3 className="widget-user-username">Node</h3>
                        <h5 className="widget-user-desc">ID:&nbsp;{this.props.nodeId}</h5>
                    </div>
                    { node && this.renderNodeInfo(node) }
                </div>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    nodes: state.nodes,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNode,
};

export default connect(mapStateToProps ,mapDispatchToProps)(FluenceNode);
