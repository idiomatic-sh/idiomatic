mod util;

pub fn ready() {
    util::double(1);
}

const _: fn() = helper::ready;
