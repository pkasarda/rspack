'use strict';

const EnvironmentPlugin = require('@rspack/core').EnvironmentPlugin;
const DefinePlugin = require('@rspack/core').DefinePlugin;

process.env.WEBPACK_API_URL = 'https://api.example.com';

/** @type {import("@rspack/core").Configuration} */
module.exports = {
  target: 'node12.18',
  dotenv: true,
  plugins: [
    new EnvironmentPlugin({
      AAA: 'aaa',
      WEBPACK_API_URL: 'https://api.example.com',
    }),
    new DefinePlugin({
      'import.meta.env': JSON.stringify({
        AAA: 'aaa',
        WEBPACK_API_URL: 'https://api.example.com',
        NODE_ENV: 'production',
      }),
    }),
  ],
  experiments: {
    outputModule: true,
  },
  output: {
    module: true,
    chunkFormat: 'module',
  },
  externals: {
    fs: 'commonjs fs',
    path: 'commonjs path',
  },
};
