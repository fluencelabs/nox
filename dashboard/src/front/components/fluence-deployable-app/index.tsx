import * as React from 'react';
import {connect} from 'react-redux';
import {DeployableApp, DeployableAppId, deployableApps, StorageType} from "../../../fluence/deployable";
import {deploy, deployUpload} from "../../actions";
import {Action} from "redux";
import Snippets from "./snippets";
import {cutId, remove0x, toIpfsHash} from "../../../utils";

interface State {
    loading: boolean,
}

export interface DeployUploadSate {
    uploaded: boolean
    uploading: boolean,
    data: object,
    storageHash: string,
    error?: any,
}

interface Props {
    id: DeployableAppId,
    deploy: (app: DeployableApp, appId: string, storageHash?: string) => Promise<Action>,
    deployUpload: (form: FormData) => Promise<Action>,
    deployedApp: number | undefined,
    deployedAppId: DeployableApp | undefined,
    upload: DeployUploadSate,
}

class FluenceDeployableApp extends React.Component<Props, State> {
    state: State = {
        loading: false,
    };

    uploadFormElement: HTMLInputElement;

    startDeploy = (e: React.MouseEvent<HTMLElement>, app: DeployableApp, appId: string) => {
        this.setState({loading: true});
        this.props.deploy(app, appId, this.props.upload.storageHash)
            .catch(function (err) {
                console.error("error while deploying " + JSON.stringify(err));
            })
            .then(() => this.setState({loading: false}));
    };

    startUpload = (e: React.MouseEvent<HTMLElement>, app: DeployableApp, appId: string) => {
        e.preventDefault();

        if (!this.uploadFormElement || !this.uploadFormElement.files || this.uploadFormElement.files.length == 0) {
            return;
        }

        const form = new FormData();
        form.append('file', this.uploadFormElement.files[0]);

        this.props.deployUpload(form).then(() => {
            this.setState({loading: true});
            return this.props.deploy(app, appId, this.props.upload.storageHash);
        }).catch(function (err) {
            console.error("error while deploying " + JSON.stringify(err));
        }).then(() => this.setState({loading: false}));
    };

    renderStorageHashBlock(app: DeployableApp): React.ReactNode[] {
        let block = [
            <strong><i className="fa fa-bullseye margin-r-5"/>WebAssembly package</strong>
        ];

        if (app.selfUpload && this.props.upload.storageHash == '') {
            return [];
        } else if (app.storageType == StorageType.Ipfs) {
            let storageHash = app.selfUpload ? this.props.upload.storageHash : app.storageHash;
            block.push(
                <p className="text-muted" title={storageHash}><a
                    href={'http://data.fluence.one:5001/api/v0/cat?arg=' + toIpfsHash(storageHash)}
                    title={storageHash}
                    target="_blank"
                    rel="noreferrer"
                    download>{cutId(storageHash)}</a></p>
            );
        } else {
            block.push(
                <p className="text-muted" title={app.storageHash}><a
                    href={'https://swarm-gateways.net/bzz:/' + remove0x(app.storageHash) + '/' + app.name + '.wasm'}
                    title={app.storageHash}
                    target="_blank">{cutId(app.storageHash)}</a></p>
            );
        }

        return block;
    }

    renderUploadBlock(): React.ReactNode[] {
        return ([
            <strong><i className="fa fa-bullseye margin-r-5"/>Upload .*wasm file</strong>,
            <p><input type="file" ref={(ref: HTMLInputElement) => { this.uploadFormElement = ref; }} /></p>,
            <hr/>
        ]);
    }

    renderAppInfo(app: DeployableApp, appId: string): React.ReactNode {
        return (
            <div className="box-footer no-padding">
                <div className="box-body">
                    {app.selfUpload && this.renderUploadBlock()}

                    {this.renderStorageHashBlock(app)}

                    <strong><i className="fa fa-bullseye margin-r-5"/>Cluster Size</strong>
                    <p className="text-muted">{app.clusterSize} nodes</p>
                    <hr/>

                    <p>
                        <span className="error" style={{display: !app.selfUpload && this.props.deployedAppId ? 'inline' : 'none'}}>app already deployed</span>
                        <button
                            type="button"
                            onClick={e => this.startDeploy(e, app, appId)}
                            style={{display: !app.selfUpload ? 'block' : 'none'}}
                            disabled={!!(this.props.deployedAppId || this.state.loading)}
                            className="btn btn-block btn-success btn-lg">
                            Deploy app <i style={{display: this.state.loading ? 'inline-block' : 'none'}}
                                          className="fa fa-refresh fa-spin"/>
                        </button>
                        <button
                            type="button"
                            onClick={e => this.startUpload(e, app, appId)}
                            style={{display: app.selfUpload ? 'block' : 'none'}}
                            disabled={!!(this.props.upload.uploading || this.props.upload.uploaded)}
                            className="btn btn-block btn-success btn-lg">
                            Deploy app <i style={{display: this.props.upload.uploading ? 'inline-block' : 'none'}}
                                      className="fa fa-refresh fa-spin"/>
                        </button>
                    </p>
                </div>
            </div>
        );
    }

    render(): React.ReactNode {
        const app = deployableApps[this.props.id];

        return (
            <div>
                <div className="col-md-4 col-xs-12">
                    <div className="box box-widget widget-user-2">
                        <div className="widget-user-header bg-fluence-blue-gradient">
                            <div className="widget-user-image">
                                <span className="entity-info-box-icon entity-info-box-icon-thin"><i
                                    className={app ? 'ion ion-ios-gear-outline' : 'fa fa-refresh fa-spin'}></i></span>
                            </div>
                            <h3 className="widget-user-username">{app.name}</h3>
                        </div>
                        {app && this.renderAppInfo(app, this.props.id)}
                    </div>
                </div>
                <div className="col-md-4 col-xs-12">
                    <Snippets/>
                </div>
            </div>
        );
    }
}

const mapStateToProps = (state: any) => ({
    deployedApp: state.deploy.app,
    deployedAppId: state.deploy.appId,
    upload: state.deploy.upload
});

const mapDispatchToProps = {
    deploy,
    deployUpload
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceDeployableApp);
