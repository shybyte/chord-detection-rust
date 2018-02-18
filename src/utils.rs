pub fn make_mono(channel_count: usize, input: &[i16], output: &mut [i16]) {
    let channel_count_i64 = channel_count as i64;
    for (chunk, out) in input.chunks(channel_count).zip(output.iter_mut()) {
        let s: i64 = chunk.iter().map(|&x| x as i64).sum();
        *out = (s / channel_count_i64) as i16;
    }
}