use std::mem;

use minifb::{Key, Window, WindowOptions};
use screenshots::image::{Pixel, Rgba, RgbaImage};
use winapi::{
    shared::windef::POINT,
    um::winuser::{self, GetCursorPos},
};

use crate::constants::*;
use crate::pic::draw_rect;

pub fn print_img(img: &RgbaImage) {
    //force little endian
    if cfg!(target_endian = "little") == false {
        panic!();
    }
    let radio = 2;
    let w = img.width() as usize;
    let h = img.height() as usize;

    let mut win = Window::new("Display", w / radio, h / radio, WindowOptions::default()).unwrap();

    let buffer: Vec<u32> = img
        .pixels()
        .into_iter()
        .enumerate()
        .filter(|(i, _)| i % radio == 0 && (i / w) % radio == 0)
        .map(|(_, x)| u32::from_le_bytes(x.0))
        .collect();
    println!("{} {w} {h}", buffer.len());
    while win.is_open() && !win.is_key_down(Key::Escape) {
        win.update_with_buffer(&buffer, w / radio, h / radio)
            .unwrap();
    }
}

pub fn get_mouse() -> (u32, u32) {
    let mut c: POINT = unsafe { mem::zeroed() };
    unsafe { GetCursorPos(&mut c) };
    (c.x as u32, c.y as u32)
}

pub fn get_size() -> (u32, u32) {
    unsafe {
        (
            winuser::GetSystemMetrics(0) as u32,
            winuser::GetSystemMetrics(1) as u32,
        )
    }
}

pub fn print_img_bits(img: RgbaImage, bits: Vec<Vec<bool>>, cx: u32, cy: u32, len: u32) {
    let mut img = img.clone();
    for i in 0..YCNT {
        for j in 0..XCNT {
            if bits[i as usize][j as usize] == false {
                draw_rect(
                    &mut img,
                    cx + j * len,
                    cy + i * len,
                    *Rgba::from_slice(&[0xff, 0xff, 0xff, 0xff]),
                );
            }
        }
    }
    print_img(&img);
}
