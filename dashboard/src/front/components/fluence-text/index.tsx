import * as React from "react";
import {connect} from "react-redux";
import {FluenceEntityType} from "../app";
import {AppRef, NodeRef} from "../../../fluence";

interface State {}

interface Props {
    appRefs: AppRef[];
    nodeRefs: NodeRef[];
    userAddress: string;
    entityType: FluenceEntityType;
    isMetamaskActive: boolean;
}

class FluenceText extends React.Component<Props, State> {

    renderText(): React.ReactNode {
        switch (this.props.entityType) {
            case FluenceEntityType.DeployableApp: {
                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Instant Deploy</h3>
                        </div>
                        <div className="box-body">
                            <p>Fluence allows to create fully decentralized applications meaning you can host your app’s
                                frontend on IPFS and backend with database on Fluence.</p>

                            <p>We already have a production-grade NoSQL database — Redis (including Lua support, which
                                allows you to have stored procedures, for example). Or you could play around with an SQL
                                DB written in Rust.</p>

                            <p>Uploading your own code would require compiling it into Wasm — <a
                                href="https://fluence.dev/docs/quick-publish" target="_blank">follow this guide</a> for
                                the instructions.</p>
                        </div>
                    </div>
                );
            }
            case FluenceEntityType.App: {
                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Applications</h3>
                        </div>
                        <div className="box-body">
                            <p>Click on any application to reveal the state of the cluster that runs it, and for
                                databases - even issue some queries.</p>
                        </div>
                    </div>
                );
            }
            case FluenceEntityType.Node: {
                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Nodes</h3>
                        </div>
                        <div className="box-body">
                            <p>Nodes in the Fluence network do all the hard work. When an app is deployed to the
                                network, several nodes form a cluster with Tendermint consensus to run this specific
                                app. Here you can check the status of each node individually.</p>

                            <p>If you want to find out, which of them runs which app — check out the Applications
                                section.</p>

                            <p>Want to set up your own node? Follow <a
                                href="https://fluence.dev/docs/fluence-miners-guide" target="_blank">this guide</a>.
                                Right now this is for test purposes only, but in the mainnet miners will be able to run
                                the nodes and host apps for profit.</p>
                        </div>
                    </div>
                );
            }
            case FluenceEntityType.Account: {
                //TODO: bad idea, logic duplicated, fix
                const appsCount = this.props.appRefs.filter(ref => ref.owner.toUpperCase() === this.props.userAddress.toUpperCase()).length;
                const nodesCount = this.props.nodeRefs.filter(ref => ref.owner.toUpperCase() === this.props.userAddress.toUpperCase()).length;

                if (!this.props.isMetamaskActive) {
                    return (
                        <div className="box box-primary">
                            <div className="box-header with-border">
                                <h3 className="box-title">Account in Demo Mode</h3>
                            </div>
                            <div className="box-body">
                                <p> Fluence network provides developers with the opportunity to create rich
                                    decentralized applications, meaning these apps have all the features one might need:
                                    database, complex backend logic, tools and integrations AND they run in a trustless
                                    decentralized environment without sacrificing performance or cost efficiency.</p>

                                <p>Try deploying an app! You can do it in few clicks:</p>

                                <ul>
                                    <li>go to “Instant Deploy”</li>
                                    <li>pick Redis.</li>
                                    <li>punch the big green Deploy button</li>
                                    <li>done! You can find your app in the Applications section using its app ID.</li>
                                </ul>

                                <p>Right now you’re in Demo Mode, login with Metamask for full-featured access.</p>
                            </div>
                        </div>
                    );
                }

                if (appsCount + nodesCount > 0) {
                    return (
                        <div className="box box-primary">
                            <div className="box-header with-border">
                                <h3 className="box-title">Account</h3>
                            </div>
                            <div className="box-body">
                                <p>You can find all your deployed applications and nodes here. If you’ve already feel
                                    comfortable enough with the databases available in “Instant Deploy”, try building
                                    and deploying your own application with <a target="_blank"
                                                href="https://fluence.dev/docs/quickstart">our guide</a>!</p>
                            </div>
                        </div>
                    );

                }

                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Account</h3>
                        </div>
                        <div className="box-body">
                            <p>Looks like you have your Metamask set up! Now it’s time to deploy your first app!</p>

                            <p>You can do this in several ways:</p>

                            <ul>
                                <li>using the “Instant Deploy” section</li>
                                <li>deploy your app with <a target="_blank"
                                                            href="https://fluence.dev/docs/fluence-cli">Fluence CLI</a>
                                </li>
                                <li>upload your app’s code right from the dashboard</li>
                            </ul>

                            <p>Check out <a target="_blank" href="https://fluence.dev/docs/quickstart">the
                                quickstart guide</a> for more.</p>
                        </div>
                    </div>
                );
            }
            default: {
                return (
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Dashboard</h3>
                        </div>
                        <div className="box-body">
                            <p><a href="https://fluence.network/" target="_blank">Fluence</a> is a robust distributed
                                cloud for databases and web applications. Please, mind that the project is a work in
                                progress and right now the network is in the test state.</p>

                            <p>Using this dashboard you can:</p>

                            <ul>
                                <li>Instantly deploy SQL/NoSQL database applications in a decentralized
                                    environment.</li>
                                <li>Monitor the state of your applications.</li>
                                <li>Monitor the state of the nodes in the network.</li>
                                <li>Control your apps and nodes using your Metamask account.</li>
                            </ul>

                            <p>If you have any questions or need help with your setup, please reach out to us on <a
                                href="https://discord.gg/AjfbDKQ" target="_blank">Discord</a> or <a
                                href="https://t.me/fluence_en" target="_blank">Telegram</a>!</p>
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
    isMetamaskActive: state.ethereumConnection.isMetamaskProviderActive,
});

const mapDispatchToProps = {

};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceText);
