#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use minifb::{Key, Window, WindowOptions};
use rand::Rng;
use screenshots::image::{ImageBuffer, Pixel, Rgba, RgbaImage};
use screenshots::Screen;
use std::cell::RefCell;
use std::cmp::{max, min, Reverse};
use std::collections::BinaryHeap;
use std::ffi::{c_char, CStr};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
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

static quit_flag: Mutex<bool> = Mutex::new(false);

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
            {
                let qf = quit_flag.lock().unwrap();
                if *qf {
                    break;
                }
            }

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

#[derive(Debug)]
struct PsbMap {
    bd: BitsDes,
    ops: Vec<(usize, usize)>,
    block_max_hight: u32,
    remove_rows: usize,
}

fn get_all_possible(bd: &BitsDes, next_colr: &[TetrColr]) -> Vec<PsbMap> {
    let mut ret = Vec::new();

    let ap;
    if next_colr.len() == 1 {
        ap = vec![PsbMap {
            bd: bd.clone(),
            ops: Vec::new(),
            block_max_hight: 0,
            remove_rows: 0,
        }];
    } else {
        ap = get_all_possible(bd, &next_colr[..next_colr.len() - 1]);
    }

    let nc = next_colr[next_colr.len() - 1];
    for x in ap.iter() {
        let rot_ids = get_rot_ids(nc);
        for rot_idx in rot_ids.iter() {
            for pos_idx in 0..XCNT as usize {
                let add_info = block_add(&x.bd, nc, *rot_idx, pos_idx);
                if add_info.is_none() {
                    continue;
                } else {
                    let mut ops = x.ops.clone();
                    ops.push((*rot_idx, pos_idx));
                    let add_info = add_info.unwrap();
                    ret.push(PsbMap {
                        bd: add_info.0,
                        ops,
                        block_max_hight: x.block_max_hight.max(add_info.1),
                        remove_rows: x.remove_rows + add_info.2,
                    });
                }
            }
        }
    }
    ret
}

fn get_best(
    bd: &BitsDes,
    next_colr: &[TetrColr],
    hold_cyan: bool,
    can_swap: bool,
) -> (Vec<(usize, usize)>, bool) {
    // check can be insert cyan
    #[derive(PartialEq)]
    struct InsPos {
        rot_idx: usize,
        pos_idx: usize,
        fi: u32,
    }
    let mut insert_pos = None;
    {
        let f = |i: i32| {
            if i < 0 || i == XCNT as i32 {
                YCNT
            } else {
                bd.cd[i as usize].len
            }
        };
        for i in 0..XCNT as i32 {
            if f(i) + 3 <= f(i - 1) && f(i) + 3 <= f(i + 1) {
                match insert_pos {
                    None => {
                        insert_pos = Some(InsPos {
                            rot_idx: 1,
                            pos_idx: i as usize,
                            fi: f(i),
                        })
                    }
                    Some(ref mut ip) if ip.fi > f(i) => {
                        *ip = InsPos {
                            rot_idx: 1,
                            pos_idx: i as usize,
                            fi: f(i),
                        }
                    }
                    Some(_) => {}
                }
            }
        }
    }
    if next_colr[0] == TetrColr::Cyan {
        match insert_pos {
            None if hold_cyan == false && can_swap => return (Vec::new(), true),
            None => {}
            Some(ref ip) => return (vec![(ip.rot_idx, ip.pos_idx)], false),
        }
    } else if hold_cyan && can_swap {
        if insert_pos != None {
            return (Vec::new(), true);
        }
    }
    //println!("PASSSSSSSSS");

    if next_colr[0] == TetrColr::Cyan {}
    let ap = get_all_possible(bd, next_colr);

    #[derive(PartialEq, Eq, Debug)]
    struct ApDes {
        idx: usize,
        hole_cnt: i32,
        max_hight: i32,
        total_hight: i32,
        hight_var: i32,
        block_max_hight: i32,
        round_len: i32,
        remove_rows: usize,
        hole_dis: i32,
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
            /*if (self.max_hight - other.max_hight).abs() > 1 {
                self.max_hight.cmp(&other.max_hight)
            } else */
            if (self.block_max_hight - other.block_max_hight).abs() > 2 {
                self.block_max_hight.cmp(&other.block_max_hight)
            } else {
                self.hole_cnt
                    .cmp(&other.hole_cnt)
                    .then(other.remove_rows.cmp(&self.remove_rows))
                    .then(self.block_max_hight.cmp(&other.block_max_hight))
                    .then(self.hole_dis.cmp(&other.hole_dis))
                    //.then(self.max_hight.cmp(&other.max_hight))
                    .then(self.round_len.cmp(&other.round_len))
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

    //cal min hole dis
    let mut min_hole_dis = XCNT as i32;
    {
        for x in bd.cd.iter() {
            min_hole_dis = min_hole_dis.min(x.len as i32 - x.first_hole);
        }
    }

    //let mut ap_des = Vec::new();
    let mut ap_des = BinaryHeap::new();
    for (idx, x) in ap.iter().enumerate() {
        let mut hole_cnt = 0;
        let mut max_hight = x.bd.cd[0].len;
        let mut total_hight = 0;
        for yy in x.bd.cd.iter().enumerate() {
            let y = yy.1;
            //println!("{} {} {}", y.len, y.cnt, yy.0);
            hole_cnt += y.len - y.cnt;
            total_hight += y.len as i32;
            max_hight = max_hight.max(y.len);
        }
        let mut round_len = 0;
        for i in 1..x.bd.cd.len() {
            round_len += (x.bd.cd[i].len as i32 - x.bd.cd[i - 1].len as i32).abs();
        }
        let avg_hight = total_hight / x.bd.cd.len() as i32;
        let hight_var = {
            let mut tmp = 0;
            for y in x.bd.cd.iter() {
                tmp += (y.len as i32 - avg_hight).pow(2);
            }
            tmp
        };
        let mut hole_dis = XCNT as i32;
        if x.remove_rows == 0 {
            for x in x.bd.cd.iter() {
                hole_dis = hole_dis.min(x.len as i32 - x.first_hole);
            }
        }
        ap_des.push(Reverse(ApDes {
            idx,
            hole_cnt: hole_cnt as i32,
            max_hight: max_hight as i32,
            total_hight,
            hight_var,
            block_max_hight: x.block_max_hight as i32,
            round_len,
            remove_rows: x.remove_rows,
            hole_dis: (hole_dis - min_hole_dis).abs(),
        }));
    }
    /*for i in ap_des.iter() {
        println!("FUCK {:?} {:?}", i, ap[i.idx]);
    }*/
    let first_des = &ap_des.peek().unwrap().0;
    let idx = first_des.idx;

    /* //old may swap
    let may_swap = if origin_hole_cnt < first_des.hole_cnt {
        //println!("ORIG {} NOW {}", origin_hole_cnt, first_des.hole_cnt);
        true
    } else {
        false
    };*/

    //println!("{:?}", first_des);
    //println!("{:#?}", ap[idx]);

    let ops = ap[idx].ops.clone();
    //ops.reverse();
    (ops, false)
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
    let mut img = scn.capture().unwrap();
    println!("{:?}", scn.display_info);
    println!("{} {}", img.len(), img.width() * img.height());
    //print_img(&img);
    let ss_width = img.width();
    let ss_height = img.height();
    assert!(ss_width % width == 0, "{ss_width} {width}");
    let radio = ss_width / width;

    // get len
    let ((cx, cy), len) = get_len(&img, px as i32, py as i32);

    thread_local! {
        static  RNG : RefCell<rand::rngs::ThreadRng>= RefCell::new(rand::thread_rng());
    }
    let gen = || RNG.with(|rng| rng.borrow_mut().gen_range(0..50));

    let mut last_swap = false;
    let mut hold_cyan = false;
    let mut cap_img = false;
    loop {
        if cap_img == false {
            img = scn.capture().unwrap();
            //cap_img = true;
        }
        cap_img = false;
        if let Ok(ci) = rx.try_recv() {
            match ci {
                CtrlInfo::Quit => break,
                _ => println!("Ingore KB"),
            }
        }
        let bits;
        let next_colrs;
        if let Some(ret) = get_current_pic(&img, cx, cy, len) {
            (bits, next_colrs) = ret;
            //println!("NEXT Colr {:?}", next_colrs);
        } else {
            continue;
        }
        const MAX_NEXT: usize = 3;
        let mut next_cnt;
        if next_colrs[0] == TetrColr::Cyan {
            next_cnt = 1usize;
        } else {
            next_cnt = 1;
            for i in 1..=MAX_NEXT {
                if next_colrs[i - 1] == TetrColr::Cyan {
                    break;
                }
                next_cnt = i;
            }
        }
        //print_img_bits(img, bits, cx, cy, len);
        let bd = bits2des(&bits);
        let fm;
        {
            let mut max_high = 0;
            for x in bd.cd.iter() {
                max_high = max_high.max(x.len);
            }
            fm = max_high > 2;
            //fm = true;
        }
        let (ops, may_swap) =
            get_best(&bd, &next_colrs[0..next_cnt], hold_cyan, last_swap == false);
        if may_swap && last_swap == false {
            key_updown(VK_HOME, fm);
            last_swap = true;
            hold_cyan = next_colrs[0] == TetrColr::Cyan;
            //println!("SWAP! {hold_cyan}");
            continue;
        } else {
            last_swap = false;
        }
        assert!(ops.len() != 0);

        let ops_cnt = ops.len();
        for (idx, (rot_idx, pos_idx)) in ops.iter().enumerate() {
            //println!("HHH {rot_idx} {pos_idx}");
            match rot_idx {
                0 => (),
                1 => {
                    key_updown(VK_DOWN, fm);
                }
                2 => {
                    key_updown(VK_END, fm);
                }
                3 => {
                    key_updown(VK_UP, fm);
                }
                _ => panic!(),
            }
            let start_pos = get_start_pos(next_colrs[idx], *rot_idx);
            let right_move = *pos_idx as i32 - start_pos;
            for _ in 0..right_move.abs() {
                key_updown(if right_move > 0 { VK_RIGHT } else { VK_LEFT }, fm);
            }
            key_updown(VK_SPACE, fm);
            if ops_cnt == idx + 1 {
                img = scn.capture().unwrap();
                cap_img = true;
            }

            if fm {
                //sleep(Duration::from_millis(30));
            } else {
                sleep(Duration::from_millis(40));
            }
        }
        //sleep(Duration::from_millis(30));
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
    {
        let mut qf = quit_flag.lock().unwrap();
        *qf = true;
    }
    kb_handler.join().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_best_test() {
        let mut cd = Vec::new();
        let mut rd = Vec::new();
        for _ in 0..XCNT {
            cd.push(BitsColDes {
                len: 0,
                cnt: 0,
                first_hole: YCNT as i32,
            })
        }
        for _ in 0..YCNT {
            rd.push(BitsRowDes { cnt: 0 });
        }
        let (ops, may_hold) = get_best(
            &BitsDes { cd, rd },
            &[TetrColr::Green, TetrColr::Orange],
            false,
            false,
        );
        assert!(false, "{:?} {may_hold}", ops);
    }
}
