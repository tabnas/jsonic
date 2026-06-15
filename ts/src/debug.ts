/* Copyright (c) 2021-2026 Richard Rodger, MIT License */

/*  debug.ts
 *  Debug tools.
 *
 *  The debug plugin has been extracted into the standalone
 *  `@tabnas/debug` package (tracing hooks and a `describe()` method for
 *  engine instances). jsonic re-exports it so the historic
 *  `jsonic/debug` subpath export keeps working.
 */

export { Debug } from '@tabnas/debug'
