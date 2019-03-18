import * as React from 'react';
import { connect } from 'react-redux';
import { displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
} from '../../actions';
import { contractAddress } from '../../../fluence/contract';
import {App, Node, NodeId, AppId} from '../../../fluence';
import { cutId } from '../../../utils';
import FluenceApp from '../fluence-app';
import FluenceNode from '../fluence-node';
import FluenceDeployableApp from '../fluence-deployable-app';
import {Action} from "redux";

import 'bootstrap/dist/css/bootstrap.css';
import 'font-awesome/css/font-awesome.css';
//import 'ionicons/dist/css/ionicons.css'; /* different version, lack of icons */
import 'admin-lte/bower_components/Ionicons/css/ionicons.min.css';
import 'admin-lte/dist/css/AdminLTE.css';
import 'admin-lte/dist/css/skins/skin-blue.css';
import './style.css';
import {DeployableApp, DeployableAppId, deployableAppIds, deployableApps} from "../../../fluence/deployable";

export interface FluenceEntity {
    id: NodeId|AppId|DeployableAppId
    type: string
}

interface State {
    appIdsLoading: boolean,
    nodeIdsLoading: boolean,
    appIdsVisible: boolean,
    nodeIdsVisible: boolean,
    deployableAppIdsVisible: boolean,
    currentEntity: FluenceEntity | null
}

interface Props {
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    retrieveNodeIds: () => Promise<Action>,
    retrieveAppIds: () => Promise<Action>,
    loading: boolean,
    nodeIds: NodeId[],
    appIds: AppId[],
    deployableAppIds: DeployableAppId[],
    apps: {
        [key: string]: App
    };
    nodes: {
        [key: string]: Node
    };
}

class DashboardApp extends React.Component<Props, State>{
    state: State = {
        appIdsLoading: false,
        nodeIdsLoading: false,
        appIdsVisible: false,
        nodeIdsVisible: false,
        deployableAppIdsVisible: false,
        currentEntity: null,
    };

    componentDidMount(): void {

        this.props.displayLoading();
        this.setState({
            appIdsLoading: true,
            nodeIdsLoading: true,
        });

        Promise.all([
            this.props.retrieveNodeIds().then(() => {
                this.setState({
                    nodeIdsLoading: false,
                });
            }),
            this.props.retrieveAppIds().then(() => {
                this.setState({
                    appIdsLoading: false,
                });
            }),
        ]).then(() => {
            this.props.hideLoading();
        }).catch((e) => {
            window.console.log(e);
            this.props.hideLoading();
        });
    }

    showApp = (e: React.MouseEvent<HTMLElement>, appId: AppId): void => {
        e.preventDefault();
        this.setState({
            currentEntity: {
                type: 'app',
                id: appId
            }
        });
    };

    showAppIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            appIdsVisible: true
        });
    };

    hideAppIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            appIdsVisible: false
        });
    };

    renderAppsCount(): React.ReactNode {
        return (
            <div className="col-md-12">
                <div className="small-box bg-fluence-blue-gradient">
                    <div className="inner">
                        <h3>{this.state.appIdsLoading ? '...' : this.props.appIds.length }</h3>

                        <p>Apps</p>
                    </div>
                    <div className="icon">
                        <i className={this.state.appIdsLoading ? 'fa fa-refresh fa-spin' : 'ion ion-ios-gear-outline' }></i>
                    </div>
                    <a href="#" className="small-box-footer" onClick={this.showAppIds} style={{ display: this.state.appIdsLoading || this.state.appIdsVisible || this.props.appIds.length  <= 0 ? 'none' : 'block' }}>
                        More info <i className="fa fa-arrow-circle-right"></i>
                    </a>
                    <a href="#" className="small-box-footer" onClick={this.hideAppIds} style={{ display: this.state.appIdsVisible ?  'block' : 'none' }}>
                        Hide info <i className="fa fa-arrow-circle-up"></i>
                    </a>
                    { this.props.appIds.map(appId => (
                        <div className="small-box-footer entity-link" onClick={(e) => this.showApp(e, appId)} style={{ display: this.state.appIdsVisible ? 'block' : 'none'}}>
                            <div className="box-body">
                                <strong><i className="fa fa-bullseye margin-r-5"></i> App {appId}</strong>
                            </div>
                        </div>
                    )) }
                </div>
            </div>
        );
    }

    showNode = (e: React.MouseEvent<HTMLElement>, nodeId: NodeId): void => {
        e.preventDefault();
        this.setState({
            currentEntity: {
                type: 'node',
                id: nodeId
            }
        });
    };

    showNodeIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            nodeIdsVisible: true
        });
    };

    hideNodeIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            nodeIdsVisible: false
        });
    };

    renderNodesCount(): React.ReactNode {
        return (
            <div className="col-md-12">
                <div className="small-box bg-fluence-blue-gradient">
                    <div className="inner">
                        <h3>{this.state.nodeIdsLoading ? '...' : this.props.nodeIds.length }</h3>

                        <p>Nodes</p>
                    </div>
                    <div className="icon">
                        <i className={this.state.nodeIdsLoading ? 'fa fa-refresh fa-spin' : 'ion ion-android-laptop' }></i>
                    </div>
                    <a href="#" className="small-box-footer" onClick={this.showNodeIds} style={{ display: this.state.nodeIdsLoading || this.state.nodeIdsVisible || this.props.nodeIds.length  <= 0 ? 'none' : 'block' }}>
                        More info <i className="fa fa-arrow-circle-right"></i>
                    </a>
                    <a href="#" className="small-box-footer" onClick={this.hideNodeIds} style={{ display: this.state.nodeIdsVisible ?  'block' : 'none' }}>
                        Hide info <i className="fa fa-arrow-circle-up"></i>
                    </a>
                    { this.props.nodeIds.map(nodeId => (
                        <div className="small-box-footer entity-link" onClick={(e) => this.showNode(e, nodeId)} style={{ display: this.state.nodeIdsVisible ? 'block' : 'none'}}>
                            <div className="box-body">
                                <strong>
                                    <i className="fa fa-bullseye margin-r-5"></i> Node <span title={nodeId}>{cutId(nodeId)}</span>
                                </strong>
                            </div>
                        </div>
                    )) }
                </div>
            </div>
        );
    }

    showDeployableAppIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            deployableAppIdsVisible: true
        });
    };

    hideDeployableAppIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            deployableAppIdsVisible: false
        });
    };

    showDeployableApp = (e: React.MouseEvent<HTMLElement>, id: DeployableAppId) => {
        e.preventDefault();
        this.setState({
            currentEntity: {
                type: 'deployableApp',
                id: id
            }
        })
    };

    renderEntity(entity: FluenceEntity|null): React.ReactNode {
        if(entity) {
            if (entity.type === 'app') {
                return <FluenceApp appId={entity.id} />
            } else if(entity.type === 'node') {
                return <FluenceNode nodeId={entity.id} />
            } else if (entity.type === 'deployableApp') {
                return <FluenceDeployableApp id={entity.id} />
            }
        }

        return (
            <div className="col-md-4 col-xs-12">
                <div className="box box-primary">
                    <div className="box-header with-border">
                        <h3 className="box-title">Fluence Network</h3>
                    </div>
                    <div className="box-body">
                        <p>Fluence is a decentralized computation platform, trustless and efficient. It could be used to set-up and access a database or to run a full-scale application backend.</p>
                        <p>Fluence Network is a work in progress, currently in a state of Devnet. We strongly advise against using the Devnet for anything than testing and education purposes.</p>
                        <p>If you've got any questions or need help with your setup, please, feel free to ask <a href="https://discord.gg/AjfbDKQ" target="_blank">on Discord</a>!</p>
                        <p>Fluence Network documentation can be found <a href="https://fluence.network/docs" target="_blank">here</a>.</p>
                        <p>Main Fluence <a href="https://github.com/fluencelabs/fluence" target="_blank">repository on GitHub</a></p>
                    </div>
                </div>
            </div>
        );
    }

    renderDeployBox(): React.ReactNode {
        return (
            <div className="col-md-12">
                <div className="small-box bg-fluence-blue-gradient">
                    <div className="inner">
                        <h3>{this.props.deployableAppIds.length}</h3>

                        <p>Deploy an app</p>
                    </div>
                    <div className="icon">
                        <i className='ion ion-cloud-download'></i>
                    </div>
                    <a href="#" className="small-box-footer" onClick={this.showDeployableAppIds} style={{ display: this.state.deployableAppIdsVisible || this.props.deployableAppIds.length  <= 0 ? 'none' : 'block' }}>
                        More info <i className="fa fa-arrow-circle-right"></i>
                    </a>
                    <a href="#" className="small-box-footer" onClick={this.hideDeployableAppIds} style={{ display: this.state.deployableAppIdsVisible ?  'block' : 'none' }}>
                        Hide info <i className="fa fa-arrow-circle-up"></i>
                    </a>
                    { this.props.deployableAppIds.map(id => (
                        <div className="small-box-footer entity-link" onClick={(e) => this.showDeployableApp(e, id)} style={{ display: this.state.deployableAppIdsVisible ? 'block' : 'none'}}>
                            <div className="box-body">
                                <strong>
                                    <i className="fa fa-bullseye margin-r-5"></i> Node <span title={id}>{id}</span>
                                </strong>
                            </div>
                        </div>
                    )) }
                </div>
            </div>
        )
    }

    render(): React.ReactNode {
        return (
            <div className="wrapper">
                <header className="main-header">
                    <nav className="navbar navbar-static-top navbar-fluence-background">

                        <a href="/" className="logo">
                            <span className="logo-lg">Fluence network dashboard</span>
                        </a>

                        <div className="navbar-custom-menu">
                            <ul className="nav navbar-nav">
                                <li>
                                    <span className="fluence-contract-address">Network contract: <a href={'https://rinkeby.etherscan.io/address/' + contractAddress} title={contractAddress} target="_blank">{cutId(contractAddress)}</a></span>
                                </li>
                                <li style={{ visibility: this.props.loading ? 'visible' : 'hidden' }}>
                                    <a href="#"><i className="fa fa-refresh fa-spin"></i></a>
                                </li>
                            </ul>
                        </div>
                    </nav>
                </header>

                <div className="content-wrapper">
                    <section className="content-header">
                        <h1>Network status</h1>
                    </section>

                    <section className="content">
                        <div className="row">
                            <div className="col-md-3 col-xs-12">
                                <div className="row">
                                    { this.renderAppsCount() }
                                </div>
                                <div className="row">
                                    { this.renderNodesCount() }
                                </div>
                                <div className="row">
                                    { this.renderDeployBox() }
                                </div>
                            </div>
                            { this.renderEntity(this.state.currentEntity) }
                        </div>
                    </section>
                </div>

                <footer className="main-footer">
                    <strong><a href="http://fluence.network/">Fluence Labs</a></strong>
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
    deployableAppIds: deployableAppIds,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNodeIds,
    retrieveAppIds,
};

export default connect(mapStateToProps, mapDispatchToProps)(DashboardApp);
