use ncurses::*;
use std::collections::HashMap;
use super::language;
use super::dbglanguage;
use std;
use super::super::cpu::*;
use super::super::disasm::*;
use std::thread::sleep;
use std::time::Duration;


static ASM_WINDOW_HEIGHT: i32 = 3;
static ASM_WINDOW_WIDTH: i32 = 10;
const WIN_Y_DIV: i32 = 5;
const WIN_Y_ADJ: i32 = 2;
const WIN_X_DIV: i32 = 4;
const WIN_X_ADJ: i32 = 3;


pub struct Debugger {
    cpu: Cpu,
    //   symbol_table: HashMap<&'str, Expression>,
    asm_win: WINDOW,
    reg_win: WINDOW,
    in_win: WINDOW,
    dissassembled_rom: Vec<(String, u16)>,
    input_buffer: String,
    output_buffer: Vec<String>,
}

impl Debugger {
    pub fn new(file_name: &str) -> Debugger {
        use ncurses::*;


        let mut max_x = 0;
        let mut max_y = 0;
        initscr();
        // raw();
        cbreak();

        keypad(stdscr(), true);
        // noecho();
        echo();

        getmaxyx(stdscr(), &mut max_y, &mut max_x);
        printw(format!("X: {} Y: {}", max_x, max_y).as_ref());

        let mut cpu = Cpu::new();
        cpu.load_rom(file_name);

        let mut romcp = [0u8; 0x8000];
        for i in 0..0x7FFF {
            romcp[i] = cpu.mem[i] as u8;
        }


        let dbg = Debugger {
            cpu: cpu,
            //         symbol_table: HashMap::new(),
            asm_win: create_win((max_y / WIN_Y_DIV) * WIN_Y_ADJ,
                                (max_x / WIN_X_DIV) * WIN_X_ADJ,
                                0,
                                max_x / WIN_X_DIV),
            reg_win: create_win((max_y / WIN_Y_DIV) * WIN_Y_ADJ, max_x / WIN_X_DIV, 0, 0),
            in_win: create_win(max_y - ((max_y / WIN_Y_DIV) * WIN_Y_ADJ),
                               max_x,
                               (max_y / WIN_Y_DIV) * WIN_Y_ADJ,
                               0),
            dissassembled_rom: disasm_rom_to_vec(romcp, 0x7FF0),
            input_buffer: String::new(),
            output_buffer: vec![],
        };
        printw("Debugger created!");
        //       scrollok(dbg.reg_win, true);
        //     wsetscrreg(dbg.reg_win, 0, 10);
        // clearok(dbg.reg_win, true);
        // clearok(dbg.asm_win, true);
        // clearok(dbg.in_win, true);

        refresh();

        dbg
    }
    pub fn refresh_screen(&mut self) {

        let mut ch = getch();
        match ch {
            KEY_LEFT => {
                self.cpu.dispatch_opcode();
            }
            // numbers
            v @ 0x20...0x7F => {
                self.input_buffer.push_str(String::from_utf8(vec![v as u8]).unwrap().as_ref())
            }
            // Enter (on linux)
            0xA => {
                // do parsing
                let parseval = dbglanguage::parse_Input(self.input_buffer.as_ref()).unwrap();
                let old_input_string = self.input_buffer.clone();
                self.output_buffer.push(old_input_string);
                self.output_buffer.push(format!("{:?}", parseval));
                self.input_buffer = String::new();
            }
            _ => (),
        }

        wclear(self.asm_win);
        wclear(self.in_win);
        box_(self.in_win, 0, 0);
        box_(self.asm_win, 0, 0);
        box_(self.reg_win, 0, 0);

        //        wscrl(self.reg_win, 5);
        self.draw_registers();
        self.draw_registers16();
        self.draw_in();
        self.draw_asm();


        wrefresh(self.asm_win);
        wrefresh(self.reg_win);
        wrefresh(self.in_win);

        refresh();
        wrefresh(self.in_win);
    }

    fn draw_in(&mut self) {
        let mut x = 0;
        let mut y = 0;
        getmaxyx(self.in_win, &mut y, &mut x);

        let num_lines = y - 3; //number of lines to draw backlog/previous input and output

        let lower_limit = if num_lines - (self.output_buffer.len() as i32) > 0 {
            num_lines - (self.output_buffer.len() as i32)
        } else {
            0
        };
        start_color();			/* Start color 			*/
        init_pair(1, COLOR_RED, COLOR_BLACK);

        wattron(self.in_win, COLOR_PAIR(1));


        for i in lower_limit..(num_lines - 1) {
            wmove(self.in_win, (num_lines - i), 1);
            wprintw(self.in_win,
                    self.output_buffer[(num_lines - i - 1) as usize].as_ref());
        }
        wmove(self.in_win, y - 2, 1);

        wprintw(self.in_win, self.input_buffer.as_ref());
        wattroff(self.in_win, COLOR_PAIR(1));

    }

    fn draw_asm(&mut self) {
        let cur_pc = self.cpu.pc;
        let ar_max = self.dissassembled_rom.len() - 1;
        let idx = binsearch_inst(&self.dissassembled_rom,
                                 cur_pc,
                                 0,
                                 ar_max as usize)
            .expect(format!("INVALID INSTRUCTION at {}", self.cpu.pc)
                .as_ref()) as u16;

        if idx > 7 {
            for i in 0..7 {
                let (cur_inst, _) = self.dissassembled_rom[(idx - (7 - i)) as usize].clone();
                let cur_instref = cur_inst.as_ref();
                self.draw_instruction((i + 1) as i32, cur_instref);
            }
            start_color();			/* Start color 			*/
            init_pair(1, COLOR_RED, COLOR_BLACK);

            wattron(self.asm_win, COLOR_PAIR(1));
            // highlight current inst
            let (cur_inst, _) = self.dissassembled_rom[idx as usize].clone();
            let cur_instref = cur_inst.as_ref();
            self.draw_instruction(7, cur_instref);
            wattroff(self.asm_win, COLOR_PAIR(1));

            for i in 8..17 {
                let (cur_inst, _) = self.dissassembled_rom[(idx + (i - 7)) as usize].clone();
                let cur_instref = cur_inst.as_ref();
                self.draw_instruction(i as i32, cur_instref);
            }
        } else {
            // not enough instructions before
            let (cur_inst, _) = self.dissassembled_rom[idx as usize].clone();
            let cur_instref = cur_inst.as_ref();
            start_color();			/* Start color 			*/
            init_pair(1, COLOR_RED, COLOR_BLACK);

            // highlight current inst
            wattron(self.asm_win, COLOR_PAIR(1));
            self.draw_instruction(1, cur_instref);
            wattroff(self.asm_win, COLOR_PAIR(1));



            for i in 1..16 {
                let (cur_inst, _) = self.dissassembled_rom[(idx + i) as usize].clone();
                let cur_instref = cur_inst.as_ref();
                self.draw_instruction((i + 1) as i32, cur_instref);

            }

        }


        //        self.dissassembled_rom;

    }

    fn draw_instruction(&mut self, y_loc: i32, disinst: &str) {
        wmove(self.asm_win, y_loc, 1);
        wprintw(self.asm_win, format!("{}", disinst).as_ref());
    }


    fn draw_register(&mut self, y_loc: i32, name: &str, reg: CpuRegister) {
        wmove(self.reg_win, y_loc, 1);
        wprintw(self.reg_win,
                format!("{:4}: 0x{:02X}",
                        name,
                        self.cpu
                            .access_register(reg)
                            .expect("invalid register"))
                    .as_ref());
    }

    fn draw_register16(&mut self, y_loc: i32, name: &str, reg: CpuRegister16) {
        wmove(self.reg_win, y_loc, 13);
        wprintw(self.reg_win,
                format!("{:2}: 0x{:04X}",
                        name,
                        self.cpu
                            .access_register16(reg))
                    .as_ref());
    }

    fn draw_registers(&mut self) {
        static reg8bit_list: [CpuRegister; 8] = [CpuRegister::A,
                                                 CpuRegister::B,
                                                 CpuRegister::C,
                                                 CpuRegister::D,
                                                 CpuRegister::E,
                                                 CpuRegister::H,
                                                 CpuRegister::HL,
                                                 CpuRegister::L];
        static reg8bit_name: [&'static str; 8] = ["A", "B", "C", "D", "E", "H", "(HL)", "L"];

        for i in 0..8 {
            self.draw_register(i + 1, reg8bit_name[i as usize], reg8bit_list[i as usize]);
        }
        wmove(self.reg_win, 8, 1);
        wprintw(self.reg_win,
                format!("{:4}: 0x{:02X}", "F", self.cpu.f).as_ref());

    }

    fn draw_registers16(&mut self) {
        static reg16bit_list: [CpuRegister16; 4] =
            [CpuRegister16::BC, CpuRegister16::DE, CpuRegister16::HL, CpuRegister16::SP];
        static reg16bit_name: [&'static str; 4] = ["BC", "DE", "HL", "SP"];
        for i in 0..4 {
            self.draw_register16(i + 1, reg16bit_name[i as usize], reg16bit_list[i as usize]);
        }

        // 4
        wmove(self.reg_win, 4, 13);
        wprintw(self.reg_win,
                format!("{:2}: 0x{:04X}", "PC", self.cpu.pc).as_ref());
    }
}

fn create_win(height: i32, width: i32, start_y: i32, start_x: i32) -> WINDOW {
    let win = newwin(height, width, start_y, start_x);
    box_(win, 0, 0);
    wrefresh(win);
    win
}
