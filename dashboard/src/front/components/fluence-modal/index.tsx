import * as React from "react";
import {connect} from "react-redux";
import ReactModal from 'react-modal';
import {closeModal} from "../../actions";

interface State {}

interface Props {
    closeModal: typeof closeModal;
    modal: any;
}

class FluenceModal extends React.Component<Props, State> {

    cancelButtonClicked = () => {
        if (this.props.modal.cancelCallback) {
            this.props.modal.cancelCallback();
            this.props.closeModal({ resetOnce: true });
        } else {
            this.props.closeModal();
        }
    };

    okButtonClicked = () => {
        if (this.props.modal.okCallback) {
            this.props.modal.okCallback();
        }
        this.props.closeModal();
    };

    renderButtons(): React.ReactNode[] {
        if (this.props.modal.okCallback) {
            return [
                <button onClick={this.cancelButtonClicked} type="button" className="btn btn-default pull-left" data-dismiss="modal">Cancel</button>,
                <button onClick={this.okButtonClicked} type="button" className="btn btn-danger">OK</button>
            ];
        }

        return [
            <button onClick={this.cancelButtonClicked} type="button" className="btn btn-danger">OK</button>
        ];
    }

    renderModalText(): React.ReactNode {
        if (this.props.modal.deployText) {
            return <p>Working in demo mode. Please install metamask and select Rinkeby network to work in normal mode.<br/>Proceed with deploy?</p>
        }
        return <p>Working in demo mode. Please install metamask and select Rinkeby network to work in normal mode.</p>;
    }

    render(): React.ReactNode {
        return (
            <ReactModal
                isOpen={this.props.modal.modalIsOpen}
                className="react-modal-content"
                overlayClassName="react-modal-overlay"
            >
                <div className="modal-dialog">
                    <div className="modal-content">
                        <div className="modal-header">
                            <button onClick={this.cancelButtonClicked} type="button" className="close" data-dismiss="modal" aria-label="Close">
                                <span aria-hidden="true">×</span></button>
                            <h4 className="modal-title">Demo Mode</h4>
                        </div>
                        <div className="modal-body">
                            {this.renderModalText()}
                        </div>
                        <div className="modal-footer center">
                            {this.renderButtons()}
                        </div>
                    </div>
                </div>
            </ReactModal>
        );
    }
}

const mapStateToProps = (state: any) => ({
    modal: state.modal
});

const mapDispatchToProps = {
    closeModal,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceModal);
