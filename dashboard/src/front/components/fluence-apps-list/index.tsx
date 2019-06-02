import * as React from 'react';
import { connect } from 'react-redux';
import { Link } from 'react-router-dom';
import { displayLoading, hideLoading, retrieveAppRefs } from '../../actions';
import { Action } from 'redux';
import {AppRef} from '../../../fluence';
import {findDeployableAppByStorageHash} from "../../../fluence/deployable";

interface State {
    appIdsLoading: boolean;
}

interface Props {
    displayLoading: typeof displayLoading;
    hideLoading: typeof hideLoading;
    retrieveAppRefs: () => Promise<Action>;
    appIdsRetrievedCallback: (appRefs: AppRef[]) => void;
    appRefs: AppRef[];
    filter: (appRef: AppRef) => boolean;
}

class FluenceAppsList extends React.Component<Props, State> {
    state: State = {
        appIdsLoading: false,
    };

    static defaultProps = {
        filter: () => true
    };

    componentDidMount(): void {
        this.props.displayLoading();
        this.setState({
            appIdsLoading: true,
        });

        this.props.retrieveAppRefs().then(() => {
            this.setState({
                appIdsLoading: false,
            });
            this.props.hideLoading();

            if (this.props.appIdsRetrievedCallback) {
                this.props.appIdsRetrievedCallback(this.props.appRefs);
            }
        }).catch(e => {
            window.console.log(e);
            this.setState({
                appIdsLoading: false,
            });
            this.props.hideLoading();
        });
    }

    getAppLabel(appRef: AppRef): string {
        const deployableApp = findDeployableAppByStorageHash(appRef.storage_hash);
        return ( (deployableApp && deployableApp.shortName) || 'App') + '#' + appRef.app_id;
    }

    render(): React.ReactNode {
        return (
            <div className="small-box bg-fluence-blue-gradient">
                <div className="inner">
                    <h3>{this.state.appIdsLoading ? '...' : this.props.appRefs.filter(this.props.filter).length}</h3>

                    <p>Apps</p>
                </div>
                <div className="icon">
                    <i className={this.state.appIdsLoading ? 'fa fa-refresh fa-spin' : 'ion ion-ios-gear-outline'}></i>
                </div>
                {this.props.appRefs.filter(this.props.filter).map(appRef => (
                    <div className="small-box-footer entity-link">
                        <Link to={`/app/${appRef.app_id}`}>
                            <div className="box-body">
                                <strong><i className="fa fa-bullseye margin-r-5"></i>{this.getAppLabel(appRef)}</strong>
                            </div>
                        </Link>
                    </div>
                ))}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    appRefs: state.appRefs,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveAppRefs,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceAppsList);
