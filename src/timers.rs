use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

const FREQUENCY: u32 = 60;

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
        loop {
            {
                let mut value = self.value.lock().unwrap();
                if *value != 0 {
                    *value -= 1;
                }
            }
            std::thread::sleep(delay_count);
        }
    }
}
