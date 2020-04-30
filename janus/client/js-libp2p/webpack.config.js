const path = require('path');
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
        hot: false
    };
    config.plugins = [
        ...config.plugins,
        new webpack.HotModuleReplacementPlugin()
    ];
}

module.exports = config;
