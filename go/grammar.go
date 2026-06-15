package jsonic

import "fmt"

// buildGrammar layers jsonic's relaxed extensions on the standard-JSON core
// that @tabnas/json installed (read from rsm). It returns an error rather
// than panicking if that core is missing or has an unexpected shape (e.g. a
// @tabnas/json version mismatch), so grammar installation never crashes the
// caller.
func buildGrammar(rsm map[string]*RuleSpec, cfg *LexConfig) error {
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

		"@pairval": AltAction(func(r *Rule, ctx *Context) {
			key, _ := r.U["key"].(string)
			val := r.Child.Node
			if IsUndefined(val) {
				val = nil
			}
			if cfg.SafeKey && r.U["list"] == true {
				if key == "__proto__" || key == "constructor" {
					return
				}
			}
			// Drop keys that match the info marker to preserve metadata.
			if cfg.InfoMarker != "" && key == cfg.InfoMarker {
				return
			}
			prev := r.U["prev"]
			if prev == nil {
				nodeMapSet(r.Node, key, val)
			} else if cfg.MapMerge != nil {
				nodeMapSet(r.Node, key, cfg.MapMerge(prev, val, r, ctx))
			} else if cfg.MapExtend {
				nodeMapSet(r.Node, key, Deep(prev, val))
			} else {
				nodeMapSet(r.Node, key, val)
			}
		}),

		// val rule state actions
		"@val-bo": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			r.Node = Undefined
		}),

		"@val-bc": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if IsUndefined(r.Node) {
				if IsUndefined(r.Child.Node) {
					if r.OS == 0 {
						r.Node = Undefined
					} else {
						val := r.O0.ResolveVal(r, ctx)
						// A nil value from a non-value token (e.g. #CS, #CB)
						// means "no value", not "null". Keep Undefined to match
						// TS where resolveVal returns undefined for such tokens.
						if val == nil && r.O0.Tin != TinVL {
							r.Node = Undefined
						} else {
							if cfg.TextInfo && (r.O0.Tin == TinST || r.O0.Tin == TinTX) {
								quote := ""
								if r.O0.Tin == TinST && len(r.O0.Src) > 0 {
									quote = string(r.O0.Src[0])
								}
								str, _ := val.(string)
								val = Text{Quote: quote, Str: str}
							}
							r.Node = val
						}
					}
				} else {
					r.Node = r.Child.Node
				}
			}
		}),

		// map rule state actions
		"@map-bo": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if cfg.MapRef {
				r.Node = MapRef{
					Val:  make(map[string]any),
					Meta: make(map[string]any),
				}
			} else {
				r.Node = make(map[string]any)
			}
		}),

		"@map-bo-jsonic": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if v, ok := r.N["dmap"]; ok {
				r.N["dmap"] = v + 1
			} else {
				r.N["dmap"] = 1
			}
		}),

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
		"@list-bo": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if cfg.ListRef {
				r.Node = ListRef{
					Val:  make([]any, 0),
					Meta: make(map[string]any),
				}
			} else {
				r.Node = make([]any, 0)
			}
		}),

		"@list-bo-jsonic": StateAction(func(r *Rule, ctx *Context) {
			_ = ctx
			if v, ok := r.N["dlist"]; ok {
				r.N["dlist"] = v + 1
			} else {
				r.N["dlist"] = 1
			}
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
		"@pair-bc-json": StateAction(func(r *Rule, ctx *Context) {
			if _, ok := r.U["pair"]; ok {
				key, _ := r.U["key"].(string)
				if cfg.SafeKey && r.U["list"] == true && (key == "__proto__" || key == "constructor") {
					return
				}
				// Drop keys that match the info marker to preserve metadata.
				if cfg.InfoMarker != "" && key == cfg.InfoMarker {
					return
				}
				r.U["prev"] = nodeMapGetVal(r.Node, r.U["key"])
				nodeMapSet(r.Node, key, r.Child.Node)
			}
		}),

		"@pair-bc-jsonic": StateAction(func(r *Rule, ctx *Context) {
			if _, ok := r.U["pair"]; ok {
				key, _ := r.U["key"].(string)
				val := r.Child.Node
				if IsUndefined(val) {
					val = nil
				}
				if cfg.SafeKey && r.U["list"] == true {
					if key == "__proto__" || key == "constructor" {
						return
					}
				}
				// Drop keys that match the info marker to preserve metadata.
				if cfg.InfoMarker != "" && key == cfg.InfoMarker {
					return
				}
				prev := r.U["prev"]
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
				r.U["prev"] = nodeMapGetVal(r.Node, r.U["key"])
				key, _ := r.U["key"].(string)
				val := r.Child.Node
				if IsUndefined(val) {
					val = nil
				}
				if cfg.SafeKey && r.U["list"] == true {
					if key == "__proto__" || key == "constructor" {
						return
					}
				}
				// Drop keys that match the info marker to preserve metadata.
				if cfg.InfoMarker != "" && key == cfg.InfoMarker {
					return
				}
				prev := r.U["prev"]
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
	// Keep @tabnas/json's @val-bo (node = Undefined); replace its strict
	// @val-bc with jsonic's fuller one (preserves plugin-set nodes and
	// implicit-null empty values).
	jvo := jsonVal.OpenAlts()
	jvc := jsonVal.CloseAlts()
	jsonVal.ClearActions("bc").AddBC(ref["@val-bc"].(StateAction))

	valOpen := []*AltSpec{jvo[0], jvo[1]}
	valOpen = append(valOpen, resolve([]*GrammarAltSpec{
		{S: "#KEY #CL", C: map[string]any{"d": 0}, P: "map", B: 2, G: "pair,jsonic,top"},
		{S: "#KEY #CL", P: "map", B: 2, N: map[string]int{"pk": 1}, G: "pair,jsonic"},
	})...)
	valOpen = append(valOpen, jvo[2])
	valOpen = append(valOpen, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: []string{"#CB #CS"}, C: map[string]any{"d": CGt(0)}, B: 1, G: "val,imp,null,jsonic"}, ref))
	valOpen = append(valOpen, resolve([]*GrammarAltSpec{
		{S: "#CA", C: map[string]any{"d": 0}, P: "list", B: 1, G: "list,imp,jsonic"},
		{S: "#CA", B: 1, G: "list,val,imp,null,jsonic"},
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
	// Keep json's @map-bo (create node) and @map-bc (MapRef implicit); append
	// jsonic's depth counter to bo.
	jmo := jsonMap.OpenAlts()
	jsonMap.AddBO(ref["@map-bo-jsonic"].(StateAction))

	mapOpen := []*AltSpec{
		ResolveGrammarAltStatic(&GrammarAltSpec{S: "#OB #ZZ", B: 1, E: "@finish", G: "end,jsonic"}, ref),
	}
	mapOpen = append(mapOpen, jmo[0], jmo[1])
	mapOpen = append(mapOpen, ResolveGrammarAltStatic(
		&GrammarAltSpec{S: "#KEY #CL", P: "pair", B: 2, G: "pair,list,val,imp,jsonic"}, ref))
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
	// Keep json's @list-bo (create node); append jsonic's depth/implist
	// handling. Replace @list-bc with jsonic's (adds child$ handling).
	jlo := jsonList.OpenAlts()
	jlc := jsonList.CloseAlts()
	jsonList.AddBO(ref["@list-bo-jsonic"].(StateAction))
	jsonList.ClearActions("bc").AddBC(ref["@list-bc"].(StateAction))

	listOpen := []*AltSpec{
		ResolveGrammarAltStatic(&GrammarAltSpec{C: "@implist-cond", P: "elem"}, ref),
	}
	listOpen = append(listOpen, jlo[0], jlo[1])
	listOpen = append(listOpen, resolve([]*GrammarAltSpec{
		{S: "#CA", P: "elem", B: 1, G: "list,elem,val,imp,jsonic"},
		{P: "elem", G: "list,elem,jsonic"},
	})...)
	jsonList.ClearOpen().AddOpen(listOpen...)

	jsonList.ClearClose().AddClose(
		jlc[0],
		ResolveGrammarAltStatic(&GrammarAltSpec{S: "#ZZ", E: "@finish", G: "end,jsonic"}, ref),
	)

	// ====== PAIR rule ======
	// jsonic supplies its own pair rule: the key alt binds jsonic's @pairkey
	// (number/keyword keys use the token source) over the KEY token set, and
	// the close action is safe-key/merge-aware — replacing json's strict pair.
	jsonPair.ClearActions("bc").
		AddBC(ref["@pair-bc-json"].(StateAction)).
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

// nodeListSetVal updates the []any inside a list node.
func nodeListSetVal(node any, arr []any) any {
	if lr, ok := node.(ListRef); ok {
		lr.Val = arr
		return lr
	}
	return arr
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
