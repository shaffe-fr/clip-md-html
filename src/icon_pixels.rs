/// Pixels RGBA 32x32 : disque bleu avec un « M » blanc (partagé avec build.rs pour l'icône de l'exe).
pub fn rgba() -> Vec<u8> {
    const N: i32 = 32;
    let mut px = vec![0u8; (N * N * 4) as usize];
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        if (0..N).contains(&x) && (0..N).contains(&y) {
            let i = ((y * N + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    };
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            if dx * dx + dy * dy <= 15.5 * 15.5 {
                put(x, y, [37, 99, 235, 255]);
            }
        }
    }
    let white = [255, 255, 255, 255];
    for y in 9..23 {
        for t in 0..3 {
            put(9 + t, y, white);
            put(20 + t, y, white);
        }
    }
    for i in 0..7 {
        for t in 0..3 {
            put(10 + i + t, 9 + i, white);
            put(21 - i + t, 9 + i, white);
        }
    }
    px
}
