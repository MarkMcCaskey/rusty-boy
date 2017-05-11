//! Everything for making sound play
use sdl2;
use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioDevice};

pub struct GBSound {
    /// The number of samples sent to the sound device every second.
    pub out_freq: f32, // FIXME maybe this is not needed to be stored here?
    pub channel1: SquareWave,
    pub channel2: SquareWave,
    pub channel3: SquareWaveRam,
}


/// Contains information for a single channel of audio
///
/// Envelopes and sweeps should be implemented as traits to allow this to
/// remain generic
pub struct SquareWave {
    /// The amount by which the `phase` is changed at each callback
    pub phase_inc: f32,

    /// The "current" value of the wave
    pub phase: f32,

    /// Multiplier for wave between 0 and 1 (functions as volume (0 is off))
    volume: f32,

    /// TODO: document this
    pub wave_duty: f32,

    /// A flag indicating the direction the phase will be changed
    pub add: bool,
}

pub struct SquareWaveRam {
    /// The amount by which the `phase` is changed at each callback
    pub phase_inc: f32,

    /// The "current" value of the wave
    pub phase: f32,

    /// Multiplier for wave between 0 and 1 (functions as volume (0 is off))
    volume: f32,

    /// TODO: document this
    pub wave_duty: f32,

    /// A flag indicating the direction the phase will be changed
    pub add: bool,

    pub wave_ram: [u8; 32],

    pub wave_ram_index: usize,
    pub shift_amount: u8,
}




trait SoundChannel {
    fn generate_sample(&mut self) -> f32;
}

impl SoundChannel for SquareWave {
    fn generate_sample(&mut self) -> f32 {
        let out = if self.phase <= self.wave_duty {
            self.volume
        } else {
            -self.volume
        };
        self.phase = (self.phase + self.phase_inc) % 1.0;
        // To make it sound slightly nicer
        out.sin()
    }
}

impl SoundChannel for SquareWaveRam {
    fn generate_sample(&mut self) -> f32 {
        let out = if self.phase <= ((self.wave_ram[self.wave_ram_index] >> self.shift_amount) as f32){
            self.volume * self.phase
        } else
            { 0.0 };

/*        self.phase = (self.phase_inc + (1.0 / self.wave_ram[self.wave_ram_index] as f32)) % 1.0;
        // To make it sound slightly nicer
*/
        

        self.phase = (self.phase + self.phase_inc) % 1.0;

        self.wave_ram_index = (self.wave_ram_index + 1) % 32;
        out.sin()
    }
}

impl AudioCallback for GBSound {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            // FIXME is just adding them is the right way to do it?
            // Maybe for floats it is?
            let val = 
                self.channel1.generate_sample()
                + self.channel2.generate_sample()
                + self.channel3.generate_sample();
            *x = val;
            // TODO mix other channels here
        }
    }
}

/// Creates a device from a context
/// May have to be changed to allow each GB channel to have its own `Wave`
pub fn setup_audio(sdl_context: &sdl2::Sdl) -> AudioDevice<GBSound> {
    // set up audio
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };

    audio_subsystem.open_playback(None, &desired_spec, |spec| {
            // Show obtained AudioSpec
            debug!("{:?}", spec);

            // initialize the audio callback
            GBSound {
                out_freq: spec.freq as f32,
                channel1: SquareWave {
                    phase_inc: 440.0 / spec.freq as f32,
                    phase: 0.0,
                    volume: 0.025,
                    wave_duty: 0.25,
                    add: true,
                },
                channel2: SquareWave {
                    phase_inc: 440.0 / spec.freq as f32,
                    phase: 0.0,
                    volume: 0.025,
                    wave_duty: 0.25,
                    add: true,
                },
                channel3: SquareWaveRam {
                    phase_inc: 440.0 / spec.freq as f32,
                    phase: 0.0,
                    volume: 0.025,
                    wave_duty: 0.25,
                    add: true,
                    wave_ram: [0u8; 32],
                    wave_ram_index: 0,
                    shift_amount: 0,
                },
            }
        })
        .unwrap()
}
