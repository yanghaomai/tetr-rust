#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

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
    DispatchMessageW, GetCursorPos, GetMessageW, MapVirtualKeyA, MapVirtualKeyW, TranslateMessage,
    UnhookWindowsHookEx, VkKeyScanA, KBDLLHOOKSTRUCT, MAPVK_VK_TO_CHAR, MAPVK_VK_TO_VSC, MSG,
    VK_ACCEPT, VK_DOWN, VK_END, VK_HOME, VK_LEFT, VK_RCONTROL, VK_RIGHT, VK_SPACE, VK_UP,
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

struct PsbMap {
    cd: Vec<BitsColDes>,
    rot_idx: usize,
    pos_idx: usize,
}

impl std::fmt::Debug for PsbMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PsbMap")
            .field("rot", &self.rot_idx)
            .field("pos", &self.pos_idx)
            .finish()
    }
}

fn get_all_possible(bd: &BitsDes, next_colr: TetrColr) -> Vec<PsbMap> {
    let mut all_possible = Vec::new();
    /*for i in brd.iter() {
        assert!(i.cnt <= i.len);
    }*/

    for rot_idx in 0..4usize {
        for pos_idx in 0..XCNT as usize {
            let add_info = block_add(&bd, next_colr, rot_idx, pos_idx);
            if add_info.is_none() {
                continue;
            } else {
                let cd = add_info.unwrap();
                /*for i in mbits.iter() {
                    assert!(i.cnt <= i.len);
                }*/
                all_possible.push(PsbMap {
                    cd,
                    rot_idx,
                    pos_idx,
                });
            }
        }
    }
    all_possible
}

fn get_best(bd: &BitsDes, next_colr: TetrColr) -> (usize, usize, bool) {
    let ap = get_all_possible(bd, next_colr);

    #[derive(PartialEq, Eq, Debug)]
    struct ApDes {
        idx: usize,
        hole_cnt: i32,
        max_hight: i32,
        total_hight: i32,
        hight_var: i32,
    }
    impl Ord for ApDes {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            /*
                self.hole_cnt
                    .cmp(&other.hole_cnt)
                    .then(self.max_hight.cmp(&other.max_hight))
                    .then(self.total_hight.cmp(&other.total_hight))
                    .then(self.hight_var.cmp(&other.hight_var))
            */
            if (self.max_hight - other.max_hight).abs() > 2 {
                self.max_hight.cmp(&other.max_hight)
            } else {
                self.hole_cnt
                    .cmp(&other.hole_cnt)
                    .then(self.max_hight.cmp(&other.max_hight))
                    .then(self.total_hight.cmp(&other.total_hight))
                    .then(self.hight_var.cmp(&other.hight_var))
            }
        }
    }

    impl PartialOrd for ApDes {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    let origin_cd = &bd.cd;
    let origin_rd = &bd.rd;
    let (origin_hole_cnt, origin_max_hight) = {
        let mut tmp1 = 0;
        let mut tmp2 = origin_cd[0].len;
        for x in origin_cd.iter() {
            assert!(x.len >= x.cnt);
            tmp1 += x.len - x.cnt;
            tmp2 = tmp2.max(x.len);
        }
        (tmp1 as i32, tmp2 as i32)
    };
    let mut ap_des = Vec::new();
    for (idx, x) in ap.iter().enumerate() {
        let mut hole_cnt = 0;
        let mut max_hight = x.cd[0].len;
        let mut total_hight = 0;
        for yy in x.cd.iter().enumerate() {
            let y = yy.1;
            //println!("{} {} {}", y.len, y.cnt, yy.0);
            hole_cnt += y.len - y.cnt;
            total_hight += y.len as i32;
            max_hight = max_hight.max(y.len);
        }
        let avg_hight = total_hight / x.cd.len() as i32;
        let hight_var = {
            let mut tmp = 0;
            for y in x.cd.iter() {
                tmp += (y.len as i32 - avg_hight).pow(2);
            }
            tmp
        };
        ap_des.push(ApDes {
            idx,
            hole_cnt: hole_cnt as i32,
            max_hight: max_hight as i32,
            total_hight,
            hight_var,
        });
    }
    ap_des.sort();
    /*for i in ap_des.iter() {
        println!("FUCK {:?} {:?}", i, ap[i.idx]);
    }*/
    let first_des = &ap_des[0];
    let idx = first_des.idx;

    let may_swap = if origin_hole_cnt != first_des.hole_cnt {
        true
    } else {
        false
    };

    let tmp = &ap[idx];
    (tmp.rot_idx, tmp.pos_idx, may_swap)
}

fn ascii_to_virtual_key(ascii_char: u8) -> i32 {
    // Convert ASCII character to virtual key code
    let ret = unsafe { VkKeyScanA(ascii_char as i8) as u16 };
    let vk = ret & 0xff;
    vk as i32
}

fn start_game(width: u32, height: u32, rx: &Receiver<CtrlInfo>) {
    let (mx, my) = get_mouse();
    let px = mx * 2;
    let py = my * 2;

    let scns = Screen::all().unwrap();
    let scn = scns[0];
    let img = scn.capture().unwrap();
    println!("{:?}", scn.display_info);
    println!("{} {}", img.len(), img.width() * img.height());
    //print_img(&img);
    let ss_width = img.width();
    let ss_height = img.height();
    assert!(ss_width % width == 0, "{ss_width} {width}");
    let radio = ss_width / width;

    // get len
    let ((cx, cy), len) = get_len(&img, px as i32, py as i32);

    let mut last_swap = false;
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
            println!("NEXT Colr {:?}", next_colr);
        } else {
            continue;
        }
        //print_img_bits(img, bits, cx, cy, len);
        let bd = bits2des(&bits);
        let (rot_idx, pos_idx, may_swap) = get_best(&bd, next_colr);
        if may_swap && last_swap == false {
            key_updown(VK_HOME);
            last_swap = true;
            continue;
        } else {
            last_swap = false;
        }
        println!("HHH {rot_idx} {pos_idx}");
        //continue;
        // do rot
        match rot_idx {
            0 => (),
            1 => {
                key_updown(VK_DOWN);
            }
            2 => {
                key_updown(VK_END);
            }
            3 => {
                key_updown(VK_UP);
            }
            _ => panic!(),
        }
        let start_pos = get_start_pos(next_colr, rot_idx);
        let right_move = pos_idx as i32 - start_pos;
        for _ in 0..right_move.abs() {
            key_updown(if right_move > 0 { VK_RIGHT } else { VK_LEFT });
        }
        key_updown(VK_SPACE);
        println!("{:?} done", next_colr);
    }
    println!("QUIT GAME");
}

fn main() {
    tetr_init_static();
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
