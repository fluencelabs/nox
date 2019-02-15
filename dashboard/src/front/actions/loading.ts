
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
export default (state = false, action: any) => {
    switch (action.type) {
        case DISPLAY_LOADING: {
            return true;
        }
        case HIDE_LOADING: {
            return false;
        }
        default: {
            return state;
        }
    }
};
