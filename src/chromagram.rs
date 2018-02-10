//=======================================================================
/** @file chromagram.rs
 *  @brief Chromagram - a class for calculating the chromagram in real-time
 *  @rust-implementation Marco Stahl
 *  @copyright Copyright (C) 2018 Marco Stahl

 *  Based on https://github.com/adamstark/Chord-Detector-and-Chromagram
 *  @original-file Chromagram.cpp
 *  @author Adam Stark
 *  @copyright Copyright (C) 2008-2014  Queen Mary University of London
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
//=======================================================================

use std::f64;
use std::f64::consts::PI;

use rustfft::FFTplanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

//const C3: f64 = 130.81278265 / 2.0;
//const C3: f64 = 55.0;
const C3: f64 = 196.0/4.0;
const BUFFER_SIZE: usize = 1024 * 4;
const NUM_HARMONICS: usize = 2;
const NUM_OCTAVES: usize = 2;
const NUM_NOTES_IN_OCTAVE: usize = 12;
const NUM_BINS_TO_SEARCH: isize = 2;
const DOWN_SAMPLING_FACTOR: usize = 1;

lazy_static! {
    static ref NOTE_FREQUENCIES: Vec<f64> = (0..NUM_NOTES_IN_OCTAVE)
        .map(|i| C3 * f64::powf (2.0, ((i as f64) / NUM_NOTES_IN_OCTAVE as f64)))
        .collect();
    static ref HAMMING_WINDOW: Vec<f64> = (0..BUFFER_SIZE)
        .map(|i| 0.54 - 0.46 * f64::cos (2.0 * PI * ((i as f64) / ( BUFFER_SIZE as f64))))
        .collect();
}

pub struct Chromagram {
    props: ChromagramInitProps,
    buffer: Vec<f64>,
    pub chromagram: Vec<f64>,
    magnitude_spectrum: Vec<f64>,
    filtered_frame: Vec<f64>,
    fft_in: Vec<Complex<f64>>,
    fft_out: Vec<Complex<f64>>,
    downsampled_input_audio_frame: Vec<f64>,
    num_samples_since_last_calculation: usize,
    // In samples at the input audio sampling frequency
    chroma_calculation_interval: usize,
    chroma_ready: bool,
}

pub struct ChromagramInitProps {
    pub frame_size: usize,
    pub sample_rate: usize,
}

impl Default for ChromagramInitProps {
    fn default() -> Self {
        Self { frame_size: 256, sample_rate: 44_100 }
    }
}

impl Chromagram {
    pub fn new(props: ChromagramInitProps) -> Self {
        let chromagram = Self {
            buffer: vec![0.0; BUFFER_SIZE],
            chromagram: vec![0.0; NUM_NOTES_IN_OCTAVE],
            magnitude_spectrum: vec![0.0; BUFFER_SIZE / 2 + 1],
            filtered_frame: vec![0.0; props.frame_size],
            fft_in: vec![Complex::zero(); BUFFER_SIZE],
            fft_out: vec![Complex::zero(); BUFFER_SIZE],
            downsampled_input_audio_frame: vec![0.0; props.frame_size / DOWN_SAMPLING_FACTOR],
            num_samples_since_last_calculation: 0,
            chroma_calculation_interval: 4096,
            chroma_ready: false,
            props,
        };

        // TODO: setup fft

        chromagram
    }

    pub fn process_audio_frame(&mut self, input_audio_frame: &[f64]) {
        self.chroma_ready = true;
        self.down_sample_frame(input_audio_frame);

        // move samples back
        for i in 0..(BUFFER_SIZE - self.downsampled_input_audio_frame.len()) {
            self.buffer[i] = self.buffer[i + self.downsampled_input_audio_frame.len()];
        }

        let mut n = 0;

        // move samples back
        for i in (BUFFER_SIZE - self.downsampled_input_audio_frame.len())..BUFFER_SIZE {
            self.buffer[i] = self.downsampled_input_audio_frame[n];
            n += 1;
        }

        self.num_samples_since_last_calculation += self.props.frame_size;

        if self.num_samples_since_last_calculation >= self.chroma_calculation_interval {
            self.calculate_chromagram();
            self.num_samples_since_last_calculation = 0;
        }
    }

    pub fn is_ready(&self) -> bool {
        self.chroma_ready
    }

    fn calculate_chromagram(&mut self) {
        self.calculate_magnitude_spectrum();

        let divisor_ratio = self.props.sample_rate as f64 / DOWN_SAMPLING_FACTOR as f64 / BUFFER_SIZE as f64;

        for n in 0..NUM_NOTES_IN_OCTAVE {
            let mut chroma_sum = 0.0;
            for octave in 1..(NUM_OCTAVES + 1) {
                let mut note_sum = 0.0;
                for harmonic in 1..(NUM_HARMONICS + 1) {
                    let center_bin: isize = (NOTE_FREQUENCIES[n] * octave as f64 * harmonic as f64 / divisor_ratio).round() as isize;
                    let min_bin = center_bin - (NUM_BINS_TO_SEARCH * harmonic as isize);
                    let max_bin = center_bin + (NUM_BINS_TO_SEARCH * harmonic as isize);

                    let mut max_val = 0.0;

                    for k in min_bin..max_bin {
                        if self.magnitude_spectrum[k as usize] > max_val {
                            max_val = self.magnitude_spectrum[k as usize];
                        }
                    }

                    note_sum += max_val / harmonic as f64;
                }

                chroma_sum += note_sum;
            }
            self.chromagram[n] = chroma_sum;
        }

        self.chroma_ready = true;
    }

    fn calculate_magnitude_spectrum(&mut self) {
        for i in 0..BUFFER_SIZE {
            self.fft_in[i] = Complex::new(self.buffer[i] * HAMMING_WINDOW[i], 0.0);
        }

        // TODO: init in constructor
        let mut planner = FFTplanner::new(false);
        let fft = planner.plan_fft(BUFFER_SIZE);
        fft.process(&mut self.fft_in, &mut self.fft_out);

        for i in 0..self.magnitude_spectrum.len() {
            self.magnitude_spectrum[i] = self.fft_out[i].norm_sqr().sqrt().sqrt();
        }
    }

    fn down_sample_frame(&mut self, input_audio_frame: &[f64]) {
        let b0 = 0.2929;
        let b1 = 0.5858;
        let b2 = 0.2929;
        let a1 = -0.0000;
        let a2 = 0.1716;
        let mut x_1 = 0.0;
        let mut x_2 = 0.0;
        let mut y_1 = 0.0;
        let mut y_2 = 0.0;

        for i in 0..self.props.frame_size {
            self.filtered_frame[i] = self.filtered_frame[i] * b0 + x_1 * b1 + x_2 * b2 - y_1 * a1 - y_2 * a2;

            x_2 = x_1;
            x_1 = input_audio_frame[i];
            y_2 = y_1;
            y_1 = self.filtered_frame[i];
        }

        for i in 0..self.downsampled_input_audio_frame.len() {
            self.downsampled_input_audio_frame[i] = self.filtered_frame[i * DOWN_SAMPLING_FACTOR];
        }
    }
}