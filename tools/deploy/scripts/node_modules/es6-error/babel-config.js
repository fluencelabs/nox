'use strict';

var plugins = [
  [
    "babel-plugin-transform-builtin-extend",
    {
      "globals": ["Error"],
      "approximate": true
    }
  ]
];

var env = process.env.BABEL_ENV || process.env.NODE_ENV;

var modules;

if (env === 'es') {
  modules = false;
} else if (env === 'ts') {
  modules = 'commonjs';
} else {
  modules = 'commonjs';
  plugins.push('add-module-exports');
}

module.exports = {
  "presets": [
    ["es2015", { modules: modules }]
  ],
  "plugins": plugins
};
