import * as React from 'react';
import {connect} from 'react-redux';
import {DeployableApp} from "../../../fluence/deployable";
import {defaultContractAddress} from "../../../constants";

interface State {
}

interface Props {
    appId: number | undefined,
    app: DeployableApp | undefined,
}

class Snippets extends React.Component<Props, State> {
    state: State = {};

    render(): React.ReactNode {
        if (this.props.app != undefined && this.props.appId != undefined) {
            return (
                <div className="box box-widget widget-user-2">
                    <div className="widget-user-header bg-fluence-blue-gradient">
                        <div className="widget-user-image">
                            <span className="entity-info-box-icon">
                                {/*<i className={app ? 'ion ion-ios-gear-outline' : 'fa fa-refresh fa-spin'}/>*/}
                            </span>
                        </div>
                        <h3 className="widget-user-username">Connect to {this.props.app.name}</h3>
                    </div>
                    <div className="box-footer no-padding">
                        <div className="box-body">
                            <p>Install dependency:</p>
                            <pre>npm install --save fluence@0.1.16</pre>
                            <p>Connect to {this.props.app.name}:</p>
                            <pre>{`
import * as fluence from "fluence";

let contract = ${defaultContractAddress};
fluence.connect(contract, ${this.props.appId}, "http://data.fluence.one").then((s) => {
    console.log("Session created");
    window.session = s;
})
                            `}
                            </pre>
                            <p>Call invoke:</p>
                            <pre>session.invoke("CREATE TABLE users ...").result().then(...)</pre>
                        </div>
                    </div>
                </div>
            )
        } else {
            return null
        }
    }
}

const mapStateToProps = (state: any) => {
    return {
        app: state.snippets.app,
        appId: state.snippets.appId
    }
};

const mapDispatchToProps = {};

export default connect(mapStateToProps, mapDispatchToProps)(Snippets);
