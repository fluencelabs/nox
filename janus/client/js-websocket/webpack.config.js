const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');

const production = (process.env.NODE_ENV === 'production');

const config = {
    entry: {
        app: ['./src/janus.ts']
    },
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                loader: 'ts-loader',
                exclude: /node_modules/
            },
            {
                test: /\.wasm$/,
                type: "webassembly/experimental"
            }
        ]
    },
    resolve: {
        extensions: [ '.tsx', '.ts', '.js', '.wasm' ]
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
        new CleanWebpackPlugin(),
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
