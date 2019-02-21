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

    render(): React.ReactNode {
        const app = this.props.apps[this.props.appId];

        let appInfo = null;
        if (app) {
            appInfo = (
                <ul>
                    <li>app_id: {app.app_id}</li>
                    <li>storage_hash: {app.storage_hash}</li>
                    <li>storage_receipt: {app.storage_receipt}</li>
                    <li>cluster_size: {app.cluster_size}</li>
                    <li>owner: {app.owner}</li>
                    <li>cluster: {<FluenceCluster cluster={app.cluster}/>}</li>
                </ul>
            );
        } else {
            appInfo = (<span>Loading...</span>);
        }
        return (
            <div>
                <h3>App {this.props.appId}</h3>
                {appInfo}
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
