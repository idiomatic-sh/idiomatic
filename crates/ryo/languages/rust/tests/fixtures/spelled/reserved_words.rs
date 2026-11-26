//! The words the language reserves and the contextual words of its builtin forms.

fn reserved() {
    let _ = (abstract, become, box, do, final, override, priv, typeof, unsized, virtual);
    gen { yield 1 };
    do yeet 1;
}

fn at(value: Option<i32>) {
    if let whole @ Some(_) = value {
        let _ = whole;
    }
}

builtin # global_asm("nop");
builtin # naked_asm("nop");

fn forms() {
    let _ = builtin # include_bytes("file");
    let builtin # deref(x) = y;
    builtin # naked_asm("ret");
    let _: builtin # pattern_type(u32 is !null) = 1;
}
