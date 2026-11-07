fn run(value: crate::Value) { consume(value); }

use crate::service::Handle;
static HANDLE: Handle = Handle::new();

fn shout(value: u8) {
    println!("{}", value);
}
