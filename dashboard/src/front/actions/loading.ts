
export const DISPLAY_LOADING = 'DISPLAY_LOADING';
export const displayLoading = () => {
    return {
        type: DISPLAY_LOADING,
    };
};

export const HIDE_LOADING = 'HIDE_LOADING';
export const hideLoading = () => {
    return {
        type: HIDE_LOADING,
    };
};

/*
 * Reducer
 */
export default (state = { isLoading: false, counter: 0 }, action: any) => {
    switch (action.type) {
        case DISPLAY_LOADING: {
            return {
                ...state,
                counter: state.counter + 1,
                isLoading: true,
            };
        }
        case HIDE_LOADING: {
            const newCounter = state.counter - 1;
            return {
                ...state,
                counter: newCounter >= 0 ? newCounter : 0,
                isLoading: newCounter > 0,
            };;
        }
        default: {
            return state;
        }
    }
};
