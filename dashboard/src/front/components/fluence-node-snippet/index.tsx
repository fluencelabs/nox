import * as React from 'react';
import { connect } from 'react-redux';
import { Action } from 'redux';
import { KademliaRpc, KademliaRpcHttpTransport } from 'fluence-kademlia';
import { displayLoading, hideLoading, retrieveNode, } from '../../actions';
import FluenceId from '../fluence-id';
import FluenceKademliaContact from '../fluence-kademlia-contact';
import { ReduxState } from '../../app';
import { NodeId, Node } from '../../../fluence';

interface State {
    kademliaPingResponse: string;
    kademliaLookupResponse: string[];
}

interface Props {
    nodeId: NodeId;
    nodes: {
        [key: string]: Node;
    };
    retrieveNode: (nodeId: NodeId) => Promise<Action>;
    displayLoading: typeof displayLoading;
    hideLoading: typeof hideLoading;
}

class FluenceNodeSnippet extends React.Component<Props, State> {
    state: State = {
        kademliaPingResponse: '',
        kademliaLookupResponse: [],
    };

    kademliaLookupKeyField: HTMLTextAreaElement;

    loadData(): void {
        this.props.displayLoading();
        this.props.retrieveNode(this.props.nodeId)
            .then(this.props.hideLoading)
            .catch(this.props.hideLoading);

    }

    componentDidMount(): void {
        this.loadData();
    }

    componentDidUpdate(prevProps: Props): void {
        if (prevProps.nodeId !== this.props.nodeId) {
            this.loadData();

            this.kademliaLookupKeyField.value = '';
            this.setState({
                kademliaPingResponse: '',
                kademliaLookupResponse: []
            });
        }
    }

    sendKademliaPingQuery = async (e: React.MouseEvent<HTMLElement>) => {
        const node = this.props.nodes[this.props.nodeId];
        const rpc = new KademliaRpc(`${node.ip_addr}:${node.api_port}`, KademliaRpcHttpTransport);

        let result = '';
        try {
            result = await rpc.ping();
        } catch (e) {
            console.error(`Error on kademlia ping request on ${node.ip_addr}:${node.api_port}`, e);

            return;
        }

        this.setState({
            kademliaPingResponse: result
        });
    }

    sendKademliaLookupQuery = async (e: React.MouseEvent<HTMLElement>) => {
        const node = this.props.nodes[this.props.nodeId];
        const rpc = new KademliaRpc(`${node.ip_addr}:${node.api_port}`, KademliaRpcHttpTransport);

        let result = [];
        try {
            result = await rpc.lookup(this.kademliaLookupKeyField.value);
        } catch (e) {
            console.error(`Error on kademlia lookup request on ${node.ip_addr}:${node.api_port} with key ${this.kademliaLookupKeyField.value}`, e);

            return;
        }

        this.setState({
            kademliaLookupResponse: result
        });
    }

    render(): React.ReactNode {
        return (
            <div className="box box-widget widget-user-2">
                <div className="widget-user-header bg-fluence-blue-gradient">
                    <div className="widget-user-image">
                        <span className="entity-info-box-icon">
                            <i className="ion ion-ios-checkmark-outline"/>
                        </span>
                    </div>
                    <h3 className="widget-user-username">Node console</h3>
                    <h3 className="widget-user-desc">nodeID: <b><FluenceId entityId={this.props.nodeId}/></b></h3>
                </div>
                <div className="box-footer no-padding">
                    <div className="box-body">
                        <p>
                            <button
                                type="button"
                                value="Send kademlia ping query"
                                className="btn btn-primary btn-block"
                                onClick={(e): any => this.sendKademliaPingQuery(e)}
                            >
                                Send kademlia ping query
                            </button>
                        </p>
                        {this.state.kademliaPingResponse && this.state.kademliaPingResponse !== '' ?
                            <>
                                <label htmlFor="result">Ping result:</label>
                                <p>
                                    <FluenceKademliaContact kademliaContactUri={this.state.kademliaPingResponse}/>
                                </p>
                            </> : null
                        }
                        <hr/>
                        <p>
                            <label>Lookup key:</label>
                            <textarea
                                className="form-control"
                                rows={1}
                                ref={(ref: HTMLTextAreaElement): void => { this.kademliaLookupKeyField = ref; }}
                            />
                        </p>
                        <p>
                            <button
                                type="button"
                                value="Send kademlia lookup query"
                                className="btn btn-primary btn-block"
                                onClick={(e): any => this.sendKademliaLookupQuery(e)}
                            >
                                Send kademlia lookup query
                            </button>
                        </p>
                        {this.state.kademliaLookupResponse.length > 0 ?
                            <>
                                <label htmlFor="result">Result:</label>
                                <p>
                                    {this.state.kademliaLookupResponse.map(c => <FluenceKademliaContact kademliaContactUri={c}/>)}
                                </p>
                            </> : null
                        }
                    </div>
                </div>
            </div>
        );

    }
}

const mapStateToProps = (state: ReduxState) => ({
        nodes: state.nodes,
});

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveNode,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceNodeSnippet);
