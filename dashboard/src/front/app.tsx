import * as React from 'react';
import ReactDOM from 'react-dom';
import ReduxThunk from 'redux-thunk';
import { compose, createStore, combineReducers, applyMiddleware } from 'redux';
import { Provider } from 'react-redux';
import { rootTagId } from '../constants';
import { reducers } from './actions';
import initialState from './initial-state';
import DashboardApp from './components/app';

const composeEnhancers = (window as any)['__REDUX_DEVTOOLS_EXTENSION_COMPOSE__'] as typeof compose || compose;

const middlewares = [
    ReduxThunk
];

const store = createStore(
    combineReducers(reducers),
    initialState,
    composeEnhancers(
        applyMiddleware(...middlewares),
    ),
);

const providers: ((el: JSX.Element) => JSX.Element)[] = [
    (children): JSX.Element => (<Provider store={store}>{children}</Provider>),
];

const body = providers.reverse().reduce((memo, callback) => callback(memo), (<DashboardApp />));

ReactDOM.render(body, document.getElementById(rootTagId));
