extern crate sdl2;
extern crate chord_detection;
extern crate goertzel;
extern crate pitch_calc;
extern crate hound;


use sdl2::render::Canvas;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use chord_detection::gromagram::{Gromagram, GromagramInitProps};

use chord_detection::midi_notes;

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use std::sync::mpsc;
use std::i16::MAX as I16_MAX;
use chord_detection::utils::make_mono;
use chord_detection::chord_detection::ChordDetector;


struct PseudoRecording {
    data: Vec<i16>,
    pos: usize,
    new_frame_sender: mpsc::Sender<Vec<i16>>,
}

impl AudioCallback for PseudoRecording {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        for dst in out.iter_mut() {
            *dst = *self.data.get(self.pos).unwrap_or(&0);
            self.pos = (self.pos + 1) % self.data.len();
        }
        self.new_frame_sender.send(out.to_vec()).unwrap();
    }
}

fn read_wav(filename: &str) -> Vec<i16> {
    let complete_filename = "/home/shybyte/mymusic/endstation-paradies/liebt-uns/".to_string();
    let mut reader = hound::WavReader::open(complete_filename + filename).unwrap();
    let wav_result: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    wav_result.unwrap()
}

pub fn main() {
    let wav_data = read_wav("liebt-uns.wav");
    let training_input = vec![read_wav("liebt-uns-a.wav"), read_wav("liebt-uns-e.wav")];
    let training_labels = vec!["a", "e"];

    sdl2::ttf::get_linked_version();
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1), // mono
        samples: None,      // default
    };

    let (new_record_frame_sender, new_record_frame_receiver) = mpsc::channel();

    let mut capture_freq: u32 = 0;
    let mut channel_count: usize = 1;
    let mut sample_count = 0;
    let capture_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        println!("Play Spec = {:?}", spec);
        capture_freq = spec.freq as u32;
        channel_count = spec.channels as usize;
        sample_count = spec.samples as usize;
        PseudoRecording {
            data: wav_data,
            pos: 0,
            new_frame_sender: new_record_frame_sender,
        }
    }).unwrap();

    let mut mono_buffer = vec![0; sample_count];
    let gromagram_init_props = GromagramInitProps {
        window_size: 1024 * 3,
        sample_rate: capture_freq,
        start_note: midi_notes::A1 as usize,
        notes_count: 24,
    };

    let mut ggram = Gromagram::new(gromagram_init_props.clone());

    let mut chord_detector = ChordDetector::new(Gromagram::new(gromagram_init_props), &training_labels);

    for (input, label) in training_input.iter().zip(training_labels.iter()) {
        chord_detector.train(input, label);
    }
    chord_detector.finish_training();

    println!("AudioDriver: {:?}", capture_device.subsystem().current_audio_driver());
    capture_device.resume();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 800, 800)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas: Canvas<_> = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut x = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                }
                _ => {}
            }
        }
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));

        let audio_frames: Vec<Vec<i16>> = new_record_frame_receiver.try_iter().collect();

        if !audio_frames.is_empty() {
            // eprintln!("audio_frames = {:?}", audio_frames.len());

            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();

            let (_w, h) = canvas.output_size().unwrap();
            let middle_h = (h / 2) as i32;
            let draw_color = Color::RGB(255, 255, 0);
            canvas.set_draw_color(draw_color);
            let audio_chunk = &audio_frames[0];
            for (x, &audio_val) in audio_chunk.iter().enumerate() {
                canvas.draw_point(Point::new(x as i32, middle_h + audio_val as i32 * middle_h / I16_MAX as i32)).unwrap();
            }

            for frame in &audio_frames {
                make_mono(channel_count, frame, &mut mono_buffer[..]);
                ggram.process_audio_frame(&mono_buffer);
            }

            let bar_height: u32 = 10;

            for (i, a_mag) in ggram.gromagram.iter().enumerate() {
                canvas.set_draw_color(Color::RGB(0, 0, 255));
                let y = (i as u32) * bar_height;
                canvas.draw_rect(Rect::new(0, y as i32, (a_mag / 5000.0) as u32, bar_height)).unwrap();
            }

            ggram.normalize();
            let start_y = ggram.gromagram.len() as u32 * bar_height + 300;

            for (i, a_mag) in ggram.gromagram.iter().enumerate() {
                canvas.set_draw_color(Color::RGB(255, 0, 0));
                let y = (i as u32) * bar_height + start_y;
                canvas.draw_rect(Rect::new(0, y as i32, (a_mag * 2000.0) as u32, bar_height)).unwrap();
            }

            let chord = chord_detector.detect(&ggram.gromagram);
            eprintln!("chord = {:?}", chord);

//            canvas.copy(&text_texture, None, Some(Rect::new(0, 0, t_width, t_height))).unwrap();


            x = (x + 1) % 400;
        }

        canvas.present();
    }


    capture_device.pause();
}
