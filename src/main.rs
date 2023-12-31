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
fn is_black(p: &Rgba<u8>) -> bool {
    return p.0[0] < 10 && p.0[1] < 10 && p.0[2] < 10;
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

    let mut bits = vec![vec![false; XCNT as usize]; YCNT as usize];
    for i in 0..YCNT {
        for j in 0..XCNT {
            let pi = img.get_pixel(cx + j * len, cy + i * len);
            if is_black(pi) == false {
                bits[i as usize][j as usize] = true;
            }
        }
    }
    if true {
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
