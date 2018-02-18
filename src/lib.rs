extern crate rustfft;
#[macro_use]
extern crate lazy_static;
extern crate goertzel;
extern crate rusty_machine;

pub mod chromagram;
pub mod gromagram;
pub mod chord_detection;
pub mod midi_notes;
pub mod utils;

use rustfft::FFTplanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use std::i16::MAX as I16_MAX;

pub fn calculate_spectrum(samples: &[i16]) -> Vec<f32> {
    let mut input: Vec<Complex<f32>> = samples.iter().map(|&x| Complex::new(x as f32 / I16_MAX as f32, 0.0)).collect();
//    println!("input = {:?}", input);
    let mut output: Vec<Complex<f32>> = vec![Complex::zero(); input.len()];

    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(input.len());
    fft.process(&mut input, &mut output);

//    println!("output = {:?}", output);
    output.iter().map(|&c| c.norm_sqr()).collect()
}


#[cfg(test)]
mod tests {
    use std::f32::consts::PI;
    use calculate_spectrum;
    use std::i16::MAX as I16_MAX;

    #[test]
    fn sin() {
        let length = 1024;
        let freq = 2.0;
        let sin_vec: Vec<i16> = (0..length).map(|i| (((i as f32 * freq * 4.0 * PI / length as f32).sin() * 0.5 * (I16_MAX as f32)) as i16)).collect();
//        println!("sin_vec = {:?}", sin_vec);
        let spectrum = calculate_spectrum(sin_vec.as_slice());
        println!("spectrum = {:?}", spectrum);
    }

    #[test]
    fn it_works() {
//        let spectrum = calculate_spectrum(vec![10, 10].as_slice());
//        println!("spectrum = {:?}", spectrum);
    }
}
