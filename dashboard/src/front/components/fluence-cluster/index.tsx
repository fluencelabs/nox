import * as React from 'react';
import {connect} from 'react-redux';
import moment from 'moment';
import {
    displayLoading,
    hideLoading,
    retrieveNodesAppStatus,
} from '../../actions';
import {cutId} from '../../../utils';
import {App, AppId, NodeId} from "../../../fluence";
import {Option} from 'ts-option';
import {Cluster} from "../../../fluence/apps";
import {NodeAppStatus} from "../../../fluence/nodes";
import {Action} from "redux";

interface State {
    clusterIsLoading: boolean,
}

interface Props {
    appId: AppId,
    cluster: Option<Cluster>
    retrieveNodesAppStatus: (nodeIds: NodeId[], appId: AppId) => Promise<Action>,
    nodesAppStatus: {
        [key: string]: {
            [key: string]: NodeAppStatus
        }
    }
}

class FluenceCluster extends React.Component<Props, State> {
    state: State = {
        clusterIsLoading: false
    };

    loadClusterInfo = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            clusterIsLoading: true
        });
        this.props.retrieveNodesAppStatus(this.props.cluster.get.node_ids, this.props.appId).then(() => {
            this.setState({
                clusterIsLoading: false
            });
            console.log('this.props.nodesAppStatus', this.props.nodesAppStatus);
        }).catch(() => {
            this.setState({
                clusterIsLoading: false
            });
        });
    };

    renderClusterMemberBadge(id: NodeId): React.ReactNode {
        if (this.props.nodesAppStatus[this.props.appId] && this.props.nodesAppStatus[this.props.appId][id]) {
            const blockHeight = parseInt(this.props.nodesAppStatus[this.props.appId][id].sync_info.latest_block_height);
            const colorStyle = blockHeight >= 2 ? 'bg-green' : 'bg-red';
            return (
                <small className={"label " + colorStyle}>{blockHeight}</small>
            );
        }

        return null;
    }

    renderClusterMember(id: NodeId): React.ReactNode {
        return (
            <li>
                {cutId(id)} {this.renderClusterMemberBadge(id)}
            </li>
        );
    }

    render(): React.ReactNode {
        let clusterInfo = null;
        if (this.props.cluster.isDefined) {
            clusterInfo = (
                <p>
                    <p>
                        <button type="button" onClick={this.loadClusterInfo} className="btn btn-block btn-primary">Check
                            cluster <i style={{display: this.state.clusterIsLoading ? 'inline-block' : 'none'}}
                                       className="fa fa-refresh fa-spin"></i></button>
                    </p>
                    <p>
                        Genesis time: {moment.unix(this.props.cluster.get.genesis_time).format()}
                    </p>
                    <p>
                        Cluster Members:
                        <ul>
                            {this.props.cluster.get.node_ids.map(nodeId => this.renderClusterMember(nodeId))}
                        </ul>
                    </p>
                </p>
            );
        } else {
            clusterInfo = (<span>-</span>);
        }
        return (
            clusterInfo
        );
    }
}

const mapStateToProps = (state: any) => ({
    nodesAppStatus: state.nodesAppStatus
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNodesAppStatus,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceCluster);
