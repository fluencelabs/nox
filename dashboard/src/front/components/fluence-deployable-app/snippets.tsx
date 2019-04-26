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
    requestSending: Boolean;
}

interface Props {
    deployState: string | undefined;
    deployedAppId: AppId | undefined,
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
    state: State = {
        requestSending: false
    };

    loadData(): void {
        this.props.displayLoading();
        this.props.retrieveApp(String(this.props.deployedAppId)).then(this.props.hideLoading).catch(this.props.hideLoading);
    }

    componentDidUpdate(prevProps: Props): void {
        if (this.props.deployedAppId && prevProps.deployedAppId !== this.props.deployedAppId) {
            this.loadData();
        }
    }

    componentDidMount(): void {
        if (this.props.deployedAppId && !this.props.apps[this.props.deployedAppId]) {
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

    renderInteractiveSnippet(appId: number, shortName: string, defaultQueries?: string[]): React.ReactNode[] {

        const queryId = `query${this.props.deployedAppId}`;
        const iconId = `icon${this.props.deployedAppId}`;
        const buttonId = `button${this.props.deployedAppId}`;
        const resultId = `result${this.props.deployedAppId}`;

        const inputField = function (): HTMLInputElement {
            return window.document.getElementById(queryId) as HTMLInputElement;
        };
        const outputField = function (): HTMLInputElement {
            return window.document.getElementById(resultId) as HTMLInputElement;
        };
        const iconEl = function (): HTMLElement {
            return window.document.getElementById(iconId) as HTMLElement;
        };
        const buttonEl = function (): HTMLButtonElement {
            return window.document.getElementById(buttonId) as HTMLButtonElement;
        };

        let session: AppSession;

        let auth = undefined;
        if (shortName === 'llamadb') {
            auth = llamaPrivateKey;
        }

        fluence.connect(defaultContractAddress, appId.toString(), fluenceNodeAddr, auth).then(s => {
            session = s;
            buttonEl().disabled = false;
            buttonEl().onclick = function (): any {
                if (inputField().value.trim().length !== 0) {
                    iconEl().style.display = 'inline-block';
                    buttonEl().disabled = true;
                    const queries = inputField().value.trim().split('\n').filter(s => {
                        return s.trim().length !== 0;
                    });
                    const results = queries.map(q => {
                        const res = session.request(q).result();

                        return res.then(r => {
                            return parser(r.asString().trim());
                        });
                    });
                    const fullResult: Promise<string[]> = Promise.all(results);
                    fullResult.then(rs => {
                        outputField().value = rs.map((r, i) => {
                            return `>>>${queries[i]}\n${r}`;
                        }).join('\n\n');
                    });
                    fullResult.finally(() => {
                        iconEl().style.display = 'none';
                        buttonEl().disabled = false;
                    });
                }
            };
            iconEl().style.display = 'none';
        });

        const defaultText = (defaultQueries) ? defaultQueries.join('\n') : '';

        let parser: (s: string) => string;
        if (shortName === 'llamadb') {
            parser = function (s: string): string {
                return s.replace('_0\n', '');
            };
        } else if (shortName === 'Redis') {
            parser = function (s: string): string {
                let res = s;
                if (s.startsWith(':')) {
                    res = s.replace(':', '');
                }

                return res.split('\n')
                    .map(r => {
                        return r.trim();
                    })
                    .filter(r => {
                        return !r.startsWith('$') && r !== String.fromCharCode(0, 0, 0, 0);
                    })
                    .filter(r => {
                        return r.length !== 0;
                    })
                    .join('\n');
            };
        } else {
            parser = function (s: string): string {
                return s;
            };
        }

        return ([
            <p>
                <label htmlFor={queryId}>Type queries:</label>
                <textarea className="form-control" rows={4} id={queryId}>{defaultText}</textarea>
            </p>,
            <p>
                <button type="button" value="Submit query" id={buttonId}
                        className="btn btn-primary btn-block"
                        disabled={true}
                >
                    Submit query <i id={iconId} style={{display: 'inline-block'}} className="fa fa-refresh fa-spin"/>
                </button>
            </p>,
            <label htmlFor="result">Result:</label>,
            <textarea id={resultId} className="form-control" rows={6} readOnly={true}/>

        ]);
    }

    renderAppSnippets(): React.ReactNode[] {
        return ([
            <button type="button"
                    onClick={e => window.open(`http://sql.fluence.network?appId=${this.props.deployedAppId}&privateKey=${llamaPrivateKey}`, '_blank')}
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
let appId = ${this.props.deployedAppId};                                                                      // Deployed database id
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

    renderUploadedAppSnippets(shortName: string): React.ReactNode[] {
        const request = (shortName === 'Redis') ? 'SET A 10' : '<enter your request here>';
        const requestForResult = (shortName === 'Redis') ? 'GET A' : '<enter your request here>';

        return ([
            this.renderTrxHashBlock(),
            <p>
                <b>
                    Connect to {shortName} directly in the browser console.
                </b>
            </p>,
            <pre>{`let contract = "${defaultContractAddress}";                         // Fluence contract address
let appId = ${this.props.deployedAppId};                                                                      // Deployed database id
let ethereumUrl = "${fluenceNodeAddr}";                                    // Ethereum light node URL

// Connect to your app
fluence.connect(contract, appId, ethereumUrl).then((s) => {
    console.log("Session created");
    window.session = s;
});

// Send a request
session.request("${request}");

// Send a request, and read its result
session.request("${requestForResult}").result().then((r) => {
    console.log("Result: " + r.asString());
});`}
            </pre>,
        ]);
    }

    render(): React.ReactNode {
        if (this.props.app !== undefined && this.props.deployedAppId !== undefined) {
            const appInfo = this.props.apps[this.props.deployedAppId];
            return (
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-fluence-blue-gradient">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon">
                                <i className="ion ion-ios-checkmark-outline"/>
                            </span>
                        </div>
                        <h3 className="widget-user-username">Try {this.props.app.shortName}</h3>
                        <h3 className="widget-user-desc">appID: <b>{this.props.deployedAppId}</b></h3>
                    </div>
                    <div className="box-footer no-padding">
                        <div className="box-body">
                            {
                                this.renderInteractiveSnippet(Number(this.props.deployedAppId), this.props.app.shortName, this.props.app.requestExamples)
                            }
                            <hr/>
                            {(this.props.app.shortName === 'Redis' || this.props.app.selfUpload) ? this.renderUploadedAppSnippets(this.props.app.shortName) : this.renderAppSnippets()}

                            <hr/>
                            <p><strong><i className="fa fa-bullseye margin-r-5"/>Check your app's health:</strong></p>
                            {appInfo && <FluenceCluster appId={this.props.deployedAppId} cluster={appInfo.cluster}/>}
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
