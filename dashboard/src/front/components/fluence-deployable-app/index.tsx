import * as React from 'react';
import {connect} from 'react-redux';
import {App} from "../../../fluence/apps";
import {DeployableApp, DeployableAppId, deployableApps} from "../../../fluence/deployable";
import {contract, web3js} from "../../../fluence/contract";

interface State {}

interface Props {
    id: DeployableAppId,
    deployableApps: {
        [key: string]: App
    }
}

class FluenceDeployableApp extends React.Component<Props, State> {
    state: State = {};

    renderAppInfo(app: DeployableApp): React.ReactNode {
        return (
            <div className="box-footer no-padding">
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Storage Hash</strong>
                    <p className="text-muted" title={app.storage_hash}>{app.storage_hash}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster Size</strong>
                    <p className="text-muted">{app.cluster_size}</p>
                    <hr/>

                    <p>
                        <button
                            type="button"
                            onClick={e => deployApp(e, app)}
                            className="btn btn-block btn-primary">
                            Deploy app <i style={{ display: 'inline-block'}} className="fa fa-refresh fa-spin"></i>
                        </button>
                    </p>
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const app = this.props.deployableApps[this.props.id];

        return (
            <div className="col-md-4 col-xs-12">
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-fluence-blue-gradient">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon">
                                <i className={app ? 'ion ion-ios-gear-outline' : 'fa fa-refresh fa-spin'}></i>
                            </span>
                        </div>
                        <h3 className="widget-user-username">App</h3>
                        <h5 className="widget-user-desc">Name:&nbsp;{this.props.id}</h5>
                    </div>
                    { app && this.renderAppInfo(app) }
                </div>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    deployableApps: deployableApps,
});

const mapDispatchToProps = { };

export default connect(mapStateToProps, mapDispatchToProps)(FluenceDeployableApp);
