import * as React from "react";
import {connect} from "react-redux";
import {FluenceEntityType} from "../app";
import {AppRef, NodeRef} from "../../../fluence";

interface State {}

interface Props {
    appRefs: AppRef[];
    nodeRefs: NodeRef[];
    userAddress: string;
    entityType: FluenceEntityType
}

class FluenceText extends React.Component<Props, State> {

    renderText(): React.ReactNode {
        switch (this.props.entityType) {
            case FluenceEntityType.App: {
                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Fluence Network text for App</h3>
                        </div>
                        <div className="box-body">
                            <p>Fluence is a permissionless decentralized database platform, trustless and efficient.
                                With Fluence, you will be able to deploy an SQL/NoSQL database with just a few clicks!</p>

                            <p>Fluence Network is a work in progress and is currently in the devnet state. Feel free to play
                                with it and build demo DApps on top of your deployed database, but keep in mind that the API
                                is not stabilized yet and might change in the future.</p>

                            <p>If you have any questions or need help with your setup, please reach out to us at <a
                                href="https://discord.gg/AjfbDKQ">Discord</a>!
                                You can also take a look at the Fluence documentation.</p>
                        </div>
                    </div>
                );
            }
            case FluenceEntityType.Account: {
                //TODO: bad idea, logic duplicated, fix
                const appsCount = this.props.appRefs.filter(ref => ref.owner.toUpperCase() === this.props.userAddress.toUpperCase()).length;
                const nodesCount = this.props.nodeRefs.filter(ref => ref.owner.toUpperCase() === this.props.userAddress.toUpperCase()).length;
                if (appsCount + nodesCount > 0) {
                    return (
                        <div className="box box-primary">
                            <div className="box-header with-border">
                                <h3 className="box-title">Fluence Network text for Account with apps</h3>
                            </div>
                            <div className="box-body">
                                <p>Fluence is a permissionless decentralized database platform, trustless and efficient.
                                    With Fluence, you will be able to deploy an SQL/NoSQL database with just a few clicks!</p>

                                <p>Fluence Network is a work in progress and is currently in the devnet state. Feel free to play
                                    with it and build demo DApps on top of your deployed database, but keep in mind that the API
                                    is not stabilized yet and might change in the future.</p>

                                <p>If you have any questions or need help with your setup, please reach out to us at <a
                                    href="https://discord.gg/AjfbDKQ">Discord</a>!
                                    You can also take a look at the Fluence documentation.</p>
                            </div>
                        </div>
                    );
                }

                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Fluence Network text for Account WITHOUT apps</h3>
                        </div>
                        <div className="box-body">
                            <p>Fluence is a permissionless decentralized database platform, trustless and efficient.
                                With Fluence, you will be able to deploy an SQL/NoSQL database with just a few clicks!</p>

                            <p>Fluence Network is a work in progress and is currently in the devnet state. Feel free to play
                                with it and build demo DApps on top of your deployed database, but keep in mind that the API
                                is not stabilized yet and might change in the future.</p>

                            <p>If you have any questions or need help with your setup, please reach out to us at <a
                                href="https://discord.gg/AjfbDKQ">Discord</a>!
                                You can also take a look at the Fluence documentation.</p>
                        </div>
                    </div>
                );
            }
            default: {
                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Fluence Network text default</h3>
                        </div>
                        <div className="box-body">
                            <p>Fluence is a permissionless decentralized database platform, trustless and efficient.
                                With Fluence, you will be able to deploy an SQL/NoSQL database with just a few clicks!</p>

                            <p>Fluence Network is a work in progress and is currently in the devnet state. Feel free to play
                                with it and build demo DApps on top of your deployed database, but keep in mind that the API
                                is not stabilized yet and might change in the future.</p>

                            <p>If you have any questions or need help with your setup, please reach out to us at <a
                                href="https://discord.gg/AjfbDKQ">Discord</a>!
                                You can also take a look at the Fluence documentation.</p>
                        </div>
                    </div>
                );
            }
        }
    }

    render(): React.ReactNode {
        return this.renderText();
    }
}

const mapStateToProps = (state: any) => ({
    nodeRefs: state.nodeRefs,
    appRefs: state.appRefs,
    userAddress: state.ethereumConnection.userAddress,
});

const mapDispatchToProps = {

};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceText);
