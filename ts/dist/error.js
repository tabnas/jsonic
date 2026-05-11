"use strict";
/* Copyright (c) 2013-2026 Richard Rodger, MIT License */
Object.defineProperty(exports, "__esModule", { value: true });
exports.JsonicError = exports.TabnasError = exports.prop = exports.strinject = exports.trimstk = exports.errmsg = exports.errsite = exports.errinject = exports.errdesc = void 0;
/*  error.ts
 *  Parse errors. The implementation now lives in the `tabnas` package;
 *  this module re-exports it, keeping the historic `JsonicError` name
 *  available alongside the engine's `TabnasError`.
 */
var error_1 = require("tabnas/error");
Object.defineProperty(exports, "errdesc", { enumerable: true, get: function () { return error_1.errdesc; } });
Object.defineProperty(exports, "errinject", { enumerable: true, get: function () { return error_1.errinject; } });
Object.defineProperty(exports, "errsite", { enumerable: true, get: function () { return error_1.errsite; } });
Object.defineProperty(exports, "errmsg", { enumerable: true, get: function () { return error_1.errmsg; } });
Object.defineProperty(exports, "trimstk", { enumerable: true, get: function () { return error_1.trimstk; } });
Object.defineProperty(exports, "strinject", { enumerable: true, get: function () { return error_1.strinject; } });
Object.defineProperty(exports, "prop", { enumerable: true, get: function () { return error_1.prop; } });
Object.defineProperty(exports, "TabnasError", { enumerable: true, get: function () { return error_1.TabnasError; } });
Object.defineProperty(exports, "JsonicError", { enumerable: true, get: function () { return error_1.TabnasError; } });
//# sourceMappingURL=error.js.map