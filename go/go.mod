module github.com/tabnas/jsonic/go

go 1.24.7

require (
	github.com/tabnas/debug/go v0.0.0-00010101000000-000000000000
	github.com/tabnas/json/go v0.0.0-00010101000000-000000000000
	github.com/tabnas/parser/go v0.0.0
)

// jsonic is a grammar plugin for the tabnas engine; it layers on the
// @tabnas/json standard-JSON grammar plugin and re-exports the
// @tabnas/debug plugin. Until these publish tagged Go modules, depend on
// sibling checkouts — the same development model the TypeScript package
// uses (file:../../parser/ts, file:../../json/ts, file:../../debug/ts).
// Clone tabnas/parser, tabnas/json and tabnas/debug as siblings of this
// repo.
replace github.com/tabnas/parser/go => ../../parser/go

replace github.com/tabnas/json/go => ../../json/go

replace github.com/tabnas/debug/go => ../../debug/go
