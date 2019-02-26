import * as React from 'react';
import { connect } from 'react-redux';
import {
    displayLoading,
    hideLoading,
    retrieveApp,
} from '../../actions';
import FluenceCluster from '../fluence-cluster';
import {AppId, App} from "../../../fluence";
import {Action} from "redux";

interface State {}

interface Props {
    appId: AppId,
    apps: {
        [key: string]: App
    },
    retrieveApp: (appId: AppId) => Promise<Action>,
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
}

class FluenceApp extends React.Component<Props, State> {
    state: State = {};

    componentDidMount(): void {
        this.props.displayLoading();
        this.props.retrieveApp(this.props.appId).then(this.props.hideLoading).catch(this.props.hideLoading);
    }

    renderAppInfo(app: App): React.ReactNode {
        return (
            <div className="box-footer no-padding">
                <div className="box-body">
                    <strong><i className="fa fa-bullseye margin-r-5"></i>Storage Hash</strong>
                    <p className="text-muted">{app.storage_hash}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Storage Receipt</strong>
                    <p className="text-muted">{app.storage_receipt}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster Size</strong>
                    <p className="text-muted">{app.cluster_size}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Owner</strong>
                    <p className="text-muted">{app.owner}</p>
                    <hr/>

                    <strong><i className="fa fa-bullseye margin-r-5"></i>Cluster</strong>
                    <p className="text-muted">{<FluenceCluster cluster={app.cluster}/>}</p>
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const app = this.props.apps[this.props.appId];

        return (
            <div className="col-md-4">
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-yellow">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon"><i className={app ? 'ion ion-ios-gear-outline' : 'fa fa-refresh fa-spin'}></i></span>
                        </div>
                        <h3 className="widget-user-username">App</h3>
                        <h5 className="widget-user-desc">ID:&nbsp;{this.props.appId}</h5>
                    </div>
                    { app && this.renderAppInfo(app) }
                </div>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    apps: state.apps,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveApp,
};

export default connect(mapStateToProps ,mapDispatchToProps)(FluenceApp);
