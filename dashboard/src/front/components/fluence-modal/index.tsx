import * as React from 'react';
import { connect } from 'react-redux';
import ReactModal from 'react-modal';
import { closeModal } from '../../actions';
import { ReduxState } from '../../app';
import { enableMetamask } from '../../../fluence/contract';

interface State {}

interface Props {
    closeModal: typeof closeModal;
    modal: any;
    showMetamaskEnableButton: boolean;
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

    enableMetamaskButtonClicked = () => {
        enableMetamask().then(() => this.props.closeModal());
    };

    renderButtons(): React.ReactNode[] {
        const buttons = [];

        if (this.props.showMetamaskEnableButton) {
            buttons.push(
                <button onClick={this.enableMetamaskButtonClicked} type="button" className="btn btn-success pull-left">Enable Metamask</button>
            );
        }

        if (this.props.modal.okCallback) {
            buttons.push([
                <button onClick={this.okButtonClicked} type="button" className="btn btn-danger pull-left">Deploy in DEMO</button>,
                <button onClick={this.cancelButtonClicked} type="button" className="btn btn-default pull-left" data-dismiss="modal">Cancel</button>,
            ]);

            return buttons;
        }

        buttons.push(
            <button onClick={this.cancelButtonClicked} type="button" className="btn btn-danger pull-left">Stay in DEMO</button>
        );

        return buttons;
    }

    renderModalText(): React.ReactNode | React.ReactNode[] {
        const elements = [
            <p>WARNING</p>,
            <p>You are in demo mode: you won’t be able to delete the apps you create, the data you upload might be tampered with by any other user.</p>,
            <p>For the full experience, please, login into Metamask (if you don’t have one, <a href="https://metamask.io/" target="_blank">get one here</a>) and switch to the Rinkeby test network.</p>,
        ];

        if (this.props.modal.deployText) {
            elements.push(
                <p>Proceed with deploy anyway?</p>
            );
        }

        return elements;
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

const mapStateToProps = (state: ReduxState) => ({
    modal: state.modal,
    showMetamaskEnableButton: !state.ethereumConnection.isMetamaskProviderActive && state.ethereumConnection.isMetamaskAvailable
});

const mapDispatchToProps = {
    closeModal,
};

export default connect(mapStateToProps, mapDispatchToProps)(FluenceModal);
