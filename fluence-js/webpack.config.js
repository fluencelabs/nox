const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const CleanWebpackPlugin = require('clean-webpack-plugin');
const { CheckerPlugin } = require('awesome-typescript-loader');

const production = (process.env.NODE_ENV === 'production');

const config = {
    entry: {
        app: ['./src/fluence.ts']
    },
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
        path: path.resolve(__dirname, 'bundle'),
        library: 'fluence'
    },
    node: {
        fs: 'empty'
    },
    plugins: [
        new CleanWebpackPlugin(['bundle']),
        new CheckerPlugin(),
    ]
};

if (production) {
    config.mode = 'production';
} else {
    config.mode = 'development';
    config.devtool = 'inline-source-map';
    config.devServer = {
        contentBase: './bundle',
        hot: true
    };
    config.plugins = [
        ...config.plugins,
        new HtmlWebpackPlugin(),
        new webpack.HotModuleReplacementPlugin()
    ];
}

module.exports = config;
