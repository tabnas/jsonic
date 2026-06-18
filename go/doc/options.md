# Options Reference (Go)

Options are passed to `Make()` to configure a parser instance. All fields use
pointer types -- `nil` means "use default".

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{
    Comment: &tabnasjsonic.CommentOptions{Lex: boolp(false)},
    Number:  &tabnasjsonic.NumberOptions{Hex: boolp(false)},
})
```

Option fields use pointer types so that `nil` means "use default". Define
small helpers to create pointer values:

```go
func boolp(b bool) *bool { return &b }
func intp(i int) *int    { return &i }
```

## `Fixed`

Controls fixed structural tokens (`{`, `}`, `[`, `]`, `:`, `,`).

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable fixed token recognition |

## `Space`

Controls whitespace handling.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable space recognition |
| `Chars` | `string` | `" \t"` | Characters treated as space |

## `Line`

Controls line ending handling.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable line recognition |
| `Chars` | `string` | `"\r\n"` | Line ending characters |
| `RowChars` | `string` | `"\n"` | Characters that increment the row counter |
| `Single` | `*bool` | `false` | Separate token per newline |

## `Text`

Controls unquoted text lexing.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable text matching |
| `Modify` | `[]ValModifier` | `nil` | Pipeline of value transformers |

`ValModifier` signature: `func(val any) any`

## `Number`

Controls numeric literal parsing.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable number matching |
| `Hex` | `*bool` | `true` | Support `0x` hexadecimal |
| `Oct` | `*bool` | `true` | Support `0o` octal |
| `Bin` | `*bool` | `true` | Support `0b` binary |
| `Sep` | `string` | `"_"` | Separator character (empty to disable) |
| `Exclude` | `func(string) bool` | `nil` | Return true to reject a number-like string |

## `Comment`

Controls comment handling.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable all comment lexing |
| `Def` | `map[string]*CommentDef` | (see below) | Comment type definitions |

Default definitions:

```go
map[string]*CommentDef{
    "hash":  {Line: true, Start: "#"},
    "slash": {Line: true, Start: "//"},
    "block": {Line: false, Start: "/*", End: "*/"},
}
```

### `CommentDef`

| Field | Type | Description |
|---|---|---|
| `Line` | `bool` | `true` for line comments, `false` for block |
| `Start` | `string` | Start marker |
| `End` | `string` | End marker (block only) |
| `Lex` | `*bool` | Enable this definition (default: true) |
| `EatLine` | `*bool` | Consume trailing newline (default: false) |

## `String`

Controls quoted string parsing.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable string matching |
| `Chars` | `string` | `"'\"\`` | Quote characters |
| `MultiChars` | `string` | `` "`" `` | Multiline quote characters |
| `EscapeChar` | `string` | `"\\"` | Escape character |
| `Escape` | `map[string]string` | (standard) | Escape sequence mappings |
| `AllowUnknown` | `*bool` | `true` | Allow unknown escape sequences |
| `Abandon` | `*bool` | `false` | On error, return nil to let next matcher try |
| `Replace` | `map[rune]string` | `nil` | Character replacements during scanning |

## `Map`

Controls object/map behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `Extend` | `*bool` | `true` | Deep-merge duplicate keys |
| `Merge` | `MapMergeFunc` | `nil` | Custom merge: `func(prev, val any, r *Rule, ctx *Context) any` |
| `Child` | `*bool` | `false` | Parse bare colon as `child$` key |

## `List`

Controls array/list behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `Property` | `*bool` | `true` | Allow key-value pairs in arrays |
| `Pair` | `*bool` | `false` | Push pairs as object elements |
| `Child` | `*bool` | `false` | Parse bare colon as child value |

## `Value`

Controls keyword recognition.

| Field | Type | Default | Description |
|---|---|---|---|
| `Lex` | `*bool` | `true` | Enable value matching |
| `Def` | `map[string]*ValueDef` | (see below) | Keyword definitions |

Default definitions:

```go
map[string]*ValueDef{
    "true":  {Val: true},
    "false": {Val: false},
    "null":  {Val: nil},
}
```

## `Rule`

Controls parser rule behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `Start` | `string` | `"val"` | Starting rule name |
| `Finish` | `*bool` | `true` | Auto-close at EOF |
| `MaxMul` | `*int` | `3` | Rule occurrence multiplier |
| `Include` | `string` | `""` | Comma-separated group tags; keep only matching alternates (applied first) |
| `Exclude` | `string` | `""` | Comma-separated group tags to drop (applied after `Include`) |

## `Lex`

Controls global lexer behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `Empty` | `*bool` | `true` | Allow empty source |
| `EmptyResult` | `any` | `nil` | Value for empty source |

## `Parser`

Custom parser override.

| Field | Type | Description |
|---|---|---|
| `Start` | `func(src string, j *Jsonic, meta map[string]any) (any, error)` | Replace the entire parse |

## `Safe`

Controls security features.

| Field | Type | Default | Description |
|---|---|---|---|
| `Key` | `*bool` | `true` | Block `__proto__` and `constructor` keys |

## `Info`

Go-only. Wraps output values in typed structs that carry extra metadata,
instead of plain Go values. Set the fields on an `*InfoOptions`.

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{Info: &tabnasjsonic.InfoOptions{
    Text: boolp(true),
    List: boolp(true),
    Map:  boolp(true),
}})
```

| Field | Type | Default | Description |
|---|---|---|---|
| `Text` | `*bool` | `false` | Wrap string/text values in `Text{Quote, Str}` (quote char preserved; `""` for unquoted) |
| `List` | `*bool` | `false` | Wrap arrays in `ListRef{Val, Implicit, Child, Meta}` (`Implicit` true when no brackets) |
| `Map` | `*bool` | `false` | Wrap objects in `MapRef{Val, Implicit, Meta}` (`Implicit` true when no braces) |
| `Marker` | `string` | `"__info__"` | Key under which info metadata is stored on wrapped values |

`List` is enabled automatically when `List.Child` is set.

## Other Fields

| Field | Type | Description |
|---|---|---|
| `Ender` | `[]string` | Additional characters that end text tokens |
| `Error` | `map[string]string` | Custom error message templates |
| `Hint` | `map[string]string` | Additional error explanations |
| `ConfigModify` | `map[string]ConfigModifier` | Post-config callbacks |
| `Tag` | `string` | Instance identifier tag |
