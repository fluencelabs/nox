const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const CleanWebpackPlugin = require('clean-webpack-plugin');
const { CheckerPlugin } = require('awesome-typescript-loader');

module.exports = {
    entry: {
        app: ['./index.ts']
    },
    devtool: 'inline-source-map',
    devServer: {
        contentBase: './bundle',
        hot: true
    },
    mode: 'development',
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                loader: 'awesome-typescript-loader'
            },
            {
                test: /\.css$/,
                use: ['style-loader', 'css-loader']
            }
        ]
    },
    resolve: {
        extensions: [ '.tsx', '.ts', '.js' ]
    },
    output: {
        filename: 'bundle.js',
        path: __dirname
    },
    node: {
        fs: 'empty'
    },
    plugins: [
        new CleanWebpackPlugin(['bundle']),
        new CheckerPlugin(),
        new webpack.HotModuleReplacementPlugin()
    ]
};
