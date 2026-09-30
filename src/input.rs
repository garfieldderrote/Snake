use std::{collections::VecDeque, thread::sleep, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{networking::InputPacket, util::Dir};

pub struct Input {
    pub should_quit: bool,
    pub input_buffer: VecDeque<char>,
    stream: TcpStream,
}

impl Input {
    pub fn clean() {
        _ = disable_raw_mode();
    }
    pub fn spawn_thread() {
        tokio::spawn(async move {
            if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9000").await {
                _ = enable_raw_mode();
                let mut input = Input {
                    should_quit: false,
                    input_buffer: VecDeque::new(),
                    stream,
                };
                loop {
                    sleep(Duration::from_millis(50));
                    input.fetch();
                    if let Some(dir) = input.get_dir() {
                        let packet = serde_json::to_string(&InputPacket::Direction { dir });
                        if packet.is_err() {
                            continue;
                        }
                        let packet = packet.unwrap();
                        _ = input.stream.write_all(packet.as_bytes()).await;
                        _ = input.stream.write_all(b"\n").await;
                    }
                }
            }
        });
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
