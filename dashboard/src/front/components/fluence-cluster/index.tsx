import * as React from 'react';
import { connect } from 'react-redux';
import {Option} from "ts-option";
import {Cluster} from "../../../fluence/apps";

interface State {}

interface Props {
    cluster: Option<Cluster>
}

class FluenceCluster extends React.Component<Props, State> {
    state: State = {};

    render(): React.ReactNode {

        let clusterInfo = null;
        if (this.props.cluster.isDefined) {
            clusterInfo= (
                <ul>
                    <li>genesis_time: {this.props.cluster.get.genesis_time}</li>
                    <li>cluster_members: {this.props.cluster.get.cluster_members.map(m => m.id + ':' + m.port).join(', \n')}</li>
                </ul>
            );
        } else {
            clusterInfo = (<span>-</span>);
        }
        return (
            clusterInfo
        );
    }
}

export default connect()(FluenceCluster);
