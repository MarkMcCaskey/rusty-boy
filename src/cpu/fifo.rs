//! GBA speciifc FIFO

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Fifo {
    data: VecDeque<i8>,
    sound_buffer: Vec<i8>,
    pub reset_flag: bool,
}

impl Fifo {
    pub fn new() -> Self {
        Self {
            data: VecDeque::with_capacity(32),
            sound_buffer: vec![],
            reset_flag: false,
        }
    }

    pub fn push(&mut self, data: i8) {
        if self.data.len() >= 32 {
            self.data.pop_back();
        }
        self.data.push_front(data)
    }

    /*
    /// Buffer uncircularized, copied into a linear array
    pub fn get_buffer(&self) -> [i8; 32] {
        let mut out = [0; 32];
        let n_elements = 32 - self.idx;
        (&mut out[..n_elements]).copy_from_slice(&self.data[self.idx..]);
        (&mut out[n_elements..]).copy_from_slice(&self.data[..self.idx]);
        out
    }
    */

    pub fn reset(&mut self) {
        self.reset_flag = true;
        self.data.clear();
        self.sound_buffer.clear();
    }

    pub fn get_current_data(&self) -> &[i8] {
        &self.sound_buffer
    }
    pub fn inc_note(&mut self) {
        if let Some(d) = self.data.pop_back() {
            self.sound_buffer.push(d);
        }
    }
    pub fn ready_for_more_data(&self) -> bool {
        self.data.len() <= 16
        //self.sound_buffer.len() <= 16
    }
    pub fn clear_sound_buffer(&mut self) {
        self.sound_buffer.clear();
    }
}
