import * as React from "react";
import { connect } from "react-redux";
import { cutId } from "../../../utils";
import { ClipboardEvent } from "react";

interface Props {
    entityId: string;
    isLink?: boolean;
}

class FluenceId extends React.Component<Props> {
    handleCopyEvent(event: ClipboardEvent<HTMLElement>, entityId: string): void {
        event.clipboardData.setData('text/plain', entityId);
        event.preventDefault();
    }

    render(): React.ReactNode {
        const { entityId, isLink, ...restProps } = this.props;

        if (isLink) {
            return (
                <a
                    onCopy={e => this.handleCopyEvent(e, entityId)}
                    title={entityId}
                    {...restProps}
                >
                    {cutId(entityId)}
                </a>
            );
        }

        return (
            <span
                onCopy={e => this.handleCopyEvent(e, entityId)}
                title={entityId}
                {...restProps}
            >
                {cutId(entityId)}
            </span>
        );
    }
}

export default connect()(FluenceId);
