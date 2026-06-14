module github.com/jsonicjs/jsonic/go

go 1.24.7

require (
	github.com/tabnas/json/go v0.0.0-00010101000000-000000000000
	github.com/tabnas/parser/go v0.0.0-00010101000000-000000000000
)

// jsonic is a grammar plugin for the tabnas engine, and layers on the
// @tabnas/json standard-JSON grammar plugin. Until tabnas/parser and
// tabnas/json publish tagged Go modules, depend on sibling checkouts —
// the same development model the TypeScript package uses for `tabnas` and
// `@tabnas/json` (file:../../parser/ts, file:../../json/ts). Clone
// https://github.com/tabnas/parser.git and https://github.com/tabnas/json.git
// as siblings of this repo.
replace github.com/tabnas/parser/go => ../../parser/go

replace github.com/tabnas/json/go => ../../json/go
