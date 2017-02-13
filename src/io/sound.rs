use sdl2;
use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioDevice};

pub struct SquareWave {
    pub phase_inc: f32,
    phase: f32,
    volume: f32,
    pub wave_duty: f32,
    pub add: bool,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {

            *x = match self.phase {
                v @ 0.0...1.0 if v <= self.wave_duty => v.sin() * self.volume,
                _ => -self.volume,
            };
            self.phase = if self.add {
                (self.phase + self.phase_inc)
            } else {
                (self.phase - self.phase_inc)
            } % 1.0;
        }
    }
}

pub fn setup_audio(sdl_context: &sdl2::Sdl) -> AudioDevice<SquareWave> {
    // set up audio
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };

    audio_subsystem.open_playback(None, &desired_spec, |spec| {
            // Show obtained AudioSpec
            println!("{:?}", spec);

            // initialize the audio callback
            SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.01,
                wave_duty: 0.25,
                add: true,
            }
        })
        .unwrap()
}
