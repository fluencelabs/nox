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

    render(): React.ReactNode {
        const nodeInfo = this.props.nodesStatus[this.props.node.id];
        let nodeStatus = null;
        if (nodeInfo && nodeInfo.status) {
            nodeStatus = (
                <ul>
                    <li>id: {this.props.node.id}</li>
                    <li>ip: {nodeInfo.status.ip}</li>
                    <li>listOfPorts: {nodeInfo.status.listOfPorts}</li>
                    <li>uptime: {nodeInfo.status.uptime}</li>
                    <li>numberOfWorkers: {nodeInfo.status.numberOfWorkers}</li>
                </ul>
            );
        } else if (nodeInfo && nodeInfo.causeBy) {
            nodeStatus = (<span>Error: {nodeInfo.causeBy}</span>);
        } else {
            nodeStatus = (<span>Loading...</span>);
        }
        return (
            <div>
                <h3>Node status {this.props.node.id}</h3>
                {nodeStatus}
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
