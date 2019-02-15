import * as React from 'react';
import { connect } from 'react-redux';
import { displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
    retrieveNodes,
    retrieveApps,
} from '../../actions';
import {App, Node, NodeId, AppId} from '../../../fluence';
import {Action} from "redux";

interface State {}

interface Props {
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    retrieveNodeIds: () => Promise<Action>,
    retrieveAppIds: () => Promise<Action>,
    retrieveNodes: (nodeIds: NodeId[]) => Promise<Action>,
    retrieveApps: (appsIds: AppId[]) => Promise<Action>,
    loading: boolean,
    nodeIds: NodeId[],
    appIds: AppId[],
    apps: App[];
    nodes: Node[];
}

class DashboardApp extends React.Component<Props, State>{
    state: State = {};

    componentDidMount(): void {

        /*
         * Warning! This is example code and it should be rewritten!
         */

        this.props.displayLoading();

        Promise.all([
            this.props.retrieveNodeIds(),
            this.props.retrieveAppIds(),
        ]).then(() => {
            return Promise.all([
                this.props.retrieveNodes(this.props.nodeIds),
                this.props.retrieveApps(this.props.nodeIds),
            ]);

        }).then(() => {
            this.props.hideLoading();
        }).catch(() => {
            this.props.hideLoading();
        });
    }

    render(): React.ReactNode {
        /*
         * Warning! This is example code and it should be rewritten!
         */
        return (
            <div>
                <img style={{ display: this.props.loading ? 'block' : 'none' }} src="/static/assets/loader.gif"/>
                <p>Apps ID's</p>
                <ul>
                    {this.props.appIds.map(item => (
                        <li>{item}</li>
                    ))}
                </ul>
                <p>Nodes ID's</p>
                <ul>
                    {this.props.nodeIds.map(item => (
                        <li>{item}</li>
                    ))}
                </ul>

                <p>Apps</p>
                <ul>
                    {this.props.apps.map(item => (
                        <li>
                            ID: {item.app_id}<br/>
                            CluserSize: {item.cluster_size}<br/>
                            Owner: {item.owner}
                        </li>
                    ))}
                </ul>

                <p>Nodes</p>
                <ul>
                    {this.props.nodes.map(item => (
                        <li>
                            ID: {item.id}<br/>
                            lastport: {item.last_port}<br/>
                            nextport: {item.next_port}<br/>
                            Owner: {item.owner}
                        </li>
                    ))}
                </ul>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    loading: state.loading,
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
    retrieveNodes,
    retrieveApps,
};

export default connect(mapStateToProps, mapDispatchToProps)(DashboardApp);
