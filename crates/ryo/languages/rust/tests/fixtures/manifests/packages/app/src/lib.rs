mod util;

pub fn ready() {
    util::double(1);
}

const _: fn() = helper::ready;
const _: fn() = support::ready;
const _: fn() = tool::ready;

mod shadow {
    use crate::util as helper;
    const _: fn(u8) -> u8 = helper::double;
}
