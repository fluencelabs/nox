import * as React from 'react';
import {connect} from 'react-redux';
import {DeployableApp, DeployableAppId, deployableApps} from "../../../fluence/deployable";
import {deploy} from "../../actions/deployable/deploy";
import {Action} from "redux";
import {none, Option} from "ts-option";
import {stringify} from "querystring";

interface State {
    appId: number|undefined,
    app: DeployableApp|undefined,
}

interface Props {
    appId: number|undefined,
    app: DeployableApp|undefined,
}

class Snippets extends React.Component<Props, State> {
    state: State = {
        appId: undefined,
        app: undefined
    };

    render(): React.ReactNode {
        console.log("render " + stringify(this.props));
        if (this.props.app != undefined && this.props.appId != undefined) {
            return (
                <div className="col-md-4 col-xs-12">
                    <div className="box box-primary">
                        <div className="box-header with-border">
                            <h3 className="box-title">Connect to {this.props.app.name}</h3>
                        </div>
                        <div className="box-body">
                            <p>Snippet</p>
                            <p>{this.props.appId}</p>
                        </div>
                    </div>
                </div>
            );
        } else {
            return null
        }
    }
}

const mapStateToProps = (state: any) => {
    console.log("mapStateToProps " + JSON.stringify(state));
    return {
        app: state.app,
        appId: state.appId
    }
};

const mapDispatchToProps = {
};

export default connect(mapStateToProps, mapDispatchToProps)(Snippets);
