import * as React from 'react';
import { connect } from 'react-redux';
import { DeployableApp } from '../../../fluence/deployable';
import { defaultContractAddress, fluenceNodeAddr, llamaPrivateKey } from '../../../constants';
import { displayLoading, hideLoading, retrieveApp, } from '../../actions';
import FluenceCluster from '../fluence-cluster';
import { App, AppId } from '../../../fluence';
import { Action } from 'redux';
import { cutId } from '../../../utils';
import * as fluence from 'fluence';
import { AppSession } from 'fluence/dist/AppSession';

interface State {
}

interface Props {
    deployState: string | undefined;
    appId: number | undefined;
    app: DeployableApp | undefined;
    apps: {
        [key: string]: App;
    };
    trxHash: string;
    retrieveApp: (appId: AppId) => Promise<Action>;
    displayLoading: typeof displayLoading;
    hideLoading: typeof hideLoading;
}

class Snippets extends React.Component<Props, State> {
    state: State = {};

    loadData(): void {
        this.props.displayLoading();
        this.props.retrieveApp(String(this.props.appId)).then(this.props.hideLoading).catch(this.props.hideLoading);
    }

    componentDidUpdate(prevProps: Props): void {
        if (this.props.appId && prevProps.appId !== this.props.appId) {
            this.loadData();
        }
    }

    componentDidMount(): void {
        if (this.props.appId && !this.props.apps[this.props.appId]) {
            this.loadData();
        }
    }

    getDeployStateLabel(deployState: any): string {
        switch (deployState.state) {
            case 'prepare': {
                return 'preparing transaction...';
            }
            case 'trx': {
                return 'sending transaction...';
            }
            case 'enqueued': {
                return 'app is enqueued...';
            }
            case 'check_cluster': {
                return `${deployState.note}...`;
            }
            default: {
                return '';
            }
        }
    }

    renderTrxHashBlock(): React.ReactNode {
        if (this.props.trxHash) {
            return (
                <p>
                    Transaction hash: <a href={'https://rinkeby.etherscan.io/tx/' + this.props.trxHash}
                                         title={this.props.trxHash} className="etherscan-link"
                                         target="_blank">{cutId(this.props.trxHash)}</a>
                </p>
            );
        }
    }

    renderInteractiveSnippet(appId: number, defaultQueries?: string[]): React.ReactNode[] {
        let session: AppSession;
        fluence.connect(defaultContractAddress, appId.toString(), fluenceNodeAddr).then(s => {
            session = s;
        });

        const queryId = `query${this.props.appId}`;
        const resultId = `result${this.props.appId}`;

        const inputField: HTMLInputElement = window.document.getElementById(queryId) as HTMLInputElement;
        const outputField: HTMLInputElement = window.document.getElementById(resultId) as HTMLInputElement;

        const defaultText = (defaultQueries) ? defaultQueries.join('\n') : '';

        return ([
            <p>
                <label htmlFor={queryId}>Type queries:</label>
                <textarea className="form-control" rows={4} id={queryId}>{defaultText}</textarea>
            </p>,
            <p>
                <button type="button" value="Submit query"
                        className="btn btn-primary btn-block"
                        onClick={e => {

                            if (inputField.value.trim().length !== 0) {
                                const queries = inputField.value.trim().split('\n');
                                const results = queries.map(q => {
                                    const res = session.request(q).result();

                                    return res.then(r => {
                                        return r.asString().trim();
                                    });
                                });
                                const fullResult: Promise<string[]> = Promise.all(results);
                                fullResult.then(r => {
                                    outputField.value = r.join('\n');
                                });
                                inputField.value = '';
                            }
                        }
                        }>
                    Submit query
                </button>
            </p>,
            <label htmlFor="result">Result:</label>,
            <textarea id={resultId} className="form-control" rows={6} readOnly/>

        ]);
    }

    renderAppSnippets(): React.ReactNode[] {
        return ([
            <button type="button"
                    onClick={e => window.open(`http://sql.fluence.network?appId=${this.props.appId}&privateKey=${llamaPrivateKey}`, '_blank')}
                    className="btn btn-block btn-link">
                <i className="fa fa-external-link margin-r-5"/> <b>Open SQL DB web interface</b>
            </button>,
            <hr/>,
            this.renderTrxHashBlock(),
            <p>
                <b>
                    Or connect to {this.props.app && this.props.app.shortName} directly in the browser console.
                </b>
            </p>,
            <p> Open Developer Tools, and paste:</p>,
            <pre>{`let privateKey = "${llamaPrivateKey}"; // Authorization private key
let contract = "${defaultContractAddress}";                         // Fluence contract address
let appId = ${this.props.appId};                                                                      // Deployed database id
let ethereumUrl = "${fluenceNodeAddr}";                                    // Ethereum light node URL

fluence.connect(contract, appId, ethereumUrl, privateKey).then((s) => {
    console.log("Session created");
    window.session = s;
});`}
            </pre>,
            <p>Execute some queries:</p>,
            <pre>{`session.request("CREATE TABLE users(id int, name varchar(128), age int)");
session.request("INSERT INTO users VALUES(1, 'Sara', 23)");
session.request("INSERT INTO users VALUES(2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 27)");
session.request("SELECT AVG(age) FROM users").result().then((r) => {
    console.log("Result: " + r.asString());
});`}
            </pre>,
            <p>That's it!</p>,
            <hr/>,
            <button type="button"
                    onClick={e => window.open(`https://github.com/fluencelabs/tutorials`, '_blank')}
                    className="btn btn-block btn-link">
                    <i className="fa fa-external-link margin-r-5"/> <b>To develop your own app, follow
                GitHub Tutorials</b>
            </button>,
            <button type="button"
                    onClick={e => window.open(`https://fluence.network/docs`, '_blank')}
                    className="btn btn-block btn-link">
                    <i className="fa fa-external-link margin-r-5"/> <b>More info in the docs</b>
            </button>
        ]);
    }

    renderUploadedAppSnippets(): React.ReactNode[] {
        return ([
            this.renderTrxHashBlock(),
            <pre>{`let contract = "${defaultContractAddress}";                         // Fluence contract address
let appId = ${this.props.appId};                                                                      // Deployed database id
let ethereumUrl = "${fluenceNodeAddr}";                                    // Ethereum light node URL

// Connect to your app
fluence.connect(contract, appId, ethereumUrl).then((s) => {
    console.log("Session created");
    window.session = s;
});

// Send a request
session.request("<enter your request here>");

// Send a request, and read its result
session.request("<enter your request here>").result().then((r) => {
    console.log("Result: " + r.asString());
});`}
            </pre>,
        ]);
    }

    render(): React.ReactNode {
        if (this.props.app !== undefined && this.props.appId !== undefined) {
            const appInfo = this.props.apps[this.props.appId];
            return (
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-fluence-blue-gradient">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon">
                                <i className="ion ion-ios-checkmark-outline"/>
                            </span>
                        </div>
                        <h3 className="widget-user-username">Connect to {this.props.app.shortName}</h3>
                        <h3 className="widget-user-desc">appID: <b>{this.props.appId}</b></h3>
                    </div>
                    <div className="box-footer no-padding">
                        <div className="box-body">
                            {this.renderInteractiveSnippet(this.props.appId, this.props.app.requestExamples)}
                            <hr/>
                            { (this.props.app.shortName === 'Redis' || this.props.app.selfUpload) ? this.renderUploadedAppSnippets() : this.renderAppSnippets()}

                            <hr/>
                            <p><strong><i className="fa fa-bullseye margin-r-5"/>Check your app's health:</strong></p>
                            {appInfo && <FluenceCluster appId={this.props.appId} cluster={appInfo.cluster}/>}
                        </div>
                    </div>
                </div>
            );
        } else if (this.props.deployState) {
            return (
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-fluence-blue-gradient">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon"><i className="fa fa-refresh fa-spin"/></span>
                        </div>
                        <h3 className="widget-user-username">Deploying app</h3>
                        <h3 className="widget-user-desc">...</h3>
                    </div>
                    <div className="box-footer no-padding">
                        <div className="box-body">
                            <p>Status: {this.getDeployStateLabel(this.props.deployState)}</p>
                        </div>
                    </div>
                </div>
            );
        } else {
            return null;
        }
    }
}

const mapStateToProps = (state: any) => {
    return {
        deployState: state.deploy.deployState,
        app: state.deploy.app,
        appId: state.deploy.appId,
        trxHash: state.deploy.trxHash,
        apps: state.apps,
    };
};

const mapDispatchToProps = {
    displayLoading,
    hideLoading,
    retrieveApp,
};

export default connect(mapStateToProps, mapDispatchToProps)(Snippets);
