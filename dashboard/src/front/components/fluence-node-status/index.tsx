import * as React from 'react';
import { connect } from 'react-redux';
import {
    displayLoading,
    hideLoading,
    retrieveNodeStatus,
} from '../../actions';
import {Node, NodeInfo} from "../../../fluence";
import {Action} from "redux";

interface State {}

interface Props {
    node: Node,
    nodesStatus: {
        [key: string]: NodeInfo
    },
    retrieveNodeStatus: (node: Node) => Promise<Action>,
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
}

class FluenceNodeStatus extends React.Component<Props, State> {
    state: State = {};

    componentDidMount(): void {
        this.props.displayLoading();
        this.props.retrieveNodeStatus(this.props.node).then(this.props.hideLoading).catch(this.props.hideLoading);
    }

    renderNodeStatusInfo(nodeInfo: NodeInfo): React.ReactNode {
        let statusInfo = null;
        if (nodeInfo.status) {
            statusInfo = (
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Ip Address</strong>
                    <p className="text-muted">{nodeInfo.status.ip}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>List Of Ports:</strong>
                    <p className="text-muted">{nodeInfo.status.listOfPorts}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Uptime:</strong>
                    <p className="text-muted">{nodeInfo.status.uptime}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Workers count:</strong>
                    <p className="text-muted">{nodeInfo.status.numberOfWorkers}</p>
                </div>
            );
        } else if (nodeInfo.causeBy) {
            statusInfo = (
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Error</strong>
                    <p className="text-muted">{nodeInfo.causeBy.toString()}</p>

                </div>
            );
        }

        return (
            <div className="box-footer no-padding">
                {statusInfo}
            </div>
        );
    }

    render(): React.ReactNode {
        const nodeInfo = this.props.nodesStatus[this.props.node.id];

        return (
            <div className="col-md-4">
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-green">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon"><i className={nodeInfo ? 'ion ion-android-laptop' : 'fa fa-refresh fa-spin'}></i></span>
                        </div>
                        <h3 className="widget-user-username">Node Status</h3>
                        <h5 className="widget-user-desc">ID:&nbsp;{this.props.node.id}</h5>
                    </div>
                    { nodeInfo && this.renderNodeStatusInfo(nodeInfo) }
                </div>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    nodesStatus: state.nodesStatus,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNodeStatus,
};

export default connect(mapStateToProps ,mapDispatchToProps)(FluenceNodeStatus);
