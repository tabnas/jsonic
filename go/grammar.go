package jsonic

import "fmt"

// buildGrammar layers jsonic's relaxed extensions on the standard-JSON core
// that @tabnas/json installed (read from rsm). It returns an error rather
// than panicking if that core is missing or has an unexpected shape (e.g. a
// @tabnas/json version mismatch), so grammar installation never crashes the
// caller.
func buildGrammar(rsm map[string]*RuleSpec, cfg *LexConfig) error {
	// pairval sets key:val on the node (the Go port of the TS `pairval`).
	// The previous value at the key is read straight off the node — the
	// engine's @setval$ builtin no longer threads it through r.U["prev"] —
	// so a repeated key (`a:1,a:2`) or a deep object can merge/extend.
	pairval := func(r *Rule, ctx *Context) {
		key, _ := r.U["key"].(string)
		val := r.Child.Node
		if IsUndefined(val) {
			val = nil
		}
		// Do not set unsafe keys on Arrays (Objects are created without a
		// prototype).
		if cfg.SafeKey && r.U["list"] == true {
			if key == "__proto__" || key == "constructor" {
				return
			}
		}
		// Drop keys that match the info marker to preserve metadata.
		if cfg.InfoMarker != "" && key == cfg.InfoMarker {
			return
		}
		prev := nodeMapGetVal(r.Node, key)
		if prev == nil {
			nodeMapSet(r.Node, key, val)
		} else if cfg.MapMerge != nil {
			nodeMapSet(r.Node, key, cfg.MapMerge(prev, val, r, ctx))
		} else if cfg.MapExtend {
			nodeMapSet(r.Node, key, Deep(prev, val))
		} else {
			nodeMapSet(r.Node, key, val)
		}
	}

	// Named function references for the grammar.
	// These closures capture cfg for runtime configuration access.
	ref := map[FuncRef]any{
		"@finish": AltError(func(r *Rule, ctx *Context) *Token {
			if !cfg.FinishRule {
				return ctx.T0
			}
			return nil
		}),

		"@pairkey": AltAction(func(r *Rule, ctx *Context) {
			_ = ctx
			keyToken := r.O0
			var key string
			if keyToken.Tin == TinST || keyToken.Tin == TinTX {
				key, _ = keyToken.Val.(string)
			} else {
				key = keyToken.Src
			}
			r.U["key"] = key
		}),

		// val rule state actions
		//
		// @val-bc takes ownership of the val close phase (mirrors the TS
		// `@val-bc/replace`): @tabnas/json's strict @val-bc is cleared and
		// this runs as the before-close state action. The matched val close
		// alt's @value$ action (which re-resolves the token) still runs after
		// it, and @val-ac restores a primitive plugin value over that.
		"@val-bc": StateAction(func(r *Rule, ctx *Context) {
			// Stash the value a plugin set in a val OPEN action (before any
			// coalescing). json's @value$ close ALT action still runs after
			// this and re-resolves the matched token, which would overwrite a
			// plugin value; @val-ac below restores it. A normal value rule
			// @reset$s its node in open, so this is Undefined except when a
			// plugin deliberately set it.
			r.U["openval"] = r.Node

			resolveToken := func() any {
				val := r.O0.ResolveVal(r, ctx)
				// A nil value from a non-value token (e.g. #CS, #CB) means
				// "no value", not "null". Keep Undefined to match TS where
				// resolveVal returns undefined for such tokens.
				if val == nil && r.O0.Tin != TinVL {
					return Undefined
				}
				if cfg.TextInfo && (r.O0.Tin == TinST || r.O0.Tin == TinTX) {
					quote := ""
					if r.O0.Tin == TinST && len(r.O0.Src) > 0 {
						quote = string(r.O0.Src[0])
					}
					str, _ := val.(string)
					val = Text{Quote: quote, Str: str}
				}
				return val
			}

			switch {
			// A child map/list node wins (the value was a container).
			case r.Child != NoRule && r.Child != nil && !IsUndefined(r.Child.Node):
				r.Node = r.Child.Node
			// else a deliberate PRIMITIVE value a plugin set in a val open
			// action (a stale parent-seeded node is always a container, so a
			// non-container node here is an intentional scalar value).
			case isPrimitiveNode(r.Node):
				// keep r.Node as-is
			// else the matched scalar token — this beats a stale parent-seeded
			// container node.
			case r.OS != 0:
				r.Node = resolveToken()
			// else a deliberate container a plugin set (no token).
			case !IsUndefined(r.Node):
				// keep r.Node as-is
			// else no value -> undefined (implicit null).
			default:
				r.Node = Undefined
			}
		}),

		// After-close: json's @value$ close alt re-resolves the matched token
		// and so overwrites a value a plugin set in a val open action (e.g.
		// fixed-token / match-custom plugins). Restore the plugin's value, but
		// only a PRIMITIVE one set with no child: a parent-seeded stale node is
		// always a container, so restricting to non-containers restores
		// deliberate scalar plugin values without disturbing normal coalescing.
		"@val-ac": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			ov := r.U["openval"]
			if isPrimitiveNode(ov) &&
				(r.Child == NoRule || r.Child == nil || IsUndefined(r.Child.Node)) {
				r.Node = ov
			}
		}),

		// map rule state actions
		//
		// @map-bo-jsonic allocates the map node in the BO (before-open) phase
		// — earlier than @tabnas/json's @object$ alt action — so a custom BO
		// action can reach the MapRef and write to its Meta bag (the jsonic Go
		// contract; see both_ref_test.go). It also bumps the dmap depth. The
		// implicit flag is set later by @map-bc from the open token.
		"@map-bo-jsonic": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if cfg.MapRef {
				r.Node = MapRef{
					Val:  make(map[string]any),
					Meta: make(map[string]any),
				}
			} else {
				r.Node = make(map[string]any)
			}
			if v, ok := r.N["dmap"]; ok {
				r.N["dmap"] = v + 1
			} else {
				r.N["dmap"] = 1
			}
		}),

		// @map-bc sets the MapRef implicit flag from the open token: an
		// explicit map opened on `{` (r.O0 == #OB) is implicit:false, a
		// brace-less jsonic map (r.O0 == #KEY) is implicit:true.
		"@map-bc": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if cfg.MapRef {
				implicit := !(r.O0 != NoToken && r.O0.Tin == TinOB)
				if mr, ok := r.Node.(MapRef); ok {
					mr.Implicit = implicit
					r.Node = mr
				}
			}
		}),

		// list rule state actions
		//
		// @list-bo-jsonic allocates the list node in the BO (before-open)
		// phase — earlier than @tabnas/json's @array$ alt action — so a custom
		// BO action can reach the ListRef and write to its Meta bag (the jsonic
		// Go contract; see both_ref_test.go). It also bumps the dlist depth and
		// promotes an implicit (bracket-less) list's already-parsed first value
		// into the freshly allocated array. The implicit flag is set later by
		// @list-bc from the open token.
		"@list-bo-jsonic": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if cfg.ListRef {
				r.Node = ListRef{
					Val:  make([]any, 0),
					Meta: make(map[string]any),
				}
			} else {
				r.Node = make([]any, 0)
			}
			if v, ok := r.N["dlist"]; ok {
				r.N["dlist"] = v + 1
			} else {
				r.N["dlist"] = 1
			}
			// Implicit (bracket-less) list: promote the already-parsed first
			// value (held on the prev val rule) into the new array.
			if r.Prev != NoRule && r.Prev != nil {
				if implist, ok := r.Prev.U["implist"]; ok && implist == true {
					prevNode := r.Prev.Node
					if IsUndefined(prevNode) {
						prevNode = nil
					}
					r.Node = nodeListAppend(r.Node, prevNode)
					r.Prev.Node = r.Node
				}
			}
		}),

		"@list-bc": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if cfg.ListRef {
				implicit := !(r.O0 != NoToken && r.O0.Tin == TinOS)
				if lr, ok := r.Node.(ListRef); ok {
					lr.Implicit = implicit
					if c, ok := r.U["child$"]; ok {
						lr.Child = c
					}
					r.Node = lr
				}
			}
		}),

		// pair rule state actions
		//
		// jsonic's pair.close alts (which replace json's) carry no @setval$,
		// so the value is assigned here via pairval (merge/extend-aware,
		// reading the previous value straight off the node).
		"@pair-bc-jsonic": StateAction(func(r *Rule, ctx *Context) {
			if _, ok := r.U["pair"]; ok {
				pairval(r, ctx)
			}
		}),

		"@pair-bc-child": StateAction(func(r *Rule, ctx *Context) {
			if childFlag, ok := r.U["child"]; !ok || childFlag != true {
				return
			}
			val := r.Child.Node
			if IsUndefined(val) {
				val = nil
			}
			prev, hasPrev := nodeMapGet(r.Node, "child$")
			if !hasPrev {
				nodeMapSet(r.Node, "child$", val)
			} else if cfg.MapMerge != nil {
				nodeMapSet(r.Node, "child$", cfg.MapMerge(prev, val, r, ctx))
			} else if cfg.MapExtend {
				nodeMapSet(r.Node, "child$", Deep(prev, val))
			} else {
				nodeMapSet(r.Node, "child$", val)
			}
		}),

		// elem rule state actions
		"@elem-bc-json": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			done, _ := r.U["done"].(bool)
			if !done && !IsUndefined(r.Child.Node) {
				if _, ok := nodeListVal(r.Node); ok {
					r.Node = nodeListAppend(r.Node, r.Child.Node)
					if r.Parent != NoRule && r.Parent != nil {
						r.Parent.Node = r.Node
					}
				}
			}
		}),

		"@elem-bc-pair": StateAction(func(r *Rule, ctx *Context) {
			if pair, ok := r.U["pair"]; !ok || pair != true {
				return
			}
			if cfg.ListPair {
				key, _ := r.U["key"].(string)
				val := r.Child.Node
				if IsUndefined(val) {
					val = nil
				}
				pairObj := map[string]any{key: val}
				if _, ok := nodeListVal(r.Node); ok {
					r.Node = nodeListAppend(r.Node, pairObj)
					if r.Parent != NoRule && r.Parent != nil {
						r.Parent.Node = r.Node
					}
				}
			} else {
				pairval(r, ctx)
			}
		}),

		"@elem-bc-child": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if childFlag, ok := r.U["child"]; !ok || childFlag != true {
				return
			}
			val := r.Child.Node
			if IsUndefined(val) {
				val = nil
			}
			if r.Parent != NoRule && r.Parent != nil {
				prev, hasPrev := r.Parent.U["child$"]
				if !hasPrev {
					r.Parent.U["child$"] = val
				} else if cfg.MapExtend {
					r.Parent.U["child$"] = Deep(prev, val)
				} else {
					r.Parent.U["child$"] = val
				}
			}
		}),

		// Inline actions used in alts
		"@val-close-err": AltError(func(r *Rule, ctx *Context) *Token {
			if r.D == 0 {
				return ctx.T0
			}
			return nil
		}),

		"@implist-cond": AltCond(func(r *Rule, ctx *Context) bool {
			return r.Prev != NoRule && r.Prev != nil && r.Prev.U["implist"] == true
		}),

		"@elem-double-comma": AltAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if _, ok := nodeListVal(r.Node); ok {
				r.Node = nodeListAppend(r.Node, nil)
				if r.Parent != NoRule && r.Parent != nil {
					r.Parent.Node = r.Node
				}
			}
		}),

		"@elem-single-comma": AltAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if _, ok := nodeListVal(r.Node); ok {
				r.Node = nodeListAppend(r.Node, nil)
				if r.Parent != NoRule && r.Parent != nil {
					r.Parent.Node = r.Node
				}
			}
		}),

		"@elem-pair-err": AltError(func(r *Rule, ctx *Context) *Token {
			if cfg.ListProperty || cfg.ListPair {
				return nil
			}
			return ctx.T0
		}),

		"@elem-close-err": AltError(func(r *Rule, ctx *Context) *Token {
			return r.C0
		}),
	}

	// Merge the engine's `$`-builtins (@reset$/@object$/@array$/...) into the
	// local funcref map so jsonic's code-built alts can reference them by
	// name. ResolveGrammarAltStatic only consults the map it is given — the
	// automatic builtin merge happens for a declarative GrammarSpec, not for
	// alts resolved here — so without this, A:"@object$"/"@array$"/"@reset$"
	// would resolve to nil and implicit containers would never be allocated.
	for name, fn := range BuiltinRefs {
		if _, exists := ref[name]; !exists {
			ref[name] = fn
		}
	}

	// Helper to resolve a GrammarAltSpec slice to []*AltSpec.
	resolve := func(gas []*GrammarAltSpec) []*AltSpec {
		alts := make([]*AltSpec, len(gas))
		for i, ga := range gas {
			alts[i] = ResolveGrammarAltStatic(ga, ref)
		}
		return alts
	}

	// The standard-JSON core (val / map / list / pair / elem) was installed
	// by tjson.RegisterJSONGrammar before buildGrammar runs. Read those rules
	// so jsonic reuses @tabnas/json's alternates and base actions and weaves
	// its relaxed extensions around them, rather than re-declaring the JSON
	// grammar here.
	jsonVal := rsm["val"]
	jsonMap := rsm["map"]
	jsonList := rsm["list"]
	jsonElem := rsm["elem"]

	// Guard the shape of the @tabnas/json core we weave around, so a missing
	// rule or unexpected alternate count returns a clear error instead of an
	// index-out-of-range panic.
	for name, rs := range map[string]*RuleSpec{
		"val": jsonVal, "map": jsonMap, "list": jsonList,
		"pair": rsm["pair"], "elem": jsonElem,
	} {
		if rs == nil {
			return fmt.Errorf("jsonic: @tabnas/json did not install the %q rule", name)
		}
	}
	jsonPair := rsm["pair"]
	if len(jsonVal.OpenAlts()) < 3 || len(jsonVal.CloseAlts()) < 2 ||
		len(jsonMap.OpenAlts()) < 2 || len(jsonList.OpenAlts()) < 2 ||
		len(jsonList.CloseAlts()) < 1 || len(jsonElem.OpenAlts()) < 1 {
		return fmt.Errorf("jsonic: unexpected @tabnas/json grammar shape " +
			"(val/map/list/elem alternate counts) — incompatible version?")
	}

	// jsonic mutates @tabnas/json's installed rules in place via the engine's
	// RuleSpec methods, reusing its alternates and base actions and weaving
	// jsonic's relaxed extensions around them.

	// ====== VAL rule ======
	// @tabnas/json has no @val-bo closure any more — its open alts @reset$ the
	// node and its close alts coalesce via the @value$ builtin. jsonic keeps
	// those, but takes ownership of the val close phase: it clears json's
	// strict @val-bc and installs jsonic's fuller before-close coalescer plus
	// the @val-ac after-close hook that restores a primitive plugin value over
	// json's @value$ close alt-action (which re-resolves the token).
	jvo := jsonVal.OpenAlts()
	jvc := jsonVal.CloseAlts()
	jsonVal.ClearActions("bc").AddBC(ref["@val-bc"].(StateAction))
	jsonVal.ClearActions("ac").AddAC(ref["@val-ac"].(StateAction))

	valOpen := []*AltSpec{jvo[0], jvo[1]}
	valOpen = append(valOpen, resolve([]*GrammarAltSpec{
		// Implicit map at top level / pair dive. @reset$ clears the
		// parent-seeded node (mirrors json's #OB/#OS open alts) so val-close
		// coalesces to the pushed map, not the inherited parent container —
		// fixes `{a:b:1}` and dives that would otherwise self-reference.
		{S: "#KEY #CL", C: map[string]any{"d": 0}, P: "map", B: 2, A: "@reset$", G: "pair,jsonic,top"},
		{S: "#KEY #CL", P: "map", B: 2, N: map[string]int{"pk": 1}, A: "@reset$", G: "pair,jsonic"},
	})...)
	// json's #VAL open alt (already carries @reset$).
	valOpen = append(valOpen, jvo[2])
	// Implicit ends `{a:}` / `[a:]`. @reset$ so the empty value resolves to
	// null (via @val-bc) rather than keeping the inherited parent container.
	valOpen = append(valOpen, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: []string{"#CB #CS"}, C: map[string]any{"d": CGt(0)}, B: 1, A: "@reset$", G: "val,imp,null,jsonic"}, ref))
	valOpen = append(valOpen, resolve([]*GrammarAltSpec{
		// Implicit list at top level starting with a comma: `,` -> [null].
		// Pushing the list rule allocates the array in its BO (@list-bo-jsonic)
		// and @list-bc marks it implicit (the open token is #CA, not #OS).
		{S: "#CA", C: map[string]any{"d": 0}, P: "list", B: 1, G: "list,imp,jsonic"},
		// Value is implicitly null when empty before commas.
		{S: "#CA", B: 1, A: "@reset$", G: "list,val,imp,null,jsonic"},
		{S: "#ZZ", G: "jsonic"},
	})...)
	jsonVal.ClearOpen().AddOpen(valOpen...)

	valClose := []*AltSpec{jvc[0]}
	valClose = append(valClose, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: []string{"#CB #CS"}, B: 1, E: "@val-close-err", G: "val,json,close"}, ref))
	valClose = append(valClose, resolve([]*GrammarAltSpec{
		{S: "#CA", C: map[string]any{"n.dlist": CLte(0), "n.dmap": CLte(0)},
			R: "list", U: map[string]any{"implist": true}, G: "list,val,imp,comma,jsonic"},
		{C: map[string]any{"n.dlist": CLte(0), "n.dmap": CLte(0)},
			R: "list", U: map[string]any{"implist": true}, B: 1, G: "list,val,imp,space,jsonic"},
		{S: "#ZZ", G: "jsonic"},
	})...)
	valClose = append(valClose, jvc[1])
	jsonVal.ClearClose().AddClose(valClose...)

	// ====== MAP rule ======
	// jsonic allocates the map node in BO (@map-bo-jsonic), so it re-declares
	// json's #OB open alts WITHOUT json's @object$ alt action (which would
	// re-allocate and discard the BO node and its Meta bag). @map-bc then sets
	// the implicit flag from the open token. @map-bo-jsonic also bumps dmap.
	jsonMap.AddBO(ref["@map-bo-jsonic"].(StateAction))
	jsonMap.AddBC(ref["@map-bc"].(StateAction))

	mapOpen := resolve([]*GrammarAltSpec{
		// Auto-close; fail if rule.finish is false (the BO already allocated
		// the empty object so `{` -> `{}` when finish is allowed).
		{S: "#OB #ZZ", B: 1, E: "@finish", G: "end,jsonic"},
		// json's explicit-brace open alts, re-declared without @object$.
		{S: "#OB #CB", B: 1, N: map[string]int{"pk": 0}, G: "map,json"},
		{S: "#OB", P: "pair", N: map[string]int{"pk": 0}, G: "map,json,pair"},
		// Pair from implicit map (no braces) — the brace-less counterpart of
		// json's #OB entry; the node is allocated in BO and marked implicit by
		// @map-bc (its open token is #KEY, not #OB).
		{S: "#KEY #CL", P: "pair", B: 2, G: "pair,list,val,imp,jsonic"},
	})
	jsonMap.ClearOpen().AddOpen(mapOpen...)

	mapClose := resolve([]*GrammarAltSpec{
		{S: "#CB", C: map[string]any{"n.pk": CLte(0)}, G: "end,json"},
		{S: "#CB", B: 1, G: "path,jsonic"},
	})
	mapClose = append(mapClose, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: []string{"#CA #CS #VAL"}, B: 1, G: "end,path,jsonic"}, ref))
	mapClose = append(mapClose, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: "#ZZ", E: "@finish", G: "end,jsonic"}, ref))
	jsonMap.ClearClose().AddClose(mapClose...)

	// ====== LIST rule ======
	// jsonic allocates the list node in BO (@list-bo-jsonic), so it
	// re-declares json's #OS open alts WITHOUT json's @array$ alt action
	// (which would re-allocate and discard the BO node and its Meta bag).
	// @list-bc sets the implicit flag from the open token and carries the
	// child$ value. @list-bo-jsonic also bumps dlist and promotes an implicit
	// list's first value.
	jsonList.AddBO(ref["@list-bo-jsonic"].(StateAction))
	jsonList.ClearActions("bc").AddBC(ref["@list-bc"].(StateAction))

	listOpen := resolve([]*GrammarAltSpec{
		{C: "@implist-cond", P: "elem"},
		// json's explicit-bracket open alts, re-declared without @array$.
		{S: "#OS #CS", B: 1, G: "list,json"},
		{S: "#OS", P: "elem", G: "list,elem,json"},
		// Initial comma [, will insert null as [null,
		{S: "#CA", P: "elem", B: 1, G: "list,elem,val,imp,jsonic"},
		// Another element.
		{P: "elem", G: "list,elem,jsonic"},
	})
	jsonList.ClearOpen().AddOpen(listOpen...)

	jsonList.ClearClose().AddClose(resolve([]*GrammarAltSpec{
		{S: "#CS", G: "end,json"},
		{S: "#ZZ", E: "@finish", G: "end,jsonic"},
	})...)

	// ====== PAIR rule ======
	// jsonic supplies its own pair rule: the key alt binds jsonic's @pairkey
	// (number/keyword keys use the token source) over the KEY token set, and
	// the close action is safe-key/merge-aware — replacing json's strict pair.
	jsonPair.ClearActions("bc").
		AddBC(ref["@pair-bc-jsonic"].(StateAction)).
		AddBC(ref["@pair-bc-child"].(StateAction))

	pairOpen := resolve([]*GrammarAltSpec{
		{S: "#KEY #CL", P: "val", U: map[string]any{"pair": true}, A: "@pairkey", G: "map,pair,key,json"},
		{S: "#CA", G: "map,pair,comma,jsonic"},
	})
	if cfg.MapChild {
		pairOpen = append(pairOpen, ResolveGrammarAltStatic(
			&GrammarAltSpec{S: "#CL", P: "val",
				U: map[string]any{"done": true, "child": true}}, ref))
	}
	jsonPair.ClearOpen().AddOpen(pairOpen...)

	pairClose := resolve([]*GrammarAltSpec{
		{S: "#CB", C: map[string]any{"n.pk": CLte(0)}, B: 1, G: "map,pair,json"},
		{S: "#CA #CB", C: map[string]any{"n.pk": CLte(0)}, B: 1, G: "map,pair,comma,jsonic"},
		{S: "#CA #ZZ", G: "end,jsonic"},
		{S: "#CA", C: map[string]any{"n.pk": CLte(0)}, R: "pair", G: "map,pair,json"},
		{S: "#CA", C: map[string]any{"n.dmap": CLte(1)}, R: "pair", G: "map,pair,jsonic"},
		{S: "#KEY", C: map[string]any{"n.dmap": CLte(1)}, R: "pair", B: 1, G: "map,pair,imp,jsonic"},
	})
	pairClose = append(pairClose, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: []string{"#CB #CA #CS #KEY"}, C: map[string]any{"n.pk": CGt(0)},
			B: 1, G: "map,pair,imp,path,jsonic"}, ref))
	pairClose = append(pairClose, resolve([]*GrammarAltSpec{
		{S: "#CS", E: "@elem-close-err", G: "end,jsonic"},
		{S: "#ZZ", E: "@finish", G: "map,pair,json"},
		{R: "pair", B: 1, G: "map,pair,imp,jsonic"},
	})...)
	jsonPair.ClearClose().AddClose(pairClose...)

	// ====== ELEM rule ======
	// Replace json's @elem-bc with jsonic's (done-guarded push + pair/child);
	// reuse json's plain value alt as the final open alternate.
	jeo := jsonElem.OpenAlts()
	jsonElem.ClearActions("bc").
		AddBC(ref["@elem-bc-json"].(StateAction)).
		AddBC(ref["@elem-bc-pair"].(StateAction)).
		AddBC(ref["@elem-bc-child"].(StateAction))

	elemOpen := resolve([]*GrammarAltSpec{
		{S: "#CA #CA", B: 2, U: map[string]any{"done": true}, A: "@elem-double-comma",
			G: "list,elem,imp,null,jsonic"},
		{S: "#CA", U: map[string]any{"done": true}, A: "@elem-single-comma",
			G: "list,elem,imp,null,jsonic"},
		{S: "#KEY #CL", P: "val",
			N: map[string]int{"pk": 1, "dmap": 1},
			U: map[string]any{"done": true, "pair": true, "list": true},
			A: "@pairkey", E: "@elem-pair-err", G: "elem,pair,jsonic"},
	})
	if cfg.ListChild {
		elemOpen = append(elemOpen, ResolveGrammarAltStatic(
			&GrammarAltSpec{S: "#CL", P: "val",
				U: map[string]any{"done": true, "child": true, "list": true},
				G: "elem,child,jsonic"}, ref))
	}
	elemOpen = append(elemOpen, jeo[0])
	jsonElem.ClearOpen().AddOpen(elemOpen...)

	elemClose := []*AltSpec{
		ResolveGrammarAltStatic(&GrammarAltSpec{S: []string{"#CA", "#CS #ZZ"}, B: 1, G: "list,elem,comma,jsonic"}, ref),
	}
	elemClose = append(elemClose, resolve([]*GrammarAltSpec{
		{S: "#CA", R: "elem", G: "list,elem,json"},
		{S: "#CS", B: 1, G: "list,elem,json"},
		{S: "#ZZ", E: "@finish", G: "list,elem,json"},
		{S: "#CB", E: "@elem-close-err", G: "end,jsonic"},
		{R: "elem", B: 1, G: "list,elem,imp,jsonic"},
	})...)
	jsonElem.ClearClose().AddClose(elemClose...)

	return nil
}

// nodeListAppend appends a value to a list node (plain []any or ListRef).
func nodeListAppend(node any, val any) any {
	if lr, ok := node.(ListRef); ok {
		lr.Val = append(lr.Val, val)
		return lr
	}
	if arr, ok := node.([]any); ok {
		return append(arr, val)
	}
	return node
}

// nodeListVal extracts the []any from a list node.
func nodeListVal(node any) ([]any, bool) {
	if lr, ok := node.(ListRef); ok {
		return lr.Val, true
	}
	if arr, ok := node.([]any); ok {
		return arr, true
	}
	return nil, false
}

// isPrimitiveNode reports whether node is a concrete scalar value — not the
// no-value sentinel, not nil, and not a container (map/MapRef/[]any/ListRef).
// A parent-seeded stale node is always a container, so a primitive node in a
// val rule is one a plugin deliberately set (an intentional scalar value).
func isPrimitiveNode(node any) bool {
	if node == nil || IsUndefined(node) {
		return false
	}
	switch node.(type) {
	case map[string]any, MapRef, []any, ListRef:
		return false
	}
	return true
}

// nodeMapSet sets a key on a map node.
func nodeMapSet(node any, key any, val any) {
	k, _ := key.(string)
	if m, ok := node.(map[string]any); ok {
		m[k] = val
	} else if mr, ok := node.(MapRef); ok {
		mr.Val[k] = val
	}
}

// nodeMapGet gets a value from a map node.
func nodeMapGet(node any, key any) (any, bool) {
	k, _ := key.(string)
	if m, ok := node.(map[string]any); ok {
		v, exists := m[k]
		return v, exists
	}
	if mr, ok := node.(MapRef); ok {
		v, exists := mr.Val[k]
		return v, exists
	}
	return nil, false
}

// nodeMapGetVal returns the value or nil.
func nodeMapGetVal(node any, key any) any {
	v, _ := nodeMapGet(node, key)
	return v
}
