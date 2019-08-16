# require-detective

find require() calls in commonjs modules, quickly using webassembly

This is a wasm-based alternative to the [detective](https://github.com/browserify/detective) module used in browserify, built on the [RESSA](https://github.com/FreeMasen/RESSA) parser. It can be used as a Rust crate and as a CommonJS module.

Node.js: [Install](#nodejs-installation) - [Usage](#nodejs-usage) - Rust: [Install](#rust-installation) - [Usage](#rust-usage) - [License: Apache-2.0](#license)

[![npm][npm-image]][npm-url]
[![travis][travis-image]][travis-url]

[npm-image]: https://img.shields.io/npm/v/detective-wasm.svg?style=flat-square
[npm-url]: https://www.npmjs.com/package/detective-wasm
[travis-image]: https://img.shields.io/travis/com/goto-bus-stop/require-detective.svg?style=flat-square
[travis-url]: https://travis-ci.com/goto-bus-stop/require-detective

## Node.js Installation

```
npm install detective-wasm
```

## Node.js Usage

```js
var detective = require('detective-wasm')

var requires = detective(`
  var a = require('a')
  var b = /**/require//
  (   "b");
`)
// → ['a', 'b']
```

## Rust Installation

Add to Cargo.toml:
```toml
[dependencies]
require-detective = "^0.1.0"
```

## Rust Usage

Please see [docs.rs](https://docs.rs/require-detective).

## License

[Apache-2.0](LICENSE.md)
The tests in test/ originate from [detective](https://github.com/browserify/detective), MIT license.
