const path = require('path');
const webpack = require('webpack');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const production = (process.env.NODE_ENV === 'production');

const config = {
    entry: {
        app: ['./src/fluence.ts']
    },
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                loader: 'ts-loader',
                exclude: /node_modules/
            }
        ]
    },
    resolve: {
        extensions: [ '.tsx', '.ts', '.js']
    },
    output: {
        filename: 'bundle.js',
        path: path.resolve(__dirname, 'bundle'),
    },
    node: {
        fs: 'empty'
    },
    plugins: [
        new HtmlWebpackPlugin(),
        new WasmPackPlugin({
            // TODO use another path somehow
            crateDirectory: path.resolve(__dirname, "../../../aquamarine/stepper"),
            outDir:path.resolve(__dirname, "./pkg")
        }),
        new webpack.ProvidePlugin({
            TextDecoder: ['text-encoding', 'TextDecoder'],
            TextEncoder: ['text-encoding', 'TextEncoder']
        })
    ]
};

if (production) {
    config.mode = 'production';
} else {
    config.mode = 'development';
    config.devtool = 'inline-source-map';
    config.devServer = {
        contentBase: './bundle',
        hot: false
    };
    config.plugins = [
        ...config.plugins,
        new webpack.HotModuleReplacementPlugin()
    ];
}

module.exports = config;
