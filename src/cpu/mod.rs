#![allow(overflowing_literals)]
#![allow(dead_code)]

#[macro_use] mod macros;
mod tests;

use std::str::from_utf8;

pub const zl: i8 = 0x80;
pub const nl: i8 = 0x40;
pub const hl: i8 = 0x20;
pub const cl: i8 = 0x10;

pub struct Cpu {
    a:   i8,
    b:   i8,
    c:   i8,
    d:   i8,
    e:   i8,
    f:   i8, //NOTE: bit 7: zero flag; bit 6: subtract flag; bit 5: half carry; bit 4: carry flag
    h:   i8,
    l:   i8,
    sp:  u16,
    pc:  u16,
    pub mem: [i8; 0xFFFF + 1],
}

#[derive(Clone,Copy,PartialEq)]
enum CpuRegister {
    A, B, C, D, E, H, L, HL, Num(i8),
}

#[derive(Clone,Copy,PartialEq)]
enum CpuRegister16 {
    BC, DE, HL, SP, Num(i16),
}

#[derive(Clone,Copy)]
enum Cc {
    NZ, Z, NC, C,
}

#[derive(Debug, PartialEq)]
enum CartridgeType  {
    RomOnly = 0,
    RomMBC1 = 1,
    RomMBC1Ram = 2,
    RomMBC1RamBatt = 3,
    RomMBC2 = 5,
    RomMBC2Batt = 6,
    RomRam = 8,
    RomRamBatt = 9,
    RomMMM01 = 0xB,
    RomMMM01SRam = 0xC,
    RomMMM01SRamBatt = 0xD,
    RomMBC3TimerRamBatt = 0x10,
    RomMBC3 = 0x11,
    RomMBC3Ram = 0x12,
    RomMBC3RamBatt = 0x13,
    RomMBC5 = 0x19,
    RomMBC5Ram = 0x1A,
    RomMBC5RamBatt = 0x1B,
    RomMBC5RumbleSRam = 0x1D,
    RomMBC5RumbleSRamBatt = 0x1E,
    PocketCamera = 0x1F,
    BandaiTAMA5 = 0xFD,
    HudsonHuC3 = 0xFE,
    HudsonHuC1 = 0xFF,
}

fn cc_dispatch(num: u8) -> Cc {
    match num {
        0 => Cc::NZ,
        1 => Cc::Z,
        2 => Cc::NC,
        3 => Cc::C,
        _ => panic!("Invalid number for Cc dispatch"),
    }
}

fn cpu_dispatch(num: u8) -> CpuRegister {
    match num {
        0 => CpuRegister::B,
        1 => CpuRegister::C,
        2 => CpuRegister::D,
        3 => CpuRegister::E,
        4 => CpuRegister::H,
        5 => CpuRegister::L,
        6 => CpuRegister::HL,
        7 => CpuRegister::A,
        _ => panic!("Invalid 8bit register in cpu_dispatch!"),
    }
}

fn cpu16_dispatch(num: u8) -> CpuRegister16 {
    match num {
        0 => CpuRegister16::BC,
        1 => CpuRegister16::DE,
        2 => CpuRegister16::HL,
        3 => CpuRegister16::SP,
        _ => panic!("Invalid number for 16bit register dispatch"),
    }
}


impl Cpu {
    pub fn new() -> Cpu {
        let mut new_cpu = Cpu {
            // TODO: abstract this later~
            a:   0x01, //for GB/SGB (GBP & GBC need different values)
            b:   0,
            c:   0,
            d:   0,
            e:   0,
            f:   0xB0,
            h:   0,
            l:   0,
            sp:  0xFFFE,
            pc:  0,
            mem: [0; 0xFFFF + 1]
        };

        //boot sequence (maybe do this by running it as a proper rom?)
        new_cpu.set_bc(0x0013);
        new_cpu.set_de(0x00D8);
        new_cpu.set_hl(0x014D);
        new_cpu.mem[0xFF05] = 0x00;
        new_cpu.mem[0xFF06] = 0x00;
        new_cpu.mem[0xFF07] = 0x00;
        new_cpu.mem[0xFF10] = 0x80;
        new_cpu.mem[0xFF11] = 0xBF;
        new_cpu.mem[0xFF12] = 0xF3;
        new_cpu.mem[0xFF14] = 0xBF;
        new_cpu.mem[0xFF16] = 0x3F;
        new_cpu.mem[0xFF17] = 0x00;
        new_cpu.mem[0xFF19] = 0xBF;
        new_cpu.mem[0xFF1A] = 0x7F;
        new_cpu.mem[0xFF1B] = 0xFF;
        new_cpu.mem[0xFF1C] = 0x9F;
        new_cpu.mem[0xFF1E] = 0xBF;
        new_cpu.mem[0xFF20] = 0xFF;
        new_cpu.mem[0xFF21] = 0x00;
        new_cpu.mem[0xFF22] = 0x00;
        new_cpu.mem[0xFF23] = 0xBF;
        new_cpu.mem[0xFF24] = 0x77;
        new_cpu.mem[0xFF25] = 0xF3;
        new_cpu.mem[0xFF26] = 0xF1; //F1 for GB // TODOA:
        new_cpu.mem[0xFF40] = 0x91;
        new_cpu.mem[0xFF42] = 0x00;
        new_cpu.mem[0xFF43] = 0x00;
        new_cpu.mem[0xFF45] = 0x00;
        new_cpu.mem[0xFF47] = 0xFC;
        new_cpu.mem[0xFF48] = 0xFF;
        new_cpu.mem[0xFF49] = 0xFF;
        new_cpu.mem[0xFF4A] = 0x00;
        new_cpu.mem[0xFF4B] = 0x00;
        new_cpu.mem[0xFFFF] = 0x00;

        new_cpu
    }

    fn ldpanic(&self, reg:CpuRegister16) {
        panic!("(load) opcode not implemented!");
    }

    /* fn get_vblank(&self) -> [u8; 8] {
    let mut ret_arr = [0u8; 8];

} */

    //FF04 Div
    /*
     * This needs to be called 16384 (~16779 on SGB) times a second
     */
    fn inc_div(&mut self) {
        let old_val: u16 = self.mem[0xFF04] as u16;
        self.mem[0xFF04] = (old_val + 1) as i8;
    }

    pub fn timer_frequency(&self) -> u16 {
        match self.mem[0xFF07] & 0x3 {
            0 => 4,
            1 => 262,
            2 => 65,
            3 => 16,
            _ => unreachable!("The impossible happened!"),
        }
    }

    pub fn timer_cycle(&mut self) {
        if self.is_timer_on() {
            self.inc_timer();
        }
    }

    fn is_timer_on(&self) -> bool {
        (self.mem[0xFF07] & 0x4) >> 2 == 1
    }

    fn inc_timer(&mut self) {
        let old_val: u16 = self.mem[0xFF05] as u16;
        let new_val = self.mem[0xFF06];

        self.mem[0xFF05] =
            if old_val + 1 > 255 {
                if self.get_interrupts_enabled() {
                    self.set_timer_interrupt_bit();
                }
                new_val
            } else {old_val as i8};
    }


    set_interrupt_bit!(set_vblank_interrupt_bit, 0x1);
    set_interrupt_bit!(set_lcdc_interrupt_bit, 0x2);
    set_interrupt_bit!(set_timer_interrupt_bit, 0x4);
    set_interrupt_bit!(set_serial_io_interrupt_bit, 0x8);
    set_interrupt_bit!(set_input_interrupt_bit, 0x10);

    unset_interrupt_bit!(unset_vblank_interrupt_bit, 0x1);
    unset_interrupt_bit!(unset_lcdc_interrupt_bit, 0x2);
    unset_interrupt_bit!(unset_timer_interrupt_bit, 0x4);
    unset_interrupt_bit!(unset_serial_io_interrupt_bit, 0x8);
    unset_interrupt_bit!(unset_input_interrupt_bit, 0x10);

    get_interrupt!(get_vblank_interrupt, 0x1);
    get_interrupt!(get_lcdc_interrupt, 0x2);
    get_interrupt!(get_timer_interrupt, 0x4);
    get_interrupt!(get_serial_io_interrupt, 0x8);
    get_interrupt!(get_input_interrupt, 0x10);


    /*
     * SOUND:
     */

    set_sound_on!(set_sound1, 0x1);
    set_sound_on!(set_sound2, 0x2);
    set_sound_on!(set_sound3, 0x4);
    set_sound_on!(set_sound4, 0x8);
    set_sound_on!(set_sound_all, 0x80);

    unset_sound_on!(unset_sound1, 0x1);
    unset_sound_on!(unset_sound2, 0x2);
    unset_sound_on!(unset_sound3, 0x4);
    unset_sound_on!(unset_sound4, 0x8);
    unset_sound_on!(unset_sound_all, 0x80);

    set_interrupt_enabled!(set_vblank_interrupt_enabled, 0x1);
    set_interrupt_enabled!(set_lcdc_interrupt_enabled, 0x2);
    set_interrupt_enabled!(set_timer_interrupt_enabled, 0x4);
    set_interrupt_enabled!(set_serial_io_interrupt_enabled, 0x8);
    set_interrupt_enabled!(set_input_interrupt_enabled, 0x10);
    set_interrupt_enabled!(set_interrupts_enabled, 0x1F);

    unset_interrupt_enabled!(unset_vblank_interrupt_enabled, 0x1);
    unset_interrupt_enabled!(unset_lcdc_interrupt_enabled, 0x2);
    unset_interrupt_enabled!(unset_timer_interrupt_enabled, 0x4);
    unset_interrupt_enabled!(unset_serial_io_interrupt_enabled, 0x8);
    unset_interrupt_enabled!(unset_input_interrupt_enabled, 0x10);
    unset_interrupt_enabled!(unset_interrupts_enabled, 0x1F);

    get_interrupt_enabled!(get_vblank_interrupt_enabled, 0x1);
    get_interrupt_enabled!(get_lcdc_interrupt_enabled, 0x2);
    get_interrupt_enabled!(get_timer_interrupt_enabled, 0x4);
    get_interrupt_enabled!(get_serial_io_interrupt_enabled, 0x8);
    get_interrupt_enabled!(get_input_interrupt_enabled, 0x10);
    get_interrupt_enabled!(get_interrupts_enabled, 0x1F);

    pub fn lcdc_on(&self) -> bool {
        (self.mem[0xFF40] >> 7) & 1 == 1
    }
    pub fn lcdc_tile_map(&self) -> bool {
        (self.mem[0xFF40] >> 6) & 1 == 1
    }
    pub fn lcdc_window_on(&self) -> bool {
        (self.mem[0xFF40] >> 5) & 1 == 1
    }
    pub fn lcdc_bg_win_tile_data(&self) -> bool {
        (self.mem[0xFF40] >> 4) & 1 == 1
    }
    pub fn lcdc_bg_tile_map(&self) -> bool {
        (self.mem[0xFF40] >> 3) & 1 == 1
    }
    pub fn lcdc_sprite_size(&self) -> bool {
        (self.mem[0xFF40] >> 2) & 1 == 1
    }
    pub fn lcdc_sprite_display(&self) -> bool {
        (self.mem[0xFF40] >> 1) & 1 == 1
    }
    pub fn lcdc_bg_win_display(&self) -> bool {
        self.mem[0xFF40] & 1 == 1
    }

    
    pub fn get_background_tiles(&self) -> [[u8; 64]; (32 * 32)] {
        let mut tiles = [[0u8; 64]; (32*32)];
        let tile_map_base_addr = if self.lcdc_bg_tile_map() {0x8000} else {0x9000};
        let tile_data_base_addr = if self.lcdc_bg_win_tile_data() {0x9C00} else {0x9800};
//        debug!("Getting {}th tile at offset {}", offset, tile_map_base_addr);

        if tile_map_base_addr == 0x9C00 {
            for j in 0..(32 * 32) {
                let tile_pointer = self.mem[(tile_map_base_addr + j) as usize];
                for i in 0..16 {
                    for k in 0..4 {
                    //multiply offset by tile size
                        tiles[j][i*(k+1)] = ((self.mem[((tile_data_base_addr as i16) + ((tile_pointer as i16) * 0x40)) as usize] as u8)
                                          >> (k*2)) & 0x3;
                    }
                }
            }
        } else {
            for j in 0..(32 * 32) {
                let tile_pointer = self.mem[(tile_map_base_addr + j) as usize] as u8;
                for i in 0..16 {
                    for k in 0..4 {
                    //multiply offset by tile size
                        tiles[j][i*(k+1)] = ((self.mem[((tile_data_base_addr as u16) + ((tile_pointer as u16) * 0x40)) as usize] as u8)
                                         >> (k * 2)) & 0x3;
                    }
                }
            }

        }

        tiles
    }


    pub fn scy(&self) -> u8 {
        self.mem[0xFF42] as u8
    }
    pub fn scx(&self) -> u8 {
        self.mem[0xFF43] as u8
    }

    pub fn ly(&self) -> u8 {
        self.mem[0xFF44] as u8
    }

    pub fn inc_ly(&mut self) {
        let v = (self.ly() + 1) % 154;
        self.mem[0xFF44] = v as i8;
        //maybe set flags for being in vblank or whatever here?
    }

    pub fn lyc(&self) -> u8 {
        self.mem[0xFF45] as u8
    }

    pub fn lyc_compare(&mut self) {
        let ly = self.ly();
        let lyc = self.lyc();

        if ly == lyc {
            //TODO: set STAT coincident flag...
        }
    }

    fn dma(&mut self) {
        let addr = (self.mem[0xFF46] as u16) << 8;
        
        //TODO: ensure this doesn't include end value
        for i in 0..0xA0 { //number of values to be copied
            let val = self.mem[(addr + i) as usize];
            self.mem[(0xFE00 + i) as usize] = val; //start addr + offset
        }
    }

    pub fn bgp(&self) -> (u8, u8, u8, u8) {
        let v4 = ((self.mem[0xFF47] >> 6) & 0x3) as u8;
        let v3 = ((self.mem[0xFF47] >> 4) & 0x3) as u8;
        let v2 = ((self.mem[0xFF47] >> 2) & 0x3) as u8;
        let v1 = ((self.mem[0xFF47] >> 0) & 0x3) as u8;

        (v1,v2,v3,v4)
    }

    pub fn obp0(&self) -> (u8, u8, u8, u8) {
        let v4 = ((self.mem[0xFF48] >> 6) & 0x3) as u8;
        let v3 = ((self.mem[0xFF48] >> 4) & 0x3) as u8;
        let v2 = ((self.mem[0xFF48] >> 2) & 0x3) as u8;
        let v1 = ((self.mem[0xFF48] >> 0) & 0x3) as u8;

        (v1,v2,v3,v4)
    }

    pub fn obp1(&self) -> (u8, u8, u8, u8) {
        let v4 = ((self.mem[0xFF49] >> 6) & 0x3) as u8;
        let v3 = ((self.mem[0xFF49] >> 4) & 0x3) as u8;
        let v2 = ((self.mem[0xFF49] >> 2) & 0x3) as u8;
        let v1 = ((self.mem[0xFF49] >> 0) & 0x3) as u8;

        (v1,v2,v3,v4)
    }

    pub fn wy(&self) -> u8 {
        self.mem[0xFF4A] as u8
    }

    pub fn wx(&self) -> u8 {
        self.mem[0xFF4B] as u8
    }

    //input register for joypad
    /*
     * 0x80 = start
     * 0x40 = select
     * 0x20 = B
     * 0x10 = A
     * 0x8  = Down
     * 0x4  = Up
     * 0x2  = Left
     * 0x1  = Right
     */
    /* Input : */
    /* NOTE: see comment next to definition of button! for why this 
    has to be done with so much boiler plate */
    button!(press_start,  unpress_start,  0x80);
    button!(press_select, unpress_select, 0x40);
    button!(press_b,      unpress_b,      0x20);
    button!(press_a,      unpress_a,      0x10);
    button!(press_down,   unpress_down,   0x8);
    button!(press_up,     unpress_up,     0x4);
    button!(press_left,   unpress_left,   0x2);
    button!(press_right,  unpress_right,  0x1);

    pub fn get_game_name(&self) -> String {
        let mut name_data: Vec<u8> = vec!();
        for i in 0..16 {
            if self.mem[0x134 + i] != 0 {
                name_data.push(self.mem[0x134 + i] as u8);
            }
        }
        String::from_utf8(name_data).unwrap()
    }

    pub fn get_cartridge_type(&self) -> u8 {
        self.mem[0x147] as u8
    }


    fn enable_interrupts(&mut self) {
        self.set_mem(0xFFFF, 0x1F); //verify value to be written here
    }

    fn disable_interrupts(&mut self) {
        self.set_mem(0xFFFF, 0); //verify value to be written here
    }

    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    fn set_hl(&mut self, hlv:u16) {
        self.h = ((hlv & 0xFF00) >> 8) as i8;
        self.l = (hlv & 0xFF)          as i8;
    }

    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    fn de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    fn set_bc(&mut self, bcv: u16) {
        self.b = ((bcv & 0xFF00) >> 8) as i8;
        self.c = (bcv & 0xFF)          as i8;
    }

    fn set_de(&mut self, dev: u16) {
        self.d = ((dev & 0xFF00) >> 8) as i8;
        self.e = (dev & 0xFF)          as i8;
    }

    fn set_register16(&mut self, reg: CpuRegister16, val: u16) {
        match reg {
            CpuRegister16::BC => self.set_bc(val),
            CpuRegister16::DE => self.set_de(val),
            CpuRegister16::HL => self.set_hl(val),
            CpuRegister16::SP => self.sp = val,
            _  => panic!("Invalid 16bit register!"),
        }
    }

    fn access_register16(&mut self, reg: CpuRegister16) -> u16 {
        match reg {
            CpuRegister16::BC     => self.bc(),
            CpuRegister16::DE     => self.de(),
            CpuRegister16::HL     => self.hl(),
            CpuRegister16::SP     => self.sp, 
            CpuRegister16::Num(i) => i as u16,
        }
    }

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        let zn = (z as i8) << 7;
        let nn = (n as i8) << 6;
        let hn = (h as i8) << 5;
        let cn = (c as i8) << 4;

        self.f = zn | nn | hn | cn;
    }


    /*
    NOTE: serial I/O is done by accessing memory addresses.
    It is read at 8192Hz, one bit at a time if external clock is used.
    See documentation and read carefully before implementing this.
     */
    fn set_mem(&mut self, address: usize, value: i8) {
        match address {
            ad @ 0xE000 ... 0xFE00 | ad @ 0xC000 ... 0xDE00
                => {
                    self.mem[ad]                     = value;
                    self.mem[ad ^ (0xE000 - 0xC000)] = value;
                },
            0xFF04 => self.mem[0xFF04] = 0,
            0xFF44 => self.mem[0xFF44] = 0,
            0xFF46 => {
                self.mem[0xFF46] = value;
                self.dma();
            }
            n => self.mem[n] = value,
        }
    }

    fn access_register(&self, reg: CpuRegister) -> Option<i8> {
        match reg {
            CpuRegister::A  => Some(self.a),
            CpuRegister::B  => Some(self.b),
            CpuRegister::C  => Some(self.c),
            CpuRegister::D  => Some(self.d),
            CpuRegister::E  => Some(self.e),
            CpuRegister::H  => Some(self.h),
            CpuRegister::L  => Some(self.l),
            CpuRegister::HL => Some(self.mem[self.hl() as usize]),
            _               => None,
        } 
    }

    fn set_register(&mut self, reg: CpuRegister, val:i8) {
        match reg {
            CpuRegister::A  => self.a = val,
            CpuRegister::B  => self.b = val,
            CpuRegister::C  => self.c = val,
            CpuRegister::D  => self.d = val,
            CpuRegister::E  => self.e = val,
            CpuRegister::H  => self.h = val,
            CpuRegister::L  => self.l = val,
            CpuRegister::HL => {
                let hlv = self.hl();
                self.set_mem(hlv as usize, val);
            },
            _               => panic!("Cannot set non-8bit values"),
        } 
    }


    fn ldnnn(&mut self, nn: CpuRegister, n: u8) {
        self.set_register(nn, n as i8);
    }

    fn ldr1r2(&mut self, r1: CpuRegister, r2:CpuRegister) {
        let val = self.access_register(r2).expect("Invalid register");
        self.set_register(r1, val);
    }

    fn ldan(&mut self, n: CpuRegister) {
        let val = self.access_register(n).expect("Invalid register");
        self.set_register(CpuRegister::A, val);
    }

    fn ldan16(&mut self, n: CpuRegister16) {
        let addr = self.access_register16(n);
        let val = self.mem[addr as usize];

        self.set_register(CpuRegister::A, val);
    }

    fn ldan16c(&mut self, b1: u8, b2: u8) {
        let val = self.mem[(((b2 as u16) << 8) | (b1 as u16)) as usize];
        self.set_register(CpuRegister::A, val);
    }

    fn ldna(&mut self, n: CpuRegister) {
        let val = self.access_register(CpuRegister::A).expect("Invalid register");
        self.set_register(n, val);
    }

    fn ldna16(&mut self, n: CpuRegister16) {
        let val = self.access_register(CpuRegister::A).expect("Invalid register");
        let addr = self.access_register16(n);

        self.set_mem(addr as usize, val);
    }

    fn ldna16c(&mut self, b1: u8, b2: u8) {
        let val = self.access_register(CpuRegister::A).expect("Invalid register");
        self.set_mem((((b2 as u16) << 8) | (b1 as u16)) as usize, val);
    }

    fn ldac(&mut self) {
        let val = self.mem[0xFF00] + self.c;
        self.set_register(CpuRegister::A, val);
    }

    fn ldca(&mut self) {
        let addr = 0xFF00 + self.c;
        let val = self.a;
        self.set_mem(addr as usize, val);
    }

    fn lddahl(&mut self) {
        let addr = self.hl();
        let val = self.mem[addr as usize];

        self.set_register(CpuRegister::A, val);
        self.dec16(CpuRegister16::HL);
    }

    fn lddhla(&mut self) {
        let val = self.a;
        let addr = self.hl();

        self.set_mem(addr as usize, val);
        self.dec16(CpuRegister16::HL);
    }

    fn ldiahl(&mut self) {
        let addr = self.hl();
        let val = self.mem[addr as usize];

        self.set_register(CpuRegister::A, val);
        self.inc16(CpuRegister16::HL);
    }

    fn ldihla(&mut self) {
        let val = self.a;
        let addr = self.hl();

        self.set_mem(addr as usize, val);
        self.inc16(CpuRegister16::HL);
    }

    fn ldhna(&mut self, n: u8) {
        let val = self.a;
        self.set_mem((0xFF00 + n) as usize, val);
    }

    fn ldhan(&mut self, n: u8) {
        let val = self.mem[(0xFF00 + n) as usize];
        self.set_register(CpuRegister::A, val);
    }

    fn ldnnn16(&mut self, n: CpuRegister16, b1: u8, b2: u8) {
        self.set_register16(n, ((b2 as u16) << 8) | (b1 as u16));
    }

    fn ldsphl(&mut self) {
        let val = self.hl();
        self.set_register16(CpuRegister16::SP, val);
    }

    fn ldhlspn(&mut self, n: u8) {
        let val = (self.sp as i16) + (n as i16);
        self.set_register16(CpuRegister16::HL, val as u16);

        self.set_flags(false, false, false, false); //last two need to be checked; TODO:
    }

    fn ldnnsp(&mut self, b1: u8, b2: u8) {
        let old_sp = self.sp;

        self.set_mem((((b2 as u16) << 8) | (b1 as u16)) as usize, old_sp as i8);
    }

    fn pushnn(&mut self, nn: CpuRegister16) {
        let val = self.access_register16(nn);

        self.push_onto_stack(val);
    }

    fn popnn(&mut self, nn: CpuRegister16) {
        let val = self.pop_from_stack();
        self.set_register16(nn, val);
    }
    
    //TODO: rename this awfully named function
    fn alu_dispatch<F>(&self, reg: CpuRegister, f: F) -> i16 where
        F: FnOnce(i8, i8) -> i16 {
        f(self.a,
          match reg {
              CpuRegister::A      => self.a,
              CpuRegister::B      => self.b,
              CpuRegister::C      => self.c,
              CpuRegister::D      => self.d,
              CpuRegister::E      => self.e,
              CpuRegister::H      => self.h,
              CpuRegister::L      => self.l,
              CpuRegister::HL     => self.mem[self.hl() as usize],
              CpuRegister::Num(i) => i, 
          })
    }

    //TODO: rename this awfully named function
    fn alu_dispatch16<F>(&self, reg: CpuRegister16, f: F) -> i32 where
    //TODO: Maybe an issue here?
        F: FnOnce(i32, i32) -> i32 {
        f(self.hl() as i32,
          match reg {
              CpuRegister16::BC     => self.bc() as i32,
              CpuRegister16::DE     => self.de() as i32,
              CpuRegister16::HL     => self.hl() as i32,
              CpuRegister16::SP     => self.sp   as i32,
              CpuRegister16::Num(i) => i as i32, 
          }) as i32
    }

    fn reg_or_const(&mut self, reg: CpuRegister) -> i16 {
        if let Some(r) = self.access_register(reg) {
            r as i16
        } else { //constant value
            if let CpuRegister::Num(v) = reg {
                v as i16
            } else { unreachable!("The impossible happened!") }
        }
    }

    fn addspn(&mut self, n:i8) {
        let new_sp = (self.sp as i16) + (n as i16);
        self.sp = new_sp as u16;

        self.set_flags(false, false, false, false); //TODO: review last tw
    }
    
    fn add(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);

        let new_a = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) + (b as i16));

        self.a = new_a as i8;

        self.set_flags((new_a as i8) == 0,
                       false,
                       ((old_a % 16) + (old_b % 16)) > 15,
                       (old_a + old_b) > i8::max_value() as i16);
    }

    fn adc(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);
        let cf: i8 = self.f & hl >> 5;
        self.add(reg);

        let new_a: i16 = (cf + self.a) as i16;


        self.f |= if (old_a + old_b) > (i8::max_value() as i16) { hl } else { 0 };
        self.f |= if ((old_a % 16) + (old_b % 16)) > 15 { cl } else { 0 };
    }

    fn sub(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) - (b as i16));

        self.a = new_a as i8;
        self.set_flags((new_a as i8) == 0,
                       true,
                       (old_a & 0xF) >= (old_b & 0xF),
                       old_b <= old_a as i16);
    }

    fn sbc(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);
        let old_c = (old_a - old_b) as i8;
        let cf: i8 = self.f & hl >> 5;
        self.sub(reg);

        //NOTE: find out whether this should be self.a - cf
        let new_a: i16 = (self.a - cf) as i16;
        self.a = new_a as i8;
        self.set_flags((new_a as i8) == 0,
                       true,
                       (old_c & 0xF) >= cf,
                       cf <= old_c);
    }

    fn and(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) & (b as i16));

        self.a = new_a as i8;
        self.set_flags(new_a == 0, false, true, false);
    }

    fn or(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| ((a as u16) | (b as u16)) as i16);

        self.a = new_a as i8;
        self.set_flags(new_a == 0, false, false, false);
    }

    fn xor(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) ^ (b as i16));

        self.a = new_a as i8;
        self.set_flags(new_a == 0, false, false, false);
    }

    fn cp(&mut self, reg: CpuRegister) {
        let old_a = self.a;
        self.sub(reg);
        self.a = old_a;
        self.f |= nl;
    }

    fn inc(&mut self, reg: CpuRegister) {
        let old_c = (self.f & hl) == hl;
        let old_3bit = self.access_register(reg).expect("invalid register") & 0x8;
        //old_3bit is used to detect overflow of 3rd bit

        let new_val: i16 = self.alu_dispatch(reg, |_, b: i8| (b + 1) as i16);
        self.set_register(reg, new_val as i8);
        self.set_flags(new_val == 0,
                       false,
                       old_3bit == 0x8 && (new_val & 0x8 == 0),
                       old_c);
    }

    fn dec(&mut self, reg: CpuRegister) {
        let old_c = (self.f & hl) == hl;
        let old_4bit = self.a & 0x10; //TODO: rename this/redo this
        //old_4bit is used to detect overflow of 4th bit

        let new_a: i16 = self.alu_dispatch(reg, |_, b: i8| (b - 1) as i16);
        self.a = new_a as i8;

        self.f = if new_a == 0 { zl } else { 0 };
        //self.f |= old_c;
        //self.f |= if old_4bit == 0x8 && (self.a & 0x8 == 0) { nl } else { 0 };
        //TODO: borrowing of 4th bit flag
        /*        self.set_flags(new_a == 0,
        false,
        old_c
         */
    }

    fn add_hl(&mut self, reg: CpuRegister16) {
        let old_z = (self.f & zl) == zl;
        //TODO: review after sleeping if this actually makes sense for checking middle-ish overflow (carefully consider negative numbmers)
        let old_11bit = self.a & 0x800;

        let new_hl = self.alu_dispatch16(reg, |a:i32, b:i32| (a as i32) + (b as i32));

        self.set_hl(new_hl as u16);
        self.set_flags(old_z,
                       false,
                       old_11bit == 1 && (new_hl & 0x800 == 0),
                       new_hl > (u16::max_value() as i32));
    }

    //Consider adding further restrictions to this type; argument must be an immediate value
    fn add_sp(&mut self, reg: CpuRegister16) {
        if let CpuRegister16::Num(i) = reg {
            self.sp = ((self.sp as i16) + i )as u16;
            //TODO: figure out what happens to the bottom two flags
            self.f &= 0x3F;
        }
        else {
            panic!("In add_sp, invalid argument.  It must be an immediate value");
        }

    }

    fn inc16(&mut self, reg: CpuRegister16) {
        match reg {
            CpuRegister16::BC => { let old_v = self.bc()+1; self.set_bc(old_v); },
            CpuRegister16::DE => { let old_v = self.de()+1; self.set_de(old_v); },
            CpuRegister16::HL => { let old_v = self.hl()+1; self.set_hl(old_v); },
            CpuRegister16::SP => self.sp += 1,
            _ => panic!("inc16 cannot take numeric values as arguments"),
        } 
    }

    fn dec16(&mut self, reg: CpuRegister16) {
        match reg {
            CpuRegister16::BC => { let old_v = self.bc()-1; self.set_bc(old_v); },
            CpuRegister16::DE => { let old_v = self.de()-1; self.set_de(old_v); },
            CpuRegister16::HL => { let old_v = self.hl()-1; self.set_hl(old_v); },
            CpuRegister16::SP => self.sp -= 1,
            _ => panic!("dec16 cannot take numeric values as arguments"),
        } 
    }

    fn swap(&mut self, reg: CpuRegister) {
        //Potentially can bitmask hl which is 16bit value
        let val = self.access_register(reg).expect("couldn't access register value");
        let top = val & 0xF0;
        let bot = val & 0x0F;
        self.set_register(reg, (top >> 4) | (bot << 4));
        
        self.f = if val == 0 { zl } else { 0 };
    }


    fn daa(&mut self) {
        let reduced_a = self.a as u16;

        let lowest_bits = reduced_a & 0xF;

        let lowest_digit = if lowest_bits > 9 {(lowest_bits + 6) & 0xF} else {lowest_bits};
        let highest_bits = ((reduced_a & 0xF0) + (if lowest_digit == lowest_bits {0} else {0x10})) & 0xF0;
        let highest_digit = if highest_bits > 0x90 {(highest_bits + 0x60) & 0xF0} else {highest_bits & 0xF0};

        self.a = (highest_digit | lowest_digit) as i8;
        let old_nflag = (self.f & nl) == nl;
        self.set_flags((highest_digit | lowest_digit) == 0,
                       old_nflag,
                       false,
                       0x99 < reduced_a); //NOTE: weird documentation, unclear value
    }

    fn cpl(&mut self) {
        let new_val = !self.a;
        let old_flags = self.f & (zl | cl);
        self.f = old_flags | nl | hl;
        self.a = new_val;
    }

    fn ccf(&mut self) {
        let old_flags = self.f & (zl | cl);
        self.f = old_flags ^ cl;
    }

    fn scf(&mut self) {
        let old_flags = self.f & zl;
        self.f = old_flags | cl;
    }

    fn nop(&self) {
        ()
    }

    //TODO:
    fn halt(&mut self) {
        debug!("HALT");
        
    }

    //TODO:
    fn stop(&mut self) {

    }

    fn di(&mut self) {
        self.disable_interrupts();
    }

    fn ei(&mut self) {
        self.enable_interrupts();
    }

    fn rlca(&mut self) {
        let old_bit7 = (self.a >> 7) & 1;

        let new_a = (self.a << 1) | old_bit7;
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn rla(&mut self) {
        let old_bit7 = (self.a >> 7) & 1;
        let old_flags = (self.f & cl) >> 4;
        

        let new_a = (self.a << 1) | old_flags;
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit7 == 1);       
    }

    fn rrca(&mut self) {
        let old_bit0 = self.a & 1;

        let new_a = (self.a >> 1) | (old_bit0 << 7);
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn rra(&mut self) {
        let old_bit0 = self.a & 1;
        let old_flags = (self.f & cl) >> 4;

        let new_a = (self.a >> 1) | (old_flags << 7);
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn rlc(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;

        let new_reg = (reg_val << 1) | old_bit7;
        self.set_register(reg, new_reg);

        self.set_flags(new_reg == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn rl(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;
        let old_flags = (self.f & cl) >> 4;

        let new_reg = (reg_val << 1) | old_flags;
        self.set_register(reg, new_reg);

        self.set_flags(new_reg == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn rrc(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;

        let new_val = (reg_val >> 1) | (old_bit0 << 7);
        self.set_register(reg, new_val);

        self.set_flags(new_val == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn rr(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;
        let old_flags = (self.f & cl) >> 4;

        let new_val = (reg_val >> 1) | old_flags;
        self.set_register(reg, new_val);

        self.set_flags(new_val == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn sla(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = reg_val >> 7;
        self.set_register(reg, reg_val << 1);

        self.set_flags((reg_val << 1) == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn sra(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;
        self.set_register(reg, reg_val >> 1);

        self.set_flags((reg_val >> 1) == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn srl(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register") as u8;
        let old_bit0 = reg_val & 1;

        self.set_register(reg, (reg_val >> 1) as i8);

        self.set_flags((reg_val >> 1) == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn bit(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_flags = (self.f & cl) >> 4;
        
        self.set_flags((reg_val >> b) & 1 == 1,
                       false,
                       true,
                       old_flags & 1 == 1);
    }

    fn set(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        self.set_register(reg, reg_val | (1 << b));
    }

    fn res(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        self.set_register(reg, reg_val & (!(1 << b)));
    }

    fn jpnn(&mut self, nn: u16) {
        self.pc = nn; //NOTE: Verify this byte order
    }

    fn jpccnn(&mut self, cc: Cc, nn: u16) -> bool {
        if 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            } {
                self.pc = nn;
                true
            } else { false }
    }

    //TODO: Double check (HL) HL thing
    fn jphl(&mut self) {
        let addr = self.hl();
        self.pc = addr;
    }

    fn jrn(&mut self, n: i8) {
        let old_pc = self.pc;
        self.pc = ((old_pc as i32) + (n as i32)) as u16;
    }

    fn jrccn(&mut self, cc: Cc, n: i8) -> bool {
        if 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            } {
                let old_pc = self.pc;
                self.pc = ((old_pc as i32) + (n as i32)) as u16;
                true
            } else { false }
    }

    //TODO: Verify if SP should be incremented first
    fn callnn(&mut self, nn: u16) {
        self.push_onto_stack(nn);
        self.pc = nn;
    }

    fn push_onto_stack(&mut self, nn: u16) {
        let first_half = (nn >> 8) as i8;
        let second_half = (nn & 0xFF) as i8;
        let old_sp = self.sp;

        self.set_mem((old_sp-1) as usize, first_half);
        self.set_mem((old_sp-2) as usize, second_half);

        self.sp -= 2;
    }

    fn callccnn(&mut self, cc: Cc, nn: u16) -> bool {
        if 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            } {
                self.callnn(nn);
                true
            } else { false }
    }

    fn rst(&mut self, n: u8) {
        let old_pc = self.pc;
        
        self.push_onto_stack(old_pc);

        self.pc = n as u16;
    }

    fn pop_from_stack(&mut self) -> u16 {
        let old_sp = self.sp;
        let val1 = self.mem[old_sp as usize];
        let val2 = self.mem[(old_sp+1) as usize];

        ((val2 as u16) << 8) | (val1 as u16)
    }

    fn ret(&mut self) {
        let new_addr = self.pop_from_stack();
        self.pc = new_addr;
    }

    fn retcc(&mut self, cc: Cc) -> bool {
        if 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            } {
                self.ret();
                true
            } else { false }
    }

    fn reti(&mut self) {
        let new_addr = self.pop_from_stack();

        self.pc = new_addr;
        self.ei();
    }

    fn read_instruction(&self) -> (u8, u8, u8, u8) {
        if self.pc > (0xFFFF - 3) {
            panic!("Less than 4bytes to read!!!\nNote: this may not be a problem with the ROM; if the ROM is correct, this is the result of lazy programming on my part -- sorry");
        }
        (self.mem[self.pc as usize] as u8,
         self.mem[(self.pc + 1) as usize] as u8,
         self.mem[(self.pc + 2) as usize] as u8,
         self.mem[(self.pc + 3) as usize] as u8)
    }

    fn inc_pc(&mut self) {
        self.pc += 1;
    }
    /*
    Handles running opcodes
    Opcodes are prefixed or unprefixed 
    1. [prefix byte,] opcode [,displacement byte] [,immediate data]
    2. prefix byte, prefix byte, displacement byte, opcode
    
    ASSUMPTION: Gameboy only uses the CB prefix codes of the Z80
    
    Returned value is number of cycles that the instruction took
     */
    pub fn dispatch_opcode(&mut self) -> u8 {
        let mut inst_time = 4;
        let (first_byte, second_byte, third_byte, fourth_byte)
            = self.read_instruction();
        let x = (first_byte >> 6) & 0x3;
        let y = (first_byte >> 3) & 0x7;
        let z = first_byte        & 0x7;

        trace!("Running instruction at 0x{:X}", self.pc);
        trace!("First byte of instruction is: 0x{:X}", first_byte);

        let uf = "The impossible happened!";

        if first_byte == 0xCB { //prefixed instruction
            let x = (second_byte >> 6) & 0x3;
            let y = (second_byte >> 3) & 0x7;
            let z = second_byte        & 0x7;

            match x { // xxyy yzzz
                0 => match y {
                    //(cpu_dispatch(z))
                    0 => self.rlc(cpu_dispatch(z)),
                    1 => self.rrc(cpu_dispatch(z)),
                    2 => self.rl(cpu_dispatch(z)),
                    3 => self.rr(cpu_dispatch(z)),
                    4 => self.sla(cpu_dispatch(z)),
                    5 => self.sra(cpu_dispatch(z)),
                    6 => self.swap(cpu_dispatch(z)),
                    7 => self.srl(cpu_dispatch(z)),
                    _ => unreachable!(uf),
                },

                1 => self.bit(y, cpu_dispatch(z)),

                2 => self.res(y, cpu_dispatch(z)),

                3 => self.set(y, cpu_dispatch(z)),

                _ => unreachable!(uf),
            }

            inst_time =
                if CpuRegister::HL == cpu_dispatch(z) {16} else {8};

            self.inc_pc();
        } else { //unprefixed instruction
            match x {
                0 =>
                    match z {
                        0 =>
                            match y {
                                0        => self.nop(), //0x00
                                1        => {
                                    self.ldnnsp(second_byte, third_byte);
                                    self.inc_pc();
                                    self.inc_pc();
                                    inst_time = 20;
                                }, //0x08
                                2        => self.stop(), //0x10
                                3        => {
                                    self.jrn(second_byte as i8);
                                    self.inc_pc();
                                    inst_time = 8;
                                },  //0x18
                                v @ 4...7 => {
                                    inst_time = 8 + if
                                        self.jrccn(cc_dispatch(v-4),
                                                   second_byte as i8) {4}
                                    else {0};
                                    self.inc_pc();
                                },  //0x20, 0x28, 0x30, 0x38
                                _        => unreachable!(uf),
                            },
                        
                        1 =>  //00yy y001
                        {
                            inst_time = if y % 2 == 0 {
                                self.ldnnn16(cpu16_dispatch(y/2),
                                             second_byte, third_byte);
                                self.inc_pc();
                                self.inc_pc();
                                12
                            } else {
                                self.add_hl(cpu16_dispatch(y/2));
                                8
                            };
                        },
                        
                        2 => //00yy y010
                        {
                            match y {
                                0 | 2 => self.ldna16(cpu16_dispatch(y/2)),
                                1 | 3 => self.ldan16(cpu16_dispatch(y/2)),
                                4 => self.ldihla(),
                                5 => self.ldiahl(),
                                6 => self.lddhla(),
                                7 => self.lddahl(),
                                _ => unreachable!(uf),
                            }
                            inst_time = 8;
                        },

                        3 => //00yy y011
                        {
                            even_odd_dispatch!(y, self, inc16, dec16, cpu16_dispatch, cpu16_dispatch, 1, 1);
                            inst_time = 8;
                        },

                        4 => //00yy y100
                        {
                            self.inc(cpu_dispatch(y));
                            inst_time =
                                if cpu_dispatch(y) == CpuRegister::HL {
                                    12} else {4};
                        },
                        
                        5 =>
                        {
                            self.dec(cpu_dispatch(y));
                            inst_time =
                                if cpu_dispatch(y) == CpuRegister::HL {
                                    12} else {4};
                        },

                        6 =>
                        {
                            self.ldnnn(cpu_dispatch(y), second_byte);
                            self.inc_pc();
                            inst_time =
                                if cpu_dispatch(y) == CpuRegister::HL {
                                    12} else {8};
                        },

                        7 => match y { //00yy y111
                            0 => self.rlca(),
                            1 => self.rrca(),
                            2 => self.rla(),
                            3 => self.rra(),
                            4 => self.daa(),
                            5 => self.cpl(),
                            6 => self.scf(),
                            7 => self.ccf(),
                            _ => unreachable!(uf),
                        },

                        _ => unreachable!(uf),
                    }, //end x=0

                1 => match (z,y) {
                    (6,6) => self.halt(),
                    (n,m) => {
                        self.ldr1r2(cpu_dispatch(m), cpu_dispatch(n));
                        inst_time = match (cpu_dispatch(m), cpu_dispatch(n)) {
                            (CpuRegister::HL,_) => 8,
                            (_,CpuRegister::HL) => 8,
                            _                   => 4,
                        }
                    },
                }, //end x = 1

                2 => { match y //10yy y000
                       {
                           0 => self.add(cpu_dispatch(z)),
                           1 => self.adc(cpu_dispatch(z)),
                           2 => self.sub(cpu_dispatch(z)),
                           3 => self.sbc(cpu_dispatch(z)),
                           4 => self.and(cpu_dispatch(z)),
                           5 => self.xor(cpu_dispatch(z)),
                           6 => self.or(cpu_dispatch(z)),
                           7 => self.cp(cpu_dispatch(z)),
                           _ => unreachable!(uf),
                       };
                       //TODO: double check the line below 
                       inst_time = if z == 6 {8} else {4};
                }, //end x = 2

                3 => match z //11yy y000
                {

                    0 => match y {
                        v @ 0...3 => inst_time = if
                            self.retcc(cc_dispatch(v)) {20} else {8},
                        4 => {
                            self.ldhna(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        5 => { //0xE8
                            self.addspn(second_byte as i8);
                            self.inc_pc();
                            inst_time = 16;
                        },
                        6 => {
                            self.ldhan(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        7 => {
                            self.ldhlspn(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        _ => unreachable!(uf),
                    },

                    1 => if y % 2 == 0 { //11yy y001
                        let adjusted_value = y / 2;
                        let val = self.pop_from_stack();
                        self.set_register16(cpu16_dispatch(adjusted_value), val);
                        inst_time = 12;
                    } else {
                        let adjusted_value = y / 2;
                        match adjusted_value {
                            0 => {
                                self.ret();
                                inst_time = 16;
                            },
                            1 => {
                                self.reti();
                                inst_time = 16;
                            },
                            2 => self.jphl(),
                            3 => {
                                self.ldsphl();
                                inst_time = 8;
                            },
                            _ => unreachable!(uf),
                        }
                    },

                    2 => match y {
                        v @ 0...3 => { // 11yy y010
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8); 
                            inst_time = if
                                self.jpccnn(cc_dispatch(v),
                                            const_val) {16} else {12};
                            self.inc_pc();
                            self.inc_pc();
                        },
                        4 => { // 0xE2
                            self.ldca();
                            inst_time = 8;
                        },
                        5 => { //0xEA
                            self.ldan16c(second_byte, third_byte);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 16;
                        },
                        6 => { //0xF2
                            self.ldac();
                            inst_time = 8;
                        },
                        7 => { //0xFA
                            self.ldan16c(second_byte, third_byte);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 16;
                        },
                        _ => unreachable!(uf),
                    },

                    3 => match y { //11yy y011
                        0 => {
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8); 
                            self.jpnn(const_val);
                            self.inc_pc();
                            self.inc_pc();

                            inst_time = 16;
                        },
                        6 => self.di(),
                        7 => self.ei(),
                        _ => panic!("Illegal opcode"),
                    },

                    4 => {

                        let const_val = (second_byte as u16) | ((third_byte as u16) << 8); 
                        inst_time =
                            if self.callccnn(cc_dispatch(y),
                                             const_val) {24} else {12};
                        self.inc_pc();
                        self.inc_pc();
                    },

                    5 => {
                        if y % 2 == 0 {
                            let value = self.access_register16(cpu16_dispatch(y / 2));
                            self.push_onto_stack(value);
                            inst_time = 16;
                        } else if y == 1 {
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8); 
                            self.callnn(const_val);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 24;
                        } else {
                            panic!("Invalid opcode: {}", first_byte)
                        }
                    },

                    6 => {
                        match y {
                            0 => self.add(CpuRegister::Num(second_byte as i8)),
                            1 => self.adc(CpuRegister::Num(second_byte as i8)),
                            2 => self.sub(CpuRegister::Num(second_byte as i8)),
                            3 => self.sbc(CpuRegister::Num(second_byte as i8)),
                            4 => self.and(CpuRegister::Num(second_byte as i8)),
                            5 => self.xor(CpuRegister::Num(second_byte as i8)),
                            6 => self.or(CpuRegister::Num(second_byte as i8)),
                            7 => self.cp(CpuRegister::Num(second_byte as i8)),
                            _ => unreachable!(uf),
                        };
                        inst_time = 8;
                        self.inc_pc();
                    },

                    7 => {
                        self.rst(8*y);
                        inst_time = 16;
                    },
                        
                    _ => unreachable!(uf),
                },
                _ => panic!("The impossible happened!"),
            }
        }
        
        self.inc_pc();

        inst_time
    }

    pub fn load_rom(&mut self, file_path: &str) {
        use std::fs::File;
        use std::io::Read;
        

        let mut rom = File::open(file_path).expect("Could not open rom file");
        let mut rom_buffer: [u8; 0x8000] = [0u8; 0x8000];

        rom.read(&mut rom_buffer).unwrap();


        for i in 0..0x8000 {
            self.set_mem(i, rom_buffer[i] as i8);
        }

    }
}


