import * as React from 'react';
import { connect } from 'react-redux';
import {
    displayLoading,
    hideLoading,
    retrieveApp,
    deleteApp
} from '../../actions';
import { findDeployableAppByStorageHash } from "../../../fluence/deployable";
import FluenceCluster from '../fluence-cluster';
import FluenceId from '../fluence-id';
import {AppId, App} from '../../../fluence';
import { Action } from 'redux';
import {History} from "history";
import {withRouter} from "react-router";
import { ReduxState } from '../../app';

interface State {
    deleting: boolean;
}

interface Props {
    appId: AppId;
    apps: {
        [key: string]: App;
    };
    retrieveApp: (appId: AppId) => Promise<Action>;
    deleteApp: (appId: AppId, history: History) => Promise<Action>;
    displayLoading: typeof displayLoading;
    hideLoading: typeof hideLoading;
    history: History;
    userAddress: string;
}

class FluenceApp extends React.Component<Props, State> {
    state: State = {
        deleting: false,
    };

    loadData(): void {
        this.props.displayLoading();
        this.props.retrieveApp(this.props.appId).then(this.props.hideLoading).catch(this.props.hideLoading);
    }

    componentDidMount(): void {
        this.loadData();
    }

    componentDidUpdate(prevProps: Props): void {
        if (prevProps.appId !== this.props.appId) {
            this.loadData();
        }
    }

    deleteApplication = (): void => {
        if (!confirm('Are you sure you want to delete this app?')) return;

        this.props.displayLoading();
        this.setState({
            deleting: true
        });
        this.props.deleteApp(this.props.appId, this.props.history).finally(() => {
            this.props.hideLoading();
            this.setState({
                deleting: false
            });
        });
    };

    renderAppInfo(app: App): React.ReactNode {
        return (
            <div className="box-footer no-padding">
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"/>Storage Hash</strong>
                    <p className="text-muted"><FluenceId entityId={app.storage_hash}/></p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"/>Storage Receipt</strong>
                    <p className="text-muted"><FluenceId entityId={app.storage_receipt}/></p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"/>Cluster Size</strong>
                    <p className="text-muted">{app.cluster_size}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"/>Owner</strong>
                    <p className="text-muted">
                        <FluenceId
                            entityId={app.owner}
                            isLink={true}
                            href={'https://rinkeby.etherscan.io/address/' + app.owner}
                            className="etherscan-link"
                            target="_blank"/>
                    </p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster</strong>
                    {<FluenceCluster appId={this.props.appId} cluster={app.cluster}/>}

                    <hr/>
                    <button type="button"
                            onClick={this.deleteApplication}
                            style={{display: app.owner.toUpperCase() === this.props.userAddress.toUpperCase() ? 'block' : 'none'}}
                            disabled={this.state.deleting}
                            className="btn btn-block btn-danger"
                    >
                        Delete app <i style={{display: (this.state.deleting) ? 'inline-block' : 'none'}} className="fa fa-refresh fa-spin"/>
                    </button>
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const app = this.props.apps[this.props.appId];

        let appName = 'App';
        if (app) {
            const deployableApp = findDeployableAppByStorageHash(app.storage_hash);
            if (deployableApp) {
                appName += ` (${deployableApp.shortName})`;
            }
        }
        return (
            <div className="box box-widget widget-user-2">
                <div className="widget-user-header bg-fluence-blue-gradient">
                    <div className="widget-user-image">
                        <span className="entity-info-box-icon"><i className={app ? 'ion ion-ios-gear-outline' : 'fa fa-refresh fa-spin'}/></span>
                    </div>
                    <h3 className="widget-user-username">{appName}</h3>
                    <h5 className="widget-user-desc">ID:&nbsp;{this.props.appId}</h5>
                </div>
                {app && this.renderAppInfo(app)}
            </div>
        );
    }
}

const mapStateToProps = (state: ReduxState) => ({
    apps: state.apps,
    userAddress: state.ethereumConnection.userAddress,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveApp,
    deleteApp,
};

export default withRouter(connect(mapStateToProps, mapDispatchToProps)(FluenceApp));
