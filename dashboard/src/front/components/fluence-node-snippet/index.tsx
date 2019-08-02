import * as React from 'react';
import { connect } from 'react-redux';
import { Action } from 'redux';
import { Contact, KademliaRpc, KademliaRpcHttpTransport } from 'fluence-kademlia';
import { displayLoading, hideLoading, retrieveNode, } from '../../actions';
import FluenceId from '../fluence-id';
import { ReduxState } from '../../app';
import { NodeId, Node } from '../../../fluence';

interface Props {
    nodeId: NodeId;
    nodes: {
        [key: string]: Node;
    };
    retrieveNode: (nodeId: NodeId) => Promise<Action>;
    displayLoading: typeof displayLoading;
    hideLoading: typeof hideLoading;
}

class FluenceNodeSnippet extends React.Component<Props> {

    kademliaPingResponseField: HTMLTextAreaElement;

    kademliaLookupKeyField: HTMLTextAreaElement;
    kademliaLookupResponseField: HTMLTextAreaElement;

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

            this.kademliaPingResponseField.value = '';
            this.kademliaLookupKeyField.value = '';
            this.kademliaLookupResponseField.value = '';
        }
    }

    sendKademliaPingQuery = async (e: React.MouseEvent<HTMLElement>) => {
        const node = this.props.nodes[this.props.nodeId];
        const rpc = new KademliaRpc(`${node.ip_addr}:${node.api_port}`, KademliaRpcHttpTransport);

        let result = '';
        try {
            result = await rpc.ping();
        } catch (e) {
            this.kademliaPingResponseField.value = e;

            return;
        }

        const contact = Contact.fromUri(result);
        const signature = `Signature ${contact.isSignatureValid() ? 'valid' : 'invalid'}`;
        const kademliaKey =`Kademlia key: ${contact.getKademliaKey()}`;
        this.kademliaPingResponseField.value = `${result}\n\n${kademliaKey}\n${signature}`;
    }

    sendKademliaLookupQuery = async (e: React.MouseEvent<HTMLElement>) => {
        const node = this.props.nodes[this.props.nodeId];
        const rpc = new KademliaRpc(`${node.ip_addr}:${node.api_port}`, KademliaRpcHttpTransport);

        let result = [];
        try {
            result = await rpc.lookup(this.kademliaLookupKeyField.value);
        } catch (e) {
            this.kademliaLookupResponseField.value = e;

            return;
        }

        this.kademliaLookupResponseField.value = JSON.stringify(result);
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
                        <label htmlFor="result">Ping result:</label>
                        <textarea
                            className="form-control"
                            rows={5}
                            readOnly={true}
                            ref={(ref: HTMLTextAreaElement): void => { this.kademliaPingResponseField = ref; }}
                        />
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
                        <label htmlFor="result">Result:</label>
                        <textarea
                            className="form-control"
                            rows={8}
                            readOnly={true}
                            ref={(ref: HTMLTextAreaElement): void => { this.kademliaLookupResponseField = ref; }}
                        />

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
