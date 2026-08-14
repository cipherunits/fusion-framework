"use strict";

let native;
try {
  native = require("../bindings/node");
} catch {
  native = {
    hello: (name = "Fusion") => `Hello, ${name}! (from security)`,
  };
}

function hello(name = "Fusion") {
  return native.hello(name);
}

module.exports = { hello };
