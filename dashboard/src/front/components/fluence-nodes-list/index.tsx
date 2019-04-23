import * as React from 'react';
import {connect} from 'react-redux';
import {Link} from "react-router-dom";
import {cutId} from "../../../utils";
import {displayLoading, hideLoading, retrieveNodeIds} from "../../actions";
import {Action} from "redux";
import {NodeId} from "../../../fluence";


interface State {
    nodeIdsLoading: boolean,
    nodeIdsVisible: boolean,
}

interface Props {
    displayLoading: typeof displayLoading,
    hideLoading: typeof hideLoading,
    retrieveNodeIds: () => Promise<Action>,
    nodeIds: NodeId[],
}

class FluenceNodesList extends React.Component<Props, State> {
    state: State = {
        nodeIdsLoading: false,
        nodeIdsVisible: false,
    };

    componentDidMount(): void {
        this.props.displayLoading();
        this.setState({
            nodeIdsLoading: true,
        });

        this.props.retrieveNodeIds().then(() => {
            this.setState({
                nodeIdsLoading: false,
            });
        }).then(() => {
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

    showNodeIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            nodeIdsVisible: true
        });
    };

    hideNodeIds = (e: React.MouseEvent<HTMLElement>): void => {
        e.preventDefault();
        this.setState({
            nodeIdsVisible: false
        });
    };

    render(): React.ReactNode {
        return (
            <div className="small-box bg-fluence-blue-gradient">
                <div className="inner">
                    <h3>{this.state.nodeIdsLoading ? '...' : this.props.nodeIds.length}</h3>

                    <p>Nodes</p>
                </div>
                <div className="icon">
                    <i className={this.state.nodeIdsLoading ? 'fa fa-refresh fa-spin' : 'ion ion-android-laptop'}></i>
                </div>
                <a href="#" className="small-box-footer" onClick={this.showNodeIds}
                   style={{display: this.state.nodeIdsLoading || this.state.nodeIdsVisible || this.props.nodeIds.length <= 0 ? 'none' : 'block'}}>
                    More info <i className="fa fa-arrow-circle-right"></i>
                </a>
                <a href="#" className="small-box-footer" onClick={this.hideNodeIds}
                   style={{display: this.state.nodeIdsVisible ? 'block' : 'none'}}>
                    Hide info <i className="fa fa-arrow-circle-up"></i>
                </a>
                {this.props.nodeIds.map(nodeId => (
                    <div className="small-box-footer entity-link" style={{display: this.state.nodeIdsVisible ? 'block' : 'none'}}>
                        <Link to={`/node/${nodeId}`}>
                            <div className="box-body">
                                <strong>
                                    <i className="fa fa-bullseye margin-r-5"></i> Node <span
                                    title={nodeId}>{cutId(nodeId)}</span>
                                </strong>
                            </div>
                        </Link>
                    </div>
                ))}
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    nodeIds: state.nodeIds,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNodeIds,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceNodesList);
