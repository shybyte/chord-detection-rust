extern crate sdl2;

use sdl2::render::Canvas;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::rect::Point;

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use std::sync::mpsc;
use std::i16::MAX as I16_MAX;

struct Recording {
    new_frame_sender: mpsc::Sender<Vec<i16>>,
}

impl AudioCallback for Recording {
    type Channel = i16;

    fn callback(&mut self, input: &mut [i16]) {
        println!("input[0] = {:?}", input[0]);
        self.new_frame_sender.send(input.to_vec()).unwrap();
    }
}


pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: None,
        channels: None,
        samples: None
    };

    let (new_record_frame_sender, new_record_frame_receiver) = mpsc::channel();

    let capture_device = audio_subsystem.open_capture(None, &desired_spec, |spec| {
        println!("Capture Spec = {:?}", spec);
        Recording {
            new_frame_sender: new_record_frame_sender,
        }
    }).unwrap();

    println!("AudioDriver: {:?}", capture_device.subsystem().current_audio_driver());
    capture_device.resume();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 800, 600)
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
            x = (x + 1) % 400;
        }

        canvas.present();
    }


    capture_device.pause();
}
