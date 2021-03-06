use std::f64;
use goertzel::Parameters;


#[derive(Clone)]
pub struct GromagramInitProps {
    pub window_size: usize,
    pub sample_rate: u32,
    pub start_note: usize,
    pub notes_count: usize,
}

impl Default for GromagramInitProps {
    fn default() -> Self {
        Self {
            window_size: 1024,
            sample_rate: 44_100,
            start_note: 28,     // e2 82.41 Hz lowest guitar string
            notes_count: 12,    // one octave
        }
    }
}

pub struct Gromagram {
    pub props: GromagramInitProps,
    buffer: Vec<i16>,
    buffer_pos: usize,
    pub gromagram: Vec<f64>,
}

impl Gromagram {
    pub fn new(props: GromagramInitProps) -> Self {
        let chromagram = Self {
            buffer: vec![0; props.window_size],
            gromagram: vec![0.0; props.notes_count as usize],
            buffer_pos: 0,
            props,
        };

        chromagram
    }

    pub fn reset(&mut self) {
        for p in &mut self.buffer {
            *p = 0;
        }
        self.buffer_pos = 0;
    }

    pub fn process_audio_frame(&mut self, frame: &[i16]) {
        for &input in frame {
            self.buffer[self.buffer_pos] = input;
            self.buffer_pos = (self.buffer_pos + 1) % self.buffer.len();
        }

        for i in 0..self.props.notes_count {
            let note = self.props.start_note + i;
            let note_freq = f64::powf(2.0, (note as f64 - 69.0) / 12.0) * 440.0;
            // eprintln!("note = {:?} {:?}", note, note_freq);
            let gp = Parameters::new(note_freq as f32, self.props.sample_rate, self.buffer.len());
            let goertzel_a = gp.start();
            let a_mag = goertzel_a
                .add(&self.buffer[self.buffer_pos..])
                .add(&self.buffer[0..self.buffer_pos])
                .finish_mag();
            self.gromagram[i] = a_mag as f64;
        }
    }

    pub fn normalize(&mut self) {
        let sum: f64 = self.gromagram.iter().sum();
        for x in &mut self.gromagram {
            *x = *x / sum;
        }
    }
}