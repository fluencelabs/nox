const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const CleanWebpackPlugin = require('clean-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const { CheckerPlugin } = require('awesome-typescript-loader');

module.exports = {
    entry: {
        app: ['./src/fluence.ts']
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
                loader: 'awesome-typescript-loader',
                exclude: /node_modules/
            }
        ]
    },
    resolve: {
        extensions: [ '.tsx', '.ts', '.js' ]
    },
    output: {
        filename: 'bundle.js',
        path: path.resolve(__dirname, 'bundle')
    },
    node: {
        fs: 'empty'
    },
    plugins: [
        new CleanWebpackPlugin(['bundle']),
        new CheckerPlugin(),
        new HtmlWebpackPlugin(),
        new webpack.HotModuleReplacementPlugin()
    ]
};