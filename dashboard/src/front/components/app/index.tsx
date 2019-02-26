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

import 'bootstrap/dist/css/bootstrap.css';
import 'font-awesome/css/font-awesome.css';
//import 'ionicons/dist/css/ionicons.css'; /* different version, lack of icons */
import 'admin-lte/bower_components/Ionicons/css/ionicons.min.css';
import 'admin-lte/dist/css/AdminLTE.css';
import 'admin-lte/dist/css/skins/skin-blue.css';
import './style.css';

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

    renderApps(): React.ReactNode {
        const nodes = this.props.appIds.map(appId => <FluenceApp appId={appId} />);
        return (
            <div>
                <h3 className="page-header">Apps</h3>
                {this.perRow(nodes)}
            </div>
        );
    }

    renderNodes(): React.ReactNode {
        const nodes = this.props.nodeIds.map(nodeId => <FluenceNode nodeId={nodeId} />);
        return (
            <div>
                <h3 className="page-header">Nodes</h3>
                {this.perRow(nodes)}
            </div>
        );
    }

    renderNodesStatus(): React.ReactNode {
        const nodes = this.props.nodeIds.map(nodeId => this.props.nodes[nodeId] && <FluenceNodeStatus node={this.props.nodes[nodeId]} />);
        return (
            <div>
                <h3 className="page-header">Nodes Status</h3>
                {this.perRow(nodes)}
            </div>
        );
    }

    perRow(items: React.ReactNode[]) {
        const perRow = 3;
        const groups = items.map((item, index) => {
            return index % perRow === 0 ? items.slice(index, index + perRow) : null;
        }).filter(item => item);

        return groups.map(
            (groupedItems: React.ReactNode[]) => (
                <div className="row">
                    {groupedItems}
                </div>
            )
        );
    }

    render(): React.ReactNode {
        return (
            <div className="wrapper">
                <header className="main-header">
                    <nav className="navbar navbar-static-top">

                        <div className="navbar-custom-menu">
                            <ul className="nav navbar-nav">
                                <li style={{ display: this.props.loading ? 'block' : 'none' }}>
                                    <a href="#"><i className="fa fa-refresh fa-spin"></i></a>
                                </li>
                            </ul>
                        </div>
                    </nav>
                </header>

                <div className="content-wrapper">
                    <section className="content-header">
                        <h1>
                            Fluence network status
                            <small>contract: {contractAddress}</small>
                        </h1>
                    </section>

                    <section className="content">
                        { this.renderApps() }
                        { this.renderNodes() }
                        { this.renderNodesStatus() }
                    </section>
                </div>

                <footer className="main-footer">
                    <strong><a href="https://fluence.one">Fluence Labs</a></strong>
                </footer>
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
