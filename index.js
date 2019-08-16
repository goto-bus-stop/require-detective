var wasm = require('./pkg')
module.exports = function detective (source) {
  return wasm.detective(source.toString())
}
module.exports.find = function find (source) {
  return wasm.find(source.toString())
}
