var wasm = require('./pkg')
module.exports = function detective (source, options) {
  return wasm.detective(source.toString(), toOptions(options))
}
module.exports.find = function find (source, options) {
  return wasm.find(source.toString(), toOptions(options))
}

function toOptions (options) {
  var wopts = wasm.Options.new()
  if (!options) return wopts
  if (options.word) wopts = wopts.word(options.word)
  return wopts
}
