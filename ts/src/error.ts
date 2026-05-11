/* Copyright (c) 2013-2026 Richard Rodger, MIT License */

/*  error.ts
 *  Parse errors. The implementation now lives in the `tabnas` package;
 *  this module re-exports it, keeping the historic `JsonicError` name
 *  available alongside the engine's `TabnasError`.
 */

export {
  errdesc,
  errinject,
  errsite,
  errmsg,
  trimstk,
  strinject,
  prop,
  TabnasError,
  TabnasError as JsonicError,
} from 'tabnas/error'
