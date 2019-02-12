const path = require('path');
const HtmlPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const CleanPlugin = require('clean-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const production = (process.env.NODE_ENV === 'production');

const host = process.env.LISTEN_HOST || '0.0.0.0';
const serverPort = parseInt(process.env.LISTEN_PORT, 10) || 8082;
const devServerPort = serverPort + 10000;

const config = {
  node: {
    fs: 'empty',
  },
  mode: production ? 'production' : 'development',
  entry: {
    app: [
      '@babel/polyfill',
      './src/front/app.tsx',
    ],
  },
  output: {
    path: `${__dirname}/build/`,
    filename: 'static/[name].[chunkhash].js',
    chunkFilename: 'static/[name].[chunkhash].js',
    publicPath: '/',
  },
  devServer: {
    contentBase: false,
    public: process.env.PUBLIC,
    host,
    port: devServerPort,
    historyApiFallback: {
      rewrites: [
        { from: /.*/, to: '/' },
      ],
    },
    disableHostCheck: true,
  },
  module: {
    rules: [
      {
        resource: {
          test: /\.js$/,
          include: [
            path.resolve(__dirname, 'src'),
          ],
        },
        use: [
          { loader: 'babel-loader' },
        ],
      },
      {
        resource: { test: /\.(css|less)$/ },
        use: [
          production ? MiniCssExtractPlugin.loader : 'style-loader',
          'css-loader',
          'less-loader',
        ],
      },
      {
        resource: { test: /\.(svg|woff|woff2|ttf|eot)$/ },
        use: [
          {
            loader: 'file-loader',
            options: {
              name: '[name].[ext]',
              outputPath: 'static/',
            },
          },
        ],
      },
      {
        test: /\.ts(x?)$/,
        exclude: /node_modules/,
        loader: 'ts-loader',
        options: {
          context: __dirname,
          configFile: 'tsconfig.client.json'
        },
      },
    ],
  },
  resolve: {
    symlinks: false,
    extensions: ['.js', '.json', '.ts', '.tsx'],
    alias: {
      'lodash-es': 'lodash',
    },
  },
  plugins: [
    new webpack.IgnorePlugin(/^\.\/locale$/, /moment$/),
    new HtmlPlugin({
      filename: 'index.html',
      template: 'src/front/static/index.html',
      inject: 'body',
    }),
    new CopyWebpackPlugin([{
      from: path.resolve(__dirname, 'src/front/assets'),
      to: path.resolve(__dirname, 'build/static/assets'),
    }]),
  ].concat(production ? [
    new CleanPlugin(['build']),
    new MiniCssExtractPlugin({
      filename: 'static/[name].css',
      allChunks: true,
    }),
  ] : []),
};

module.exports = config;
