import * as React from 'react';
import ReactDOM from 'react-dom';
import ReduxThunk from 'redux-thunk';
import { compose, createStore, combineReducers, applyMiddleware } from 'redux';
import { createBrowserHistory } from 'history';
import { connectRouter, routerMiddleware, ConnectedRouter } from 'connected-react-router';
import { BrowserRouter as Router, Route } from "react-router-dom";
import { Provider } from 'react-redux';
import { rootTagId } from '../constants';
import { reducers } from './actions';
import initialState from './initial-state';
import DashboardApp from './components/app';

const composeEnhancers = (window as any)['__REDUX_DEVTOOLS_EXTENSION_COMPOSE__'] as typeof compose || compose;

export const history = createBrowserHistory();

const middlewares = [
    routerMiddleware(history),
    ReduxThunk
];

const store = createStore(
    combineReducers({
        router: connectRouter(history),
        ...reducers
    }),
    initialState,
    composeEnhancers(
        applyMiddleware(...middlewares),
    ),
);

const providers: ((el: JSX.Element) => JSX.Element)[] = [
    (children): JSX.Element => (
        <Provider store={store}>
            <ConnectedRouter history={history}>
                <Router>
                    {children}
                </Router>
            </ConnectedRouter>
        </Provider>),
];

const body = providers.reverse().reduce((memo, callback) => callback(memo), (<Route path={['/:entityType/:entityId/:appId', '/:entityType/:entityId', '/']} component={DashboardApp} />));

ReactDOM.render(body, document.getElementById(rootTagId));
