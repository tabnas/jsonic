"use strict";
/* Copyright (c) 2021-2026 Richard Rodger, MIT License */
Object.defineProperty(exports, "__esModule", { value: true });
exports.Debug = void 0;
/*  debug.ts
 *  Debug tools.
 *
 *  The debug plugin has been extracted into the standalone
 *  `@tabnas/debug` package (tracing hooks and a `describe()` method for
 *  engine instances). jsonic re-exports it so the historic
 *  `jsonic/debug` subpath export keeps working.
 */
var debug_1 = require("@tabnas/debug");
Object.defineProperty(exports, "Debug", { enumerable: true, get: function () { return debug_1.Debug; } });
//# sourceMappingURL=debug.js.map