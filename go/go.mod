module github.com/jsonicjs/jsonic/go

go 1.24.7

require github.com/tabnas/parser/go v0.0.0-00010101000000-000000000000

// jsonic is a grammar plugin for the tabnas engine. Until tabnas/parser
// publishes a tagged Go module, depend on a sibling checkout — the same
// development model the TypeScript package uses for `tabnas`
// (file:../../parser/ts). Clone https://github.com/tabnas/parser.git as a
// sibling of this repo.
replace github.com/tabnas/parser/go => ../../parser/go
