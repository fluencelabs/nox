import * as React from 'react';
import { connect} from 'react-redux';
import { Link } from "react-router-dom";
import FluenceId from '../fluence-id';
import { displayLoading, hideLoading, retrieveNodeRefs } from "../../actions";
import { Action } from "redux";
import {NodeRef} from "../../../fluence";
import { ReduxState } from '../../app';


interface State {
    nodeIdsLoading: boolean,
}

interface Props {
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    retrieveNodeRefs: () => Promise<Action>,
    nodeRefs: NodeRef[],
    filter: (nodeRef: NodeRef) => boolean;
}

class FluenceNodesList extends React.Component<Props, State> {
    state: State = {
        nodeIdsLoading: false,
    };

    static defaultProps = {
        filter: () => true
    };

    componentDidMount(): void {
        this.props.displayLoading();
        this.setState({
            nodeIdsLoading: true,
        });

        this.props.retrieveNodeRefs().then(() => {
            this.setState({
                nodeIdsLoading: false,
            });
            this.props.hideLoading();
        }).catch((e) => {
            window.console.log(e);
            this.setState({
                nodeIdsLoading: false,
            });
            this.props.hideLoading();
        });
    }

    render(): React.ReactNode {
        return (
            <div className="small-box bg-fluence-blue-gradient">
                <div className="inner">
                    <h3>{this.state.nodeIdsLoading ? '...' : this.props.nodeRefs.filter(this.props.filter).length}</h3>

                    <p>Nodes</p>
                </div>
                <div className="icon">
                    <i className={this.state.nodeIdsLoading ? 'fa fa-refresh fa-spin' : 'ion ion-android-laptop'}></i>
                </div>
                {this.props.nodeRefs.filter(this.props.filter).map(nodeRef => (
                    <div className="small-box-footer entity-link">
                        <Link to={`/node/${nodeRef.node_id}`}>
                            <div className="box-body">
                                <strong>
                                    <i className="fa fa-bullseye margin-r-5"></i> Node <FluenceId entityId={nodeRef.node_id}/>
                                </strong>
                            </div>
                        </Link>
                    </div>
                ))}
            </div>
        );
    }
}

const mapStateToProps = (state: ReduxState) => ({
    nodeRefs: state.nodeRefs,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNodeRefs,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceNodesList);
