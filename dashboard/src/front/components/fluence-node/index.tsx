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

    render(): React.ReactNode {
        const node = this.props.nodes[this.props.nodeId];
        let nodeInfo = null;
        if (node) {
            nodeInfo = (
                <ul>
                    <li>id: {node.id}</li>
                    <li>tendermint_key: {node.tendermint_key}</li>
                    <li>ip_addr: {node.ip_addr}</li>
                    <li>next_port: {node.next_port}</li>
                    <li>last_port: {node.last_port}</li>
                    <li>owner: {node.owner}</li>
                    <li>is_private: {node.is_private.toString()}</li>
                    <li>clusters_ids: {node.clusters_ids.join(', ')}</li>
                </ul>
            );
        } else {
            nodeInfo = (<span>Loading...</span>);
        }
        return (
            <div>
                <h3>Node {this.props.nodeId}</h3>
                {nodeInfo}
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
