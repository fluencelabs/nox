import * as React from 'react';
import {connect} from 'react-redux';
import {DeployableAppId, deployableAppIds, deployableApps} from '../../../fluence/deployable';
import {showEntity} from '../../actions';
import {FluenceEntity, FluenceEntityType} from "../../actions/entity";
import {Action} from "redux";

interface State {
    deployableAppIdsVisible: boolean,
}

interface Props {
    showEntity: (entity: FluenceEntity) => Action,
}

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

    showDeployableApp = (e: React.MouseEvent<HTMLElement>, id: DeployableAppId) => {
        e.preventDefault();
        this.props.showEntity({
            type: FluenceEntityType.DeployableApp,
            id: id,
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
                {deployableAppIds.map(id => (
                    <div className="small-box-footer entity-link bg-fluence-green-gradient"
                         onClick={(e) => this.showDeployableApp(e, id)}>
                        <div className="box-body">
                            <strong>
                                <i className="fa fa-bullseye margin-r-5"/> <span
                                title={deployableApps[id].name}>{deployableApps[id].name}</span>
                            </strong>
                        </div>
                    </div>
                ))}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
});

const mapDispatchToProps = {
    showEntity
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceDeployList);
