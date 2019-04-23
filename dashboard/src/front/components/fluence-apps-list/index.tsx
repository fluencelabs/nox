import * as React from 'react';
import {connect} from 'react-redux';
import {Link} from "react-router-dom";
import {displayLoading, hideLoading, retrieveAppIds} from "../../actions";
import {Action} from "redux";
import {AppId} from "../../../fluence";

interface State {
    appIdsLoading: boolean,
    appIdsVisible: boolean,
}

interface Props {
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    retrieveAppIds: () => Promise<Action>,
    appIdsRetrievedCallback: (appIds: AppId[]) => void,
    appIds: AppId[],
}

class FluenceAppsList extends React.Component<Props, State> {
    state: State = {
        appIdsLoading: false,
        appIdsVisible: false,
    };

    componentDidMount(): void {
        this.props.displayLoading();
        this.setState({
            appIdsLoading: true,
        });

        this.props.retrieveAppIds().then(() => {
            this.setState({
                appIdsLoading: false,
            });
            this.props.hideLoading();

            if (this.props.appIdsRetrievedCallback) {
                this.props.appIdsRetrievedCallback(this.props.appIds);
            }
        }).catch((e) => {
            window.console.log(e);
            this.setState({
                appIdsLoading: false,
            });
            this.props.hideLoading();
        });
    }

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

    render(): React.ReactNode {
        return (
            <div className="small-box bg-fluence-blue-gradient">
                <div className="inner">
                    <h3>{this.state.appIdsLoading ? '...' : this.props.appIds.length}</h3>

                    <p>Apps</p>
                </div>
                <div className="icon">
                    <i className={this.state.appIdsLoading ? 'fa fa-refresh fa-spin' : 'ion ion-ios-gear-outline'}></i>
                </div>
                <a href="#" className="small-box-footer" onClick={this.showAppIds}
                   style={{display: this.state.appIdsLoading || this.state.appIdsVisible || this.props.appIds.length <= 0 ? 'none' : 'block'}}>
                    More info <i className="fa fa-arrow-circle-right"></i>
                </a>
                <a href="#" className="small-box-footer" onClick={this.hideAppIds}
                   style={{display: this.state.appIdsVisible ? 'block' : 'none'}}>
                    Hide info <i className="fa fa-arrow-circle-up"></i>
                </a>
                {this.props.appIds.map(appId => (
                    <div className="small-box-footer entity-link" style={{display: this.state.appIdsVisible ? 'block' : 'none'}}>
                        <Link to={`/app/${appId}`}>
                            <div className="box-body">
                                <strong><i className="fa fa-bullseye margin-r-5"></i> App {appId}</strong>
                            </div>
                        </Link>
                    </div>
                ))}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    appIds: state.appIds,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveAppIds,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceAppsList);
