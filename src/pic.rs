use crate::colr::*;
use crate::constants::*;
use crate::screen::print_img;
use screenshots::image::{Pixel, Rgba, RgbaImage};

fn get_block_xy_len(img: &RgbaImage, px: i32, py: i32) -> (u32, u32, u32) {
    const XY: [[i32; 2]; 4] = [[0, 1], [0, -1], [1, 0], [-1, 0]];
    let mut xy_min = [px, py];
    let mut xy_max = [px, py];

    let op = img.get_pixel(px as u32, py as u32);
    for xy in &XY {
        let mut st = [px, py];
        while let Some(p) = img.get_pixel_checked(st[0] as u32, st[1] as u32) {
            if p != op {
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
    if (a - b).abs() >= 4 {
        println!("WRONG abs");
        print_img(&img);
    }
    assert!((a - b).abs() < 4, "{} {a} {b}", (a - b).abs());
    let mx = (xy_min[0] + xy_max[0]) as u32 / 2;
    let my = (xy_min[1] + xy_max[1]) as u32 / 2;
    let len = (a + b) as u32 / 2;
    (mx, my, len)
}

pub fn get_len(img: &RgbaImage, px: i32, py: i32) -> ((u32, u32), u32) {
    const UP_BLOCK_CNT: u32 = 6;
    let (mx, my, len) = get_block_xy_len(img, px, py);
    let (mx1, my1, len1) = get_block_xy_len(img, mx as i32, (my - len * UP_BLOCK_CNT) as i32);
    let len = (my - my1) / UP_BLOCK_CNT;

    if false {
        let mut img = img.clone();
        let wp = *Rgba::from_slice(&[0xff, 0, 0xff, 0xff]);
        for i in 0..XCNT {
            for j in 0..YCNT {
                let mx = mx + i * len;
                let my = my - (YCNT - 1) * len + j * len;

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
    ((mx, my - (YCNT - 1) * len), len)
}

pub fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, p: Rgba<u8>) {
    const LEN: u32 = 6;
    for i in (x - LEN)..=(x + LEN) {
        for j in (y - LEN)..=(y + LEN) {
            *img.get_pixel_mut(i, j) = p;
        }
    }
}

pub fn get_current_pic(
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
            if i < TOP_LINE as u32 {
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
