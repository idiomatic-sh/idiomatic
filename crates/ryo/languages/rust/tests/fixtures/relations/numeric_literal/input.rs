const SIXTEEN: [u8; 3] = [16, 0x10, 0b1_0000];

const TENTH: f32 = 0.1f32;

fn small(x: i64) -> bool {
    matches!(x, -1..=9) && x != 1e3 as i64
}
