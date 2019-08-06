import * as React from 'react';
import { connect } from 'react-redux';
import { Contact } from 'fluence-kademlia';
import { cutId } from '../../../utils';
import { ClipboardEvent } from 'react';

import './style.css';

interface Props {
    kademliaContactUri: string;
}

class FluenceKademliaContact extends React.Component<Props> {
    handleCopyEvent(event: ClipboardEvent<HTMLElement>, data: string): void {
        event.clipboardData.setData('text/plain', data);
        event.preventDefault();
    }

    render(): React.ReactNode {
        const { kademliaContactUri, ...restProps } = this.props;
        const contact = Contact.fromUri(kademliaContactUri);
        const signatureIcon = contact.isSignatureValid() ?
            <i className="fa fa-check-circle" title="Signature valid"/> :
            <i className="fa  fa-times-circle" title="Signature invalid"/>;

        return (
            <>
                {signatureIcon}
                &nbsp;
                <span
                    onCopy={e => this.handleCopyEvent(e, kademliaContactUri)}
                    title={kademliaContactUri}
                    {...restProps}
                >
                    {`fluence://${cutId(contact.getPublicKey())}:${cutId(contact.getSignature())}@${contact.getHostPort()}`}
                </span>
                <p className="kadKey">kad&nbsp;<i className="fa fa-key" />:&nbsp;{contact.getKademliaKey()}</p>
            </>
        );
    }
}

export default connect()(FluenceKademliaContact);
