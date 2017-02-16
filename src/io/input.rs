//! Input related functions for the emulator (controls)
use sdl2;

///
pub fn setup_controller_subsystem(sdl_context: &sdl2::Sdl)
                                  -> Option<sdl2::controller::GameController> {
    let controller_subsystem = sdl_context.game_controller().unwrap();
    controller_subsystem.load_mappings("controllers/sneslayout.txt").unwrap();

    let available = match controller_subsystem.num_joysticks() {
        Ok(n) => n,
        Err(e) => {
            error!("Joystick error: {}", e);
            0
        }
    };

    let mut controller = None;
    for id in 0..available {
        if controller_subsystem.is_game_controller(id) {
            debug!("Attempting to open controller {}", id);

            match controller_subsystem.open(id) {
                Ok(c) => {
                    info!("Success: opened controller \"{}\"", c.name());
                    controller = Some(c);
                    break;
                }
                Err(e) => warn!("failed to open controller: {:?}", e),
            }

        } else {
            debug!("{} is not a game controller", id);
        }
    }

    controller
}
