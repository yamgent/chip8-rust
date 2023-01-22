use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use rodio::{source::SineWave, OutputStream, Sink};

const FREQUENCY: u32 = 60;
const SINE_WAVE_HZ: f32 = 250f32;

pub struct Timer {
    value: Arc<Mutex<u8>>,
    sound: bool,
}

impl Timer {
    pub fn new(sound: bool) -> Self {
        Self {
            value: Arc::new(Mutex::new(0)),
            sound,
        }
    }

    pub fn get_value_arc(&self) -> Arc<Mutex<u8>> {
        self.value.clone()
    }

    pub fn run(&self) {
        let delay_count = Duration::from_secs_f32(1f32 / FREQUENCY as f32);

        // NOTE: We must keep both values in the tuple returned by try_default().
        // Dropping the first value will cause the second value to be invalid, as
        // it will cause the sound device to be dropped.
        let sound = if self.sound {
            let sound_stream =
                OutputStream::try_default().expect("Cannot create sound output stream");
            let sink = Sink::try_new(&sound_stream.1).expect("Cannot create sound sink");
            sink.append(SineWave::new(SINE_WAVE_HZ));
            Some((sound_stream, sink))
        } else {
            None
        };

        loop {
            let current_value;
            {
                let mut value = self.value.lock().unwrap();
                if *value != 0 {
                    *value -= 1;
                }
                current_value = *value;
            }

            if let Some((_, sound_sink)) = &sound {
                if current_value == 0 && !sound_sink.is_paused() {
                    sound_sink.pause();
                }
                if current_value != 0 && sound_sink.is_paused() {
                    sound_sink.play();
                }
            }

            std::thread::sleep(delay_count);
        }
    }
}
