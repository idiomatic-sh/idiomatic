fn outer() { fn inner() {} }

fn a() { fn b() { fn c() {} } }

mod m { fn in_mod() {} }

struct S;

impl S { fn method(&self) {} }

trait T { fn required(&self); }
