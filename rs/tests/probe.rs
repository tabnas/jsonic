use tabnas::{Options, Tabnas};

#[test]
fn can_we_layer_on_the_rust_json_port() {
    let mut p = Tabnas::new();
    tabnas_json::json(&mut p).expect("json installs");

    // json's document sets rule.include="json", which would exclude any
    // alternate jsonic adds; and number.check, which rejects the relaxed
    // number forms jsonic accepts. Both have to come off.
    p.set_options(|o: &mut Options| {
        o.rule.include = String::new();
        o.number.check = None;
        o.text.lex = true;
        o.comment.lex = true;
        o.map.extend = true;
        o.lex.empty = true;
    })
    .expect("options apply");

    for src in [r#"{"a":1}"#, "{a:1}", "a:1", "[1,2,]", "1 // c", "0xff", "{a:1,b:2}"] {
        println!("{src:?} => {:?}", p.parse(src).map(|v| v.to_string()).map_err(|e| e.code));
    }
}
