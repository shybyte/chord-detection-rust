extern crate sdl2;
extern crate chord_detection;
extern crate goertzel;
extern crate pitch_calc;

use std::path::Path;

use sdl2::render::Canvas;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::TextureQuery;
use std::time::Duration;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use chord_detection::calculate_spectrum;
use chord_detection::gromagram::{Gromagram, GromagramInitProps};

use chord_detection::chromagram::{Chromagram, ChromagramInitProps};
use chord_detection::midi_notes;

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use std::sync::mpsc;
use std::i16::MAX as I16_MAX;

use pitch_calc::{
    Hz,
    Letter,
    LetterOctave,
    ScaledPerc,
    Step,
};


struct Recording {
    new_frame_sender: mpsc::Sender<Vec<i16>>,
}

impl AudioCallback for Recording {
    type Channel = i16;

    fn callback(&mut self, input: &mut [i16]) {
        // println!("input.len = {:?}", input.len());
        self.new_frame_sender.send(input.to_vec()).unwrap();
    }
}


pub fn main() {
    sdl2::ttf::get_linked_version();
    let ttf_context = sdl2::ttf::init().unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: None,
        channels: None,
        samples: None,
    };

    let (new_record_frame_sender, new_record_frame_receiver) = mpsc::channel();

    let a = LetterOctave(Letter::A, 1);
    eprintln!("a = {:?}", a.hz());

    let mut capture_freq: u32 = 0;
    let mut channel_count: usize = 1;
    let mut sample_count = 0;
    let capture_device = audio_subsystem.open_capture(None, &desired_spec, |spec| {
        println!("Capture Spec = {:?}", spec);
        capture_freq = spec.freq as u32;
        channel_count = spec.channels as usize;
        sample_count = spec.samples as usize;
        Recording {
            new_frame_sender: new_record_frame_sender,
        }
    }).unwrap();

    let mut input_buffer = vec![0.0; sample_count];

    let mut chromagram = Chromagram::new(ChromagramInitProps {
        sample_rate: capture_freq as usize,
        frame_size: sample_count,
    });

    let mut ggram = Gromagram::new(GromagramInitProps {
        window_size: 1024 * 2,
        sample_rate: capture_freq,
        channel_count: channel_count,
        start_note: midi_notes::A1 as usize,
        notes_count: 24
        }
    );


    println!("AudioDriver: {:?}", capture_device.subsystem().current_audio_driver());
    capture_device.resume();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas: Canvas<_> = window.into_canvas().build().unwrap();


    let font_path = Path::new("./examples/data/Roboto-Regular.ttf");
    let texture_creator = canvas.texture_creator();
    let text_texture = {
        // Load a font
        let font = ttf_context.load_font(font_path, 32).unwrap();
        // font.set_style(sdl2::ttf::STYLE_BOLD);

        // render a surface, and convert it to a texture bound to the canvas
        let surface = font.render("Am")
            .blended(Color::RGBA(255, 0, 0, 255)).unwrap();
        texture_creator.create_texture_from_surface(&surface).unwrap()
    };
    let TextureQuery { width: t_width, height: t_height, .. } = text_texture.query();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut x = 0;
    let mut max_spectrum = 0.0;

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
            eprintln!("audio_frames = {:?}", audio_frames.len());
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

            canvas.set_draw_color(Color::RGB(255, 0, 0));
            let spectrum = calculate_spectrum(audio_chunk);
            let max_spectrum_now: f32 = *spectrum.iter().max_by_key(|&&f| f as i16).unwrap();
            if max_spectrum_now > max_spectrum {
                max_spectrum = max_spectrum_now;
            }
            for (x, &spectrum_val) in spectrum.iter().enumerate() {
                let height: u32 = spectrum_val as u32 * middle_h as u32 / (max_spectrum as u32 + 1);
                canvas.draw_rect(Rect::new(x as i32, middle_h - height as i32, 1, height)).unwrap();
            }


            for frame in &audio_frames {
                ggram.process_audio_frame(frame);
            }

            for (i, a_mag) in ggram.gromagram.iter().enumerate(){
                canvas.set_draw_color(Color::RGB(0, 0, 255));
                let y = (i as i32) * 20;
                canvas.draw_rect(Rect::new(0, y, (a_mag / 5000.0) as u32, 10)).unwrap();
            }

            for frame in &audio_frames {
                for (i, chunk) in frame.chunks(channel_count).enumerate() {
                    let mut s: i64 = chunk.iter().map(|&x| x as i64).sum();
                    input_buffer[i] = s as f64 / channel_count as f64;
                }
                chromagram.process_audio_frame(&input_buffer);
            }

//            if chromagram.is_ready() {
//                canvas.set_draw_color(Color::RGB(0, 0, 255));
//                for i in 0..12 {
//                    let a_mag = chromagram.chromagram[i];
//
//                    let y = i * 30;
//                    canvas.draw_rect(Rect::new(0, y as i32, (a_mag / 5.0) as u32, 20)).unwrap();
//                }
//            }


//            println!("a_mag = {:?}", a_mag);

            canvas.copy(&text_texture, None, Some(Rect::new(0, 0, t_width, t_height))).unwrap();


            x = (x + 1) % 400;
        }

        canvas.present();
    }


    capture_device.pause();
}
