use crate::cpu::Cpu;
use crate::io::constants::*;

// window_x can be changed during scanline interrupts
// window_y is read once at the start of drawing and cached
// sprites with smaller x coords are drawn over ones with larger x coords (draw sprites from right to left?)
// when sprites with same x coords overlap, table ordering takes effect (0xFE00 is highest 0xFE04 is one lower)
// special rules for interpreting sprite pattern data in 8x16 mode
// if > 10 sprites per line then lower priority is removed... (so draw left to right but track each line the number of sprites drawn...)
// sprite priority bit: if on then it's drawn on top of Window and Background (easy)
//                      if off then it's then sprite is only drawn over color 0 of background and window (window and bg can't be transparent)

// interrupts:
// v-blank interrupt occurs at the start of the end of drawing
// LCDC status is used on each line?

struct SpriteData {
    // store pixel data here in unmapped form and mapping?
// is this too hard on CGB? hmm
}

// FF44(LY) LCDC Y coord
// FF45(LYC) value to compare the above to and set a flag (I think this is an interrupt)
pub fn deferred_renderer(cpu: &mut Cpu) -> ([[u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT], usize) {
    let mut bg_pixels = [[0u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT];
    let mut window_pixels = [[0u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT];
    //let mut sprites = vec![];

    let oam_interrupt_enabled = cpu.get_oam_interrupt();
    let vblank_interrupt_enabled = cpu.get_vblank_interrupt_stat();
    // false is !=, true is ==
    let coincidence_equal_interrupt = cpu.get_coincidence_flag();
    let window_y = cpu.window_y_pos();
    let mut total_cycles: u32 = 0;

    for y in 0..GB_SCREEN_HEIGHT {
        // LY
        cpu.inc_ly();
        let coincidence_interrupt_enabled = cpu.get_coincidence_interrupt();
        if cpu.get_interrupts_enabled() && coincidence_interrupt_enabled {
            let coincidence_equal_interrupt = cpu.get_coincidence_flag();
            let cmp_op = if coincidence_equal_interrupt {
                std::cmp::PartialEq::eq
            } else {
                std::cmp::PartialEq::ne
            };
            if cmp_op(&cpu.ly(), &cpu.lyc()) {
                cpu.set_coincidence_flag();
                cpu.set_lcdc_interrupt_bit();
            } else {
                cpu.unset_coincidence_flag();
            }
        }
        {
            cpu.set_oam_lock();
            let mut oam_scan_cycles = 0;
            while oam_scan_cycles < 80 {
                oam_scan_cycles += cpu.dispatch_opcode();
            }
            total_cycles += oam_scan_cycles as u32;
        }
        let scy = cpu.scy();
        let scx = cpu.scx();
        {
            cpu.set_oam_and_display_lock();
            let mut generate_picture_cycles = 0;
            while generate_picture_cycles < 168 {
                generate_picture_cycles += cpu.dispatch_opcode();
            }
            total_cycles += generate_picture_cycles as u32;
        }

        let tile_bg_map_addr = if cpu.lcdc_bg_win_tile_data() {
            0x8000
        } else {
            0x9000
        };
        let adj_y = (scy as u16 + y as u16) & 0xFF;
        for x in 0..GB_SCREEN_WIDTH {
            let adj_x = (scx as u16 + x as u16) & 0xFF;
            // adj_y / 32 * 32
            let idx_into_tile_idx_mem = tile_bg_map_addr + (adj_y & !0x7) + (adj_x >> 3);
            let tile_idx = cpu.mem[idx_into_tile_idx_mem as usize];
            let tile_start = cpu.get_nth_background_tile(tile_idx);

            let tile_ptr = tile_start +
                    // select the correct y pos based on lower 3 bits
                    ((adj_y & 0x7) * 2) +
                    // select which byte based on the upper bit of lower 3 bits
                    ((adj_x & 0x7) >> 2);
            let data = (cpu.mem[tile_ptr as usize]
                // bottom 2 bits select which 2 bit section of the byte to use as the pixel data
                >> ((adj_x & 0x3) * 2))
                & 0x3;
            bg_pixels[y][x] = data;
        }
        {
            cpu.set_hblank();
            let hblank_interrupt_enabled = cpu.get_hblank_interrupt();
            if cpu.get_interrupts_enabled() && hblank_interrupt_enabled {
                cpu.set_lcdc_interrupt_bit();
            }
            let mut hblank_cycles = 0;
            while hblank_cycles < 85 {
                hblank_cycles += cpu.dispatch_opcode();
            }
            total_cycles += hblank_cycles as u32;
        }
    }

    let vblank_interrupt_enabled = cpu.get_vblank_interrupt_stat();
    cpu.set_vblank();
    if cpu.get_interrupts_enabled() && vblank_interrupt_enabled {
        cpu.set_vblank_interrupt_bit();
    }
    // in vblank
    for y in GB_SCREEN_WIDTH..153 {}

    (bg_pixels, total_cycles as usize)
}
