import * as React from 'react';
import {connect} from 'react-redux';
import { push as pushHistory } from 'connected-react-router';
import {match, withRouter} from "react-router";
import {Action} from "redux";
import {contractAddress} from '../../../fluence/contract';
import {App, AppId, Node} from '../../../fluence';
import {cutId} from '../../../utils';
import {clearDeployedApp, getDeployedApp} from "../../../utils/cookie";
import FluenceApp from '../fluence-app';
import FluenceNode from '../fluence-node';
import FluenceDeployableApp from '../fluence-deployable-app';
import FluenceAppsList from '../fluence-apps-list'
import FluenceNodesList from '../fluence-nodes-list'
import FluenceDeployList from '../fluence-deploy-list'
import {restoreDeployed} from '../../actions';
import * as fluence from "fluence";

import 'bootstrap/dist/css/bootstrap.css';
import 'font-awesome/css/font-awesome.css';
//import 'ionicons/dist/css/ionicons.css'; /* different version, lack of icons */
import 'admin-lte/bower_components/Ionicons/css/ionicons.min.css';
import 'admin-lte/dist/css/AdminLTE.css';
import 'admin-lte/dist/css/skins/skin-blue.css';
import './style.css';

export enum FluenceEntityType {
    None = -1,
    Stub = 'stub',
    App  = 'app',
    Node = 'node',
    DeployableApp = 'deploy',
}

interface State {}

interface UrlParams {
    entityType: string,
    entityId: string,
    appId: string,
}

interface Props {
    match: match<UrlParams>,
    loading: boolean,
    restoreDeployed: (appId: string, appTypeId: string) => Action,
    apps: {
        [key: string]: App
    };
    nodes: {
        [key: string]: Node
    };
}

class DashboardApp extends React.Component<Props, State> {
    state: State = {};

    componentDidMount(): void {
        // Make fluence available from browser console
        (window as any).fluence = fluence;

        const deployedApp = getDeployedApp();
        if (deployedApp && this.props.match.path == '/') {
            this.props.restoreDeployed(deployedApp.deployedAppId, deployedApp.deployedAppTypeId);
        }
    }

    checkAppExists = (appIds: AppId[]): void => {
        const deployedApp = getDeployedApp();
        if (deployedApp && appIds.indexOf(deployedApp.deployedAppId) === -1) {
            if (
                this.props.match.params.entityType == FluenceEntityType.DeployableApp
                && this.props.match.params.entityId == deployedApp.deployedAppTypeId
            ) {
                pushHistory(`/`);
            }
            clearDeployedApp();
        }
    };

    renderEntity(): React.ReactNode {
        let entityType = this.props.match.params.entityType;
        let entityId = this.props.match.params.entityId;
        if (entityType && entityId) {
            if (entityType === FluenceEntityType.App) {
                return <FluenceApp appId={entityId}/>
            } else if (entityType === FluenceEntityType.Node) {
                return <FluenceNode nodeId={entityId}/>
            } else if (entityType === FluenceEntityType.DeployableApp) {
                return <FluenceDeployableApp id={entityId}/>
            }
        }

        return (
            <div className="col-md-4 col-xs-12">
                <div className="box box-primary">
                    <div className="box-header with-border">
                        <h3 className="box-title">Fluence Network</h3>
                    </div>
                    <div className="box-body">
                        <p>Fluence is a permissionless decentralized database platform, trustless and efficient.
                            With Fluence, you will be able to deploy an SQL/NoSQL database with just a few clicks!</p>

                        <p>Fluence Network is a work in progress and is currently in the devnet state. Feel free to play
                            with it and build demo DApps on top of your deployed database, but keep in mind that the API
                            is not stabilized yet and might change in the future.</p>

                        <p>If you have any questions or need help with your setup, please reach out to us at <a
                            href="https://discord.gg/AjfbDKQ">Discord</a>!
                            You can also take a look at the Fluence documentation.</p>
                    </div>
                </div>
            </div>
        );
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
                                    <span className="fluence-contract-address">Network contract: <a
                                        href={'https://rinkeby.etherscan.io/address/' + contractAddress}
                                        title={contractAddress} target="_blank">{cutId(contractAddress)}</a></span>
                                </li>
                                <li style={{visibility: this.props.loading ? 'visible' : 'hidden'}}>
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
                                    <div className="col-md-12">
                                        <FluenceDeployList/>
                                    </div>
                                </div>
                                <div className="row">
                                    <div className="col-md-12">
                                        <FluenceAppsList appIdsRetrievedCallback={this.checkAppExists}/>
                                    </div>
                                </div>
                                <div className="row">
                                    <div className="col-md-12">
                                        <FluenceNodesList/>
                                    </div>
                                </div>
                            </div>
                            {this.renderEntity()}
                        </div>
                    </section>
                </div>

                <footer className="main-footer">
                    <strong><a href="http://fluence.network/">Fluence Labs&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://discordapp.com/invite/AjfbDKQ">Discord&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://github.com/fluencelabs/fluence">GitHub&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://github.com/fluencelabs/tutorials">Tutorials&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://fluence.network/docs">Documentation</a></strong>
                </footer>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    loading: state.loading.isLoading,
    nodes: state.nodes,
    apps: state.apps,
});

const mapDispatchToProps = {
    restoreDeployed,
};

export default withRouter(connect(mapStateToProps, mapDispatchToProps)(DashboardApp));
