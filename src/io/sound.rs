//! Everything for making sound play
use sdl2;
use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioDevice};

/// Contains information for a single channel of audio
///
/// Envelopes and sweeps should be implemented as traits to allow this to
/// remain generic
pub struct Wave {
    /// The amount by which the `phase` is changed at each callback
    pub phase_inc: f32,

    /// The "current" value of the wave
    phase: f32,

    /// Multiplier for wave between 0 and 1 (functions as volume (0 is off))
    volume: f32,

    /// TODO: document this
    pub wave_duty: f32,

    /// A flag indicating the direction the phase will be changed
    pub add: bool,
}

impl AudioCallback for Wave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
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

/// Creates a device from a context
/// May have to be changed to allow each GB channel to have its own `Wave`
pub fn setup_audio(sdl_context: &sdl2::Sdl) -> AudioDevice<Wave> {
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
            Wave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.01,
                wave_duty: 0.25,
                add: true,
            }
        })
        .unwrap()
}
