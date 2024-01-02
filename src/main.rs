use minifb::{Key, Window, WindowOptions};
use screenshots::image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use screenshots::Screen;
use std::cmp::{max, min};
use std::ffi::{c_char, CStr};
use std::sync::mpsc::{self, Receiver};
use std::thread::sleep;
use std::time::Instant;
use std::{mem, thread};
use std::{ptr, time::Duration};
use winapi::shared::minwindef::DWORD;
use winapi::shared::windef::POINT;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use winapi::um::wingdi::BITSPIXEL;
use winapi::um::winuser::{
    DispatchMessageW, GetCursorPos, GetMessageW, MapVirtualKeyW, TranslateMessage,
    UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MAPVK_VK_TO_CHAR, MSG,
};
use winapi::{
    shared::{
        minwindef::{LPARAM, LRESULT, WPARAM},
        windef::HHOOK,
    },
    um::libloaderapi::GetModuleHandleW,
    um::winuser::{self, CallNextHookEx, WH_KEYBOARD_LL},
};

fn get_size() -> (u32, u32) {
    unsafe {
        (
            winuser::GetSystemMetrics(0) as u32,
            winuser::GetSystemMetrics(1) as u32,
        )
    }
}

enum CtrlInfo {
    Start,
    Quit,
    Exit,
}

static mut SENDER: Option<mpsc::Sender<CtrlInfo>> = None;

extern "system" fn keyboard_hook_callback(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if (n_code >= 0) {
        let kbd_struct = unsafe { *(l_param as *const KBDLLHOOKSTRUCT) };
        let mapped_char =
            unsafe { MapVirtualKeyW(kbd_struct.vkCode as u32, MAPVK_VK_TO_CHAR) as u8 as char };
        if (kbd_struct.flags == 0 && mapped_char.is_ascii_graphic()) {
            if let Some(tx) = unsafe { &SENDER } {
                match mapped_char {
                    '1' => {
                        println!("START GAME!");
                        tx.send(CtrlInfo::Start).unwrap();
                    }
                    '2' => {
                        println!("GAME OVER!");
                        tx.send(CtrlInfo::Quit).unwrap();
                    }
                    '3' => {
                        println!("EXIT");
                        tx.send(CtrlInfo::Exit).unwrap();
                    }
                    _ => (),
                }
            } else {
                panic!("Something wrong");
            }
        }
    }
    unsafe { CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param) }
}

fn kb_func() {
    let mut hook_handle: HHOOK = ptr::null_mut();
    unsafe {
        let h_instance = GetModuleHandleW(ptr::null());
        hook_handle =
            winuser::SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook_callback), h_instance, 0);
    }
    // 消息循环，使钩子生效
    unsafe {
        let mut msg: MSG = mem::zeroed();
        loop {
            if GetMessageW(&mut msg, ptr::null_mut(), 0, 0) == 0 {
                break;
            } else {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        UnhookWindowsHookEx(hook_handle);
    }
    println!("KBHandler exit");
}

fn get_mouse() -> (u32, u32) {
    let mut c: POINT = unsafe { mem::zeroed() };
    unsafe { GetCursorPos(&mut c) };
    (c.x as u32, c.y as u32)
}

fn print_img(img: &RgbaImage) {
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

const XCNT: u32 = 10;
const YCNT: u32 = 20;
const LEN_FIX: u32 = 6;
const X_FIX: u32 = 3;
const Y_FIX: u32 = 3;
const TOP_LINE: usize = 3;

fn get_len(img: &RgbaImage, px: i32, py: i32) -> ((u32, u32), u32) {
    const XY: [[i32; 2]; 4] = [[0, 1], [0, -1], [1, 0], [-1, 0]];
    let mut xy_min = [px, py];
    let mut xy_max = [px, py];

    let op = img.get_pixel(px as u32, py as u32);
    for xy in &XY {
        let mut st = [px, py];
        while let Some(p) = img.get_pixel_checked(st[0] as u32, st[1] as u32) {
            if (p != op) {
                break;
            }
            st[0] += xy[0];
            st[1] += xy[1];
        }
        for i in 0..2 {
            xy_min[i] = xy_min[i].min(st[i]);
            xy_max[i] = xy_max[i].max(st[i]);
        }
    }
    let a = xy_max[0] - xy_min[0];
    let b = xy_max[1] - xy_min[1];

    //println!("HHH{a} {b}");
    assert!((a - b).abs() < 2);
    let mx = (xy_min[0] + xy_max[0]) as u32 / 2 - X_FIX;
    let my = (xy_min[1] + xy_max[1]) as u32 / 2 - Y_FIX;
    let len = (a + b) as u32 / 2 + LEN_FIX;

    if false {
        let mut img = img.clone();
        let wp = *Rgba::from_slice(&[0xff, 0, 0xff, 0xff]);
        for i in xy_min[0]..=xy_max[0] {
            for j in 0..2 {
                *img.get_pixel_mut(i as u32, j + xy_min[1] as u32) = wp;
                *img.get_pixel_mut(i as u32, j + xy_max[1] as u32) = wp;
            }
        }
        for i in xy_min[1]..=xy_max[1] {
            for j in 0..2 {
                *img.get_pixel_mut(j + xy_min[0] as u32, i as u32) = wp;
                *img.get_pixel_mut(j + xy_max[0] as u32, i as u32) = wp;
            }
        }
        for i in 0..XCNT {
            for j in 0..YCNT {
                let mx = mx + i * len;
                let my = my + j * len;

                *img.get_pixel_mut(mx, my) = wp;
                *img.get_pixel_mut(mx + 1, my) = wp;
                *img.get_pixel_mut(mx, my + 1) = wp;
                *img.get_pixel_mut(mx + 1, my + 1) = wp;

                *img.get_pixel_mut(mx, my) = wp;
                *img.get_pixel_mut(mx - 1, my) = wp;
                *img.get_pixel_mut(mx, my - 1) = wp;
                *img.get_pixel_mut(mx - 1, my - 1) = wp;
            }
        }
        print_img(&img);
    }
    ((mx, my), len)
}

fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, p: Rgba<u8>) {
    const LEN: u32 = 6;
    for i in (x - LEN)..=(x + LEN) {
        for j in (y - LEN)..=(y + LEN) {
            *img.get_pixel_mut(i, j) = p;
        }
    }
}

#[derive(Debug, PartialEq)]
enum TetrColr {
    Purple,
    Red,
    Cyan,
    Blue,
    Green,
    Orange,
    Yellow,
    Black,
    Gray,
}
#[derive(Debug)]
struct ColrDes {
    c: TetrColr,
    name: &'static str,
    center: Rgba<u8>,
}

const COLR_ARR: [ColrDes; 8] = [
    ColrDes {
        c: TetrColr::Purple,
        name: "purple", //T
        center: Rgba([206, 58, 192, 255]),
    },
    ColrDes {
        c: TetrColr::Red,
        name: "red", //Z
        center: Rgba([227, 43, 53, 255]),
    },
    ColrDes {
        c: TetrColr::Cyan,
        name: "cyan", //I
        center: Rgba([41, 227, 158, 255]),
    },
    ColrDes {
        c: TetrColr::Blue,
        name: "blue", //J
        center: Rgba([83, 58, 206, 255]),
    },
    ColrDes {
        c: TetrColr::Green,
        name: "green", //S
        center: Rgba([157, 227, 41, 255]),
    },
    ColrDes {
        c: TetrColr::Orange,
        name: "orage", //L
        center: Rgba([227, 112, 40, 255]),
    },
    ColrDes {
        c: TetrColr::Yellow,
        name: "yellow", //O
        center: Rgba([227, 190, 41, 255]),
    },
    ColrDes {
        c: TetrColr::Black,
        name: "Black",
        center: Rgba([0, 0, 0, 255]),
    },
];

fn get_color(p: &Rgba<u8>) -> Option<TetrColr> {
    let mut min_dis: Option<(i32, TetrColr)> = None;
    for x in COLR_ARR {
        let mut dis = 0;
        for i in 0..3 {
            let a = p.0[i] as i32;
            let b = x.center.0[i] as i32;
            dis += (a - b).abs().pow(2);
        }

        if let Some(md) = &mut min_dis {
            if md.0 > dis {
                *md = (dis, x.c);
            }
        } else {
            min_dis = Some((dis, x.c));
        }
    }
    match min_dis {
        None => None,
        Some(md) => Some(md.1),
    }
}

fn is_black(p: &Rgba<u8>) -> bool {
    //return p.0[0] < 10 && p.0[1] < 10 && p.0[2] < 10;
    get_color(p).unwrap() == TetrColr::Black
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_test() {
        let t = Rgba([227, 190, 41, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Yellow);

        let t = Rgba([206, 59, 192, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Purple);

        let t = Rgba([227, 45, 53, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Red);
        let t = Rgba([41, 227, 158, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Cyan);

        let t = Rgba([83, 58, 207, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Blue);

        let t = Rgba([155, 227, 41, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Green);

        let t = Rgba([223, 112, 40, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Orange);

        let t = Rgba([1, 2, 1, 255]);
        assert_eq!(get_color(&t).unwrap(), TetrColr::Black);
    }
}

fn get_current_pic(
    img: &RgbaImage,
    cx: u32,
    cy: u32,
    len: u32,
) -> Option<(Vec<Vec<bool>>, TetrColr)> {
    let mut bits = vec![vec![false; XCNT as usize]; YCNT as usize];
    let mut next_colr = None;
    let mut top_line_colr_cnt = 0;
    for i in 0..YCNT {
        for j in 0..XCNT {
            let pi = img.get_pixel(cx + j * len, cy + i * len);
            if i < TOP_LINE.try_into().unwrap() {
                if is_black(pi) == false {
                    if next_colr == None {
                        next_colr = Some(*pi);
                    }
                    top_line_colr_cnt += 1;
                }
            } else {
                if is_black(pi) == false {
                    bits[i as usize][j as usize] = true;
                }
            }
            if is_black(pi) == false {}
        }
    }
    if next_colr == None || top_line_colr_cnt != 4 {
        None
    } else {
        let next_colr = next_colr.unwrap();
        Some((bits, get_color(&next_colr).unwrap()))
    }
}

fn get_all_possible(bits: &Vec<Vec<bool>>, next_colr: TetrColr) -> Vec<(Vec<Vec<bool>>, u32, u32)> {
    let mut all_possible = Vec::new();
    for rot_idx in 0..3u32 {
        for pos_idx in 0..XCNT {
            let mut mbits = bits.clone();

            all_possible.push((mbits, rot_idx, pos_idx));
        }
    }
    all_possible
}
fn get_best(bits: &Vec<Vec<bool>>, next_colr: TetrColr) {
    let ap = get_all_possible(bits, next_colr);
}

fn start_game(width: u32, height: u32, rx: &Receiver<CtrlInfo>) {
    let (mx, my) = get_mouse();
    let px = mx * 2;
    let py = my * 2;

    let scns = Screen::all().unwrap();
    let scn = scns[0];
    let img = scn.capture().unwrap();
    let ss_width = img.width();
    let ss_height = img.height();
    assert!(ss_width % width == 0);
    let radio = ss_width / width;

    // get len
    let ((cx, cy), len) = get_len(&img, px as i32, py as i32);

    loop {
        if let Ok(ci) = rx.try_recv() {
            match ci {
                CtrlInfo::Quit => break,
                _ => println!("Ingore KB"),
            }
        }
        let img = scn.capture().unwrap();
        let bits;
        let next_colr;
        if let Some(ret) = get_current_pic(&img, cx, cy, len) {
            (bits, next_colr) = ret;
        } else {
            continue;
        }

        if false {
            let mut img = img.clone();
            for i in 0..YCNT {
                for j in 0..XCNT {
                    if (bits[i as usize][j as usize] == false) {
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
        get_best(&bits, next_colr);
    }
    println!("QUIT GAME");
}

fn main() {
    let (width, height) = get_size();
    println!("Screen {width}x{height}");

    //create mpsc
    let (tx, rx) = mpsc::channel::<CtrlInfo>();
    unsafe {
        SENDER = Some(tx);
    }

    let kb_handler = thread::Builder::new()
        .name("KBHandler".into())
        .spawn(kb_func)
        .unwrap();

    loop {
        if let Ok(ci) = rx.try_recv() {
            match ci {
                CtrlInfo::Start => {
                    start_game(width, height, &rx);
                }
                CtrlInfo::Quit => {}
                CtrlInfo::Exit => break,
            }
        }
    }
    kb_handler.join().unwrap();
}
