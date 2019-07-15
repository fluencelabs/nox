import { AnyAction } from 'redux';

export interface ModalState {
    modalIsOpen: boolean;
    alreadyOpened?: boolean;
    okCallback?: () => void;
    cancelCallback?: () => void;
    once?: boolean;
    deployText?: string;
}

export interface ModalOptions {
    once?: boolean;
    okCallback?: () => void;
    cancelCallback?: () => void;
    resetOnce?: boolean;
    deployText?: boolean;
}

const initialState: ModalState = { modalIsOpen: false };

export const SHOW_MODAL = 'SHOW_MODAL';
export const showModal = (options?: ModalOptions): AnyAction => {
    return {
        type: SHOW_MODAL,
        options: options || {},
    };
};

export const CLOSE_MODAL = 'CLOSE_MODAL';
export const closeModal = (options?: ModalOptions): AnyAction => {
    return {
        type: CLOSE_MODAL,
        options: options || {},
    };
};

/*
 * Reducer
 */
export default (state = initialState, action: any): ModalState => {
    switch (action.type) {
        case SHOW_MODAL: {
            let modalIsOpen = true;
            let alreadyOpened = state.alreadyOpened ? state.alreadyOpened : false;

            if (action.options.once && !alreadyOpened) {
                alreadyOpened = true;
            } else if (action.options.once && alreadyOpened) {
                modalIsOpen = false;
            }

            return {
                ...state,
                modalIsOpen,
                alreadyOpened,
                okCallback: action.options.okCallback ? action.options.okCallback : undefined,
                cancelCallback: action.options.cancelCallback ? action.options.cancelCallback : undefined,
                once: action.options.once || false,
                deployText: action.options.deployText || false,
            };
        }
        case CLOSE_MODAL: {
            const alreadyOpened = action.options.resetOnce ? false : state.alreadyOpened;

            return {
                ...state,
                modalIsOpen: false,
                okCallback: undefined,
                cancelCallback: undefined,
                once: undefined,
                deployText: undefined,
                alreadyOpened,
            };
        }
        default: {
            return state;
        }
    }
};
