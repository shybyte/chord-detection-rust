extern crate sdl2;
extern crate hound;

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use std::time::Duration;

struct Sound {
    data: Vec<i16>,
    pos: usize,
}

impl AudioCallback for Sound {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        for dst in out.iter_mut() {
            *dst = *self.data.get(self.pos).unwrap_or(&0);
            self.pos += 1;
        }
    }
}

fn main() {
    let mut reader = hound::WavReader::open("/home/shybyte/mymusic/endstation-paradies/liebt-uns/liebt-uns-a.wav").unwrap();
    let wav_result: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let wav_data  = wav_result.unwrap();

    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1), // mono
        samples: None      // default
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        eprintln!("spec = {:?}", spec);

        // initialize the audio callback
        Sound {
            data: wav_data,
            pos: 0,
        }
    }).unwrap();

    // Start playback
    device.resume();

    // Play for a second
    std::thread::sleep(Duration::from_millis(10_000));

    // Device is automatically closed when dropped
}