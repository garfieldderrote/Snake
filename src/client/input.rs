use std::{collections::VecDeque, thread::sleep, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::util::Dir;

pub struct Input {
    pub should_quit: bool,
    pub input_buffer: VecDeque<char>,
}

impl Input {
    pub fn clean() {
        _ = disable_raw_mode();
    }
    pub fn new() -> Self {
        _ = enable_raw_mode();
        Input {
            should_quit: false,
            input_buffer: VecDeque::new(),
        }
    }
    pub fn fetch(&mut self) -> Option<()> {
        while event::poll(Duration::ZERO).ok()?
            && let Event::Key(key) = event::read().ok()?
        {
            check_quit_button(self, key);
            if let KeyCode::Char(c) = key.code
                && self.input_buffer.len() < 2
            {
                self.input_buffer.push_back(c);
            }
        }
        Some(())
    }
    pub fn get_dir(&mut self) -> Option<Dir> {
        if let Some(c) = self.input_buffer.pop_front() {
            match c {
                'w' => return Some(Dir::Up),
                's' => return Some(Dir::Down),
                'a' => return Some(Dir::Left),
                'd' => return Some(Dir::Right),
                _ => return None,
            }
        }
        None
    }
}

fn check_quit_button(input: &mut Input, key: KeyEvent) {
    let quit_pressed = match key.code {
        KeyCode::Esc => true,
        KeyCode::Char(c) if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) => true,
        _ => false,
    };
    if quit_pressed {
        input.should_quit = true;
    }
}
