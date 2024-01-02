#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

//use lazy_static
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

mod colr;
mod constants;
mod pic;
mod screen;
mod tetr;
use colr::*;
use constants::*;
use pic::*;
use screen::*;
use tetr::*;

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
    if n_code >= 0 {
        let kbd_struct = unsafe { *(l_param as *const KBDLLHOOKSTRUCT) };
        let mapped_char =
            unsafe { MapVirtualKeyW(kbd_struct.vkCode as u32, MAPVK_VK_TO_CHAR) as u8 as char };
        if kbd_struct.flags == 0 && mapped_char.is_ascii_graphic() {
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
    let hook_handle;
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

fn get_all_possible(
    bits: &Vec<Vec<bool>>,
    next_colr: TetrColr,
) -> Vec<(Vec<BitsRowDes>, usize, usize)> {
    let mut all_possible = Vec::new();

    let brd = bits2rowdes(bits);

    for rot_idx in 0..3usize {
        for pos_idx in 0..XCNT as usize {
            let mbits = block_add(&brd, next_colr, rot_idx, pos_idx);
            if mbits.is_none() {
                continue;
            } else {
                all_possible.push((mbits.unwrap(), rot_idx, pos_idx));
            }
        }
    }
    all_possible
}

fn get_best(bits: &Vec<Vec<bool>>, next_colr: TetrColr) -> (usize, usize) {
    let ap = get_all_possible(bits, next_colr);

    #[derive(PartialEq, Eq)]
    struct ApDes {
        idx: usize,
        hole_cnt: u32,
        max_h: u32,
    }
    impl Ord for ApDes {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // 首先按照年龄排序
            self.hole_cnt
                .cmp(&other.hole_cnt)
                .then(self.max_h.cmp(&other.max_h))
        }
    }

    impl PartialOrd for ApDes {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    let mut ap_des = Vec::new();
    for x in ap.iter().enumerate() {
        let mut hole_cnt = 0;
        let mut max_h = x.1 .0[0].len;
        for y in x.1 .0.iter() {
            hole_cnt += y.len - y.cnt;
            max_h = max_h.max(y.len);
        }
        ap_des.push(ApDes {
            idx: x.0,
            hole_cnt,
            max_h,
        });
    }
    ap_des.sort_unstable();
    let tmp = &ap[ap_des[0].idx];
    (tmp.1, tmp.2)
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
        //print_img_bits(img, bits, cx, cy, len);

        let (rot_idx, pos_idx) = get_best(&bits, next_colr);

        // do rot
        match rot_idx {
            0 => (),
            1 => {
                key_updown(b'd');
            }
            2 => {
                key_updown(b'w');
            }
            3 => {
                key_updown(b'a');
            }
            _ => panic!(),
        }
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
