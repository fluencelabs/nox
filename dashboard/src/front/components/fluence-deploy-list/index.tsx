import * as React from 'react';
import {connect} from 'react-redux';
import {Link} from "react-router-dom";
import {DeployableAppId, deployableAppIds, deployableApps} from '../../../fluence/deployable';

interface State {
    deployableAppIdsVisible: boolean,
}

interface Props {}

class FluenceDeployList extends React.Component<Props, State> {
    state: State = {
        deployableAppIdsVisible: false,
    };

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

    render(): React.ReactNode {
        return (
            <div className="small-box bg-fluence-blue-gradient">
                <div className="inner">
                    <h3>{deployableAppIds.length}</h3>

                    <p>Instant deploy</p>
                </div>
                <div className="icon">
                    <i className='ion ion-cloud-download'/>
                </div>
                {deployableAppIds.map((id: DeployableAppId) => (
                    <div className="small-box-footer entity-link bg-fluence-green-gradient">
                        <Link to={`/deploy/${id}`}>
                            <div className="box-body">
                                <strong>
                                    <i className="fa fa-bullseye margin-r-5"/> <span
                                    title={deployableApps[id].name}>{deployableApps[id].name}</span>
                                </strong>
                            </div>
                        </Link>
                    </div>
                ))}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({});

const mapDispatchToProps = {};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceDeployList);
