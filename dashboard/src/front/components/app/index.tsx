import * as React from 'react';
import { connect } from 'react-redux';
import { Link } from 'react-router-dom';
import { match, withRouter } from 'react-router';
import {
    contractAddress,
    getDashboardContract,
    subscribeToMetamaskEvents,
    EthConnectionState,
    MetamaskEvent,
} from '../../../fluence/contract';
import { updateEthereumConnectionState, showModal } from '../../actions';
import FluenceApp from '../fluence-app';
import FluenceNode from '../fluence-node';
import FluenceDeployableApp from '../fluence-deployable-app';
import FluenceAppSnippet from '../fluence-app-snippet';
import FluenceMenu from '../fluence-menu';
import FluenceList from '../fluence-list';
import FluenceModal from '../fluence-modal';
import FluenceText from '../fluence-text';
import FluenceId from '../fluence-id';

import * as fluence from 'fluence';

import 'bootstrap/dist/css/bootstrap.css';
import 'font-awesome/css/font-awesome.css';
//import 'ionicons/dist/css/ionicons.css'; /* different version, lack of icons */
import 'admin-lte/bower_components/Ionicons/css/ionicons.min.css';
import 'admin-lte/dist/css/AdminLTE.css';
import 'admin-lte/dist/css/skins/skin-blue.css';
import './style.css';
import { ReduxState } from '../../app';

export enum FluenceEntityType {
    None = -1,
    Stub = 'stub',
    App  = 'app',
    Node = 'node',
    DeployableApp = 'deploy',
    Account = 'account',
}

interface State {}

interface UrlParams {
    entityType: string;
    entityId: string;
    entitySubType: string;
}

interface Props {
    match: match<UrlParams>;
    updateEthereumConnectionState: typeof updateEthereumConnectionState;
    showModal: typeof showModal;
    loading: boolean;
    isMetamaskActive: boolean;
}

class DashboardApp extends React.Component<Props, State> {
    state: State = {};

    componentDidMount(): void {
        // Make fluence available from browser console
        (window as any).fluence = fluence;

        subscribeToMetamaskEvents((eventType: MetamaskEvent, ethConnectionState: EthConnectionState) => {
            this.props.updateEthereumConnectionState(ethConnectionState);
        });
    }

    renderEntity(entityType: FluenceEntityType, entityId: string): React.ReactNode | React.ReactNode[] {
        if(entityType && entityId) {
            switch (entityType) {
                case FluenceEntityType.App: {
                    return [
                        <div className="col-md-6 col-xs-12">
                            <FluenceApp appId={entityId}/>
                        </div>,
                        <div className="col-md-6 col-xs-12">
                            <FluenceAppSnippet appId={entityId}/>
                        </div>,
                    ];
                }
                case FluenceEntityType.Node: {
                    return (
                        <div className="col-md-6 col-xs-12">
                            <FluenceNode nodeId={entityId}/>
                        </div>
                    );
                }
                case FluenceEntityType.DeployableApp: {
                    return (
                        <div className="col-md-6 col-xs-12">
                            <FluenceDeployableApp id={entityId}/>
                        </div>
                    );
                }
                case FluenceEntityType.Account: {
                    return this.renderEntity(this.props.match.params.entitySubType as FluenceEntityType, entityId);
                }
            }
        }

        return (
            <div className="col-md-6 col-xs-12">
                <FluenceText entityType={entityType}/>
            </div>
        );
    }

    renderSectionLabel(entityType: FluenceEntityType): string {
        switch (entityType) {
            case FluenceEntityType.App: {
                return 'Applications';
            }
            case FluenceEntityType.Node: {
                return 'Nodes';
            }
            case FluenceEntityType.DeployableApp: {
                return 'Deploy';
            }
            case FluenceEntityType.Account: {
                return 'Account';
            }
            default: {
                return 'Network status';
            }
        }
    }

    render(): React.ReactNode {
        return (
            <div className={'wrapper ' + (!this.props.match.params.entityType ? 'one-column-mode' : '')}>
                <header className="main-header">
                    <nav className="navbar navbar-static-top navbar-fluence-background">

                        <Link to="/" className="logo">
                            <span className="logo-lg">Fluence network dashboard</span>
                        </Link>

                        <div className="navbar-custom-menu">
                            <ul className="nav navbar-nav">
                                <li>
                                    {this.props.isMetamaskActive
                                        ? <span className="fluence-header-label">Using <span>Metamask account</span></span>
                                        : <span className="fluence-header-label">Working in <span onClick={() => this.props.showModal()} className="error pseudolink">DEMO MODE</span></span>
                                    }
                                </li>
                                <li>
                                    <span className="fluence-header-label">Network contract: <FluenceId
                                        href={'https://rinkeby.etherscan.io/address/' + contractAddress}
                                        isLink={true} target="_blank" entityId={contractAddress}/></span>
                                </li>
                                <li style={{visibility: this.props.loading ? 'visible' : 'hidden'}}>
                                    <a href="#"><i className="fa fa-refresh fa-spin"></i></a>
                                </li>
                            </ul>
                        </div>
                    </nav>
                </header>

                <div className="content-middle">
                    <aside className="main-sidebar">
                        <FluenceMenu entityType={this.props.match.params.entityType} entityId={this.props.match.params.entityId}/>
                        <FluenceList entityType={this.props.match.params.entityType} entityId={this.props.match.params.entityId}/>
                    </aside>

                    <div className="content-wrapper">
                        <section className="content-header">
                            <h1>{this.renderSectionLabel(this.props.match.params.entityType as FluenceEntityType)}</h1>
                        </section>

                        <section className="content">
                            <div className="row">
                                {this.renderEntity(this.props.match.params.entityType as FluenceEntityType, this.props.match.params.entityId)}
                            </div>
                        </section>
                    </div>
                </div>

                <footer className="main-footer">
                    <strong><a href="http://fluence.network/">Fluence Labs&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://discordapp.com/invite/AjfbDKQ">Discord&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://github.com/fluencelabs/fluence">GitHub&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://github.com/fluencelabs/tutorials">Tutorials&nbsp;&nbsp;</a>|&nbsp;&nbsp;</strong>
                    <strong><a href="https://fluence.dev/">Documentation</a></strong>
                </footer>

                <FluenceModal />
            </div>
        );
    }
}

const mapStateToProps = (state: ReduxState) => ({
    loading: state.loading.isLoading,
    isMetamaskActive: state.ethereumConnection.isMetamaskProviderActive,
});

const mapDispatchToProps = {
    updateEthereumConnectionState,
    showModal,
};

export default withRouter(connect(mapStateToProps, mapDispatchToProps)(DashboardApp));
