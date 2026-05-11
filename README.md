# jsonic

A dynamic JSON parser that isn't strict and can be customized.

```
a:1,foo:bar  →  {"a": 1, "foo": "bar"}
```

This repository contains:

| Path | Description |
|---|---|
| [`ts/`](ts/) | TypeScript / JavaScript implementation (`jsonic` on npm). Built on the [`tabnas`](https://github.com/tabnas/parser) parsing engine. |
| [`go/`](go/) | Go port. |

Start with [`ts/README.md`](ts/README.md) for the JS API or
[`go/README.md`](go/README.md) for Go.

## License

MIT. Copyright (c) Richard Rodger.
