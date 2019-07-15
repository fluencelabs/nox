import * as React from "react";
import {connect} from "react-redux";
import {Link} from "react-router-dom";
import FluenceId from '../fluence-id';
import {FluenceEntityType} from "../app";
import {
    DeployableAppId,
    deployableAppIds,
    deployableApps,
    findDeployableAppByStorageHash
} from "../../../fluence/deployable";
import {displayLoading, hideLoading, retrieveNodeRefs, retrieveAppRefs, showModal} from "../../actions";
import {AppRef, NodeRef} from "../../../fluence";
import {Action} from "redux";
import { ReduxState } from '../../app';

interface State {
    loading: boolean;
}

interface Props {
    entityType: FluenceEntityType,
    entityId: string,
    displayLoading: typeof displayLoading;
    hideLoading: typeof hideLoading;
    retrieveAppRefs: () => Promise<Action>;
    appIdsRetrievedCallback: (appRefs: AppRef[]) => void;
    retrieveNodeRefs: () => Promise<Action>,
    appRefs: AppRef[];
    nodeRefs: NodeRef[];
    userAddress: string;
    isMetamaskActive: boolean;
    showModal: typeof showModal;
}

interface RenderOptions {
    filter?: (ref: AppRef | NodeRef) => boolean;
    urlPrefix?: string;
}

class FluenceList extends React.Component<Props, State> {
    state: State = {
        loading: false,
    };

    componentDidMount(): void {
        this.showModal();

        this.props.displayLoading();
        this.setState({
            loading: true,
        });

        Promise.all([
            this.props.retrieveNodeRefs(),
            this.props.retrieveAppRefs(),
        ]).then(() => {
            this.setState({
                loading: false,
            });
            this.props.hideLoading();
        }).catch((err: any) => {
            window.console.log(err);
            this.setState({
                loading: false,
            });
            this.props.hideLoading();
        });
    }

    componentDidUpdate(): void {
        this.showModal();
    }

    showModal(): void {
        if (this.props.entityType == FluenceEntityType.Account && !this.props.isMetamaskActive) {
            this.props.showModal({ once: true });
        }
    }

    renderDeployItems(): React.ReactNode[] {
        return [
            <li className="header">Deploy</li>,
            ...deployableAppIds.map((id: DeployableAppId) => (
                <li className={this.props.entityId == id ? 'active' : ''}>
                    <Link to={`/deploy/${id}`}>
                        <i className="fa fa-arrow-circle-up"></i> {deployableApps[id].name}
                    </Link>
                </li>
            ))
        ];
    }

    getAppLabel(appRef: AppRef): string {
        const deployableApp = findDeployableAppByStorageHash(appRef.storage_hash);
        return '#' + appRef.app_id + ' \u2013 ' + ( (deployableApp && deployableApp.shortName) || 'App');
    }

    renderApplicationItems(options?: RenderOptions): React.ReactNode[] {
        options = options || {};
        const filter = options.filter || (() => true);
        const urlPrefix = options.urlPrefix || '';
        const appRefs =  this.props.appRefs.filter(filter).sort((refA, refB) => parseInt(refA.app_id) - parseInt(refB.app_id));
        return [
            <li className="header">Applications</li>,
            ...appRefs.map(appRef => (
                <li className={this.props.entityId == appRef.app_id ? 'active' : ''}>
                    <Link to={`${urlPrefix}/app/${appRef.app_id}`}>
                        <i className="ion ion-ios-gear-outline"></i> {this.getAppLabel(appRef)}
                    </Link>
                </li>
            )),
            appRefs.length <= 0 ? <li className="list-filler"><i className="ion ion-ios-gear-outline"/></li> : null,
        ];
    }

    renderNodesItems(options?: RenderOptions): React.ReactNode[] {
        options = options || {};
        const filter = options.filter || (() => true);
        const urlPrefix = options.urlPrefix || '';
        const nodeRefs = this.props.nodeRefs.filter(filter);
        return [
            <li className="header">Nodes</li>,
            ...nodeRefs.map(nodeRef => (
                <li className={this.props.entityId == nodeRef.node_id ? 'active' : ''}>
                    <Link to={`${urlPrefix}/node/${nodeRef.node_id}`}>
                        <i className="ion ion-android-laptop"></i>
                        Node <FluenceId entityId={nodeRef.node_id}/>
                        <span className="node-details">Capacity: {nodeRef.capacity}</span>
                        {nodeRef.is_private && <span className="node-details">private</span>}
                    </Link>
                </li>
            )),
            nodeRefs.length <= 0 ? <li className="list-filler"><i className="ion ion-android-laptop"/></li> : null,
        ];
    }

    entityTypeMap: { [key: string]: (() => React.ReactNode[])[] } = {
        [FluenceEntityType.DeployableApp]: [
            () => this.renderDeployItems()
        ],
        [FluenceEntityType.App]: [
            () => this.renderApplicationItems()
        ],
        [FluenceEntityType.Node]: [
            () => this.renderNodesItems()
        ],
        [FluenceEntityType.Account]: [
            () => this.renderApplicationItems({
                urlPrefix: '/account',
                filter: ref => ref.owner.toUpperCase() === this.props.userAddress.toUpperCase()
            }),
            () => this.renderNodesItems({
                urlPrefix: '/account',
                filter: ref => ref.owner.toUpperCase() === this.props.userAddress.toUpperCase()
            })
        ],
    };

    render(): React.ReactNode {
        return (
            <section className="sidebar fluence-list">
                <ul className="sidebar-menu" data-widget="tree">
                    {this.props.entityType && this.entityTypeMap[this.props.entityType].map(items => items())}
                </ul>
            </section>
        );
    }
}

const mapStateToProps = (state: ReduxState) => ({
    nodeRefs: state.nodeRefs,
    appRefs: state.appRefs,
    userAddress: state.ethereumConnection.userAddress,
    isMetamaskActive: state.ethereumConnection.isMetamaskProviderActive,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveAppRefs,
    retrieveNodeRefs,
    showModal,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceList);
