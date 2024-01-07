use std::collections::btree_map::Range;
use std::collections::HashMap;

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
    if (a - b).abs() >= 3 {
        println!("WRONG abs");
        print_img(&img);
    }
    //print_img(&img);
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
    if false {
        let mut img = img.clone();
        let colr = Rgba([255, 0, 255, 255]);
        for i in 0..XCNT {
            for j in 0..YCNT {
                draw_rect(&mut img, mx + len * i, my + len * j, colr);
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

pub fn draw_x_line<I>(img: &mut RgbaImage, x: I, y: u32, p: Rgba<u8>)
where
    I: Iterator<Item = u32>,
{
    const LEN: u32 = 6;
    for i in x {
        for j in (y - LEN)..=(y + LEN) {
            *img.get_pixel_mut(i, j) = p;
        }
    }
}

pub fn draw_y_line<I>(img: &mut RgbaImage, x: u32, y: I, p: Rgba<u8>)
where
    I: Iterator<Item = u32>,
{
    const LEN: u32 = 6;
    for i in y {
        for j in (x - LEN)..=(x + LEN) {
            *img.get_pixel_mut(j, i) = p;
        }
    }
}

pub fn get_current_pic(
    img: &RgbaImage,
    cx: u32,
    cy: u32,
    len: u32,
) -> Option<(Vec<Vec<bool>>, Vec<TetrColr>)> {
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
                if top_line_colr_cnt != 4 {
                    return None;
                }
                if is_black(pi) == false {
                    bits[i as usize][j as usize] = true;
                }
            }
            if is_black(pi) == false {}
        }
    }
    if next_colr == None || top_line_colr_cnt != 4 {
        return None;
    }
    let next_colr = next_colr.unwrap();

    const NEXT_START_Y: f32 = 2.5;
    const NEXT_END_Y: f32 = 17.0;
    const NEXT_START_X: u32 = XCNT + 1;
    const NEXT_END_X: u32 = NEXT_START_X + 3;

    let next_start_y = cy + (len as f32 * NEXT_START_Y) as u32;
    let next_end_y = cy + (len as f32 * NEXT_END_Y) as u32;
    let next_start_x = cx + len * NEXT_START_X;
    let next_end_x = cx + len * NEXT_END_X;

    const NEXT_CNT: u32 = 5;
    let one_y_len = (next_end_y - next_start_y) / NEXT_CNT;
    let one_x_len = next_end_x - next_start_x;
    let start_y: Vec<u32> = (0..5).map(|x| one_y_len * x + next_start_y).collect();

    let mut next_colr_vec = vec![get_color(&next_colr).unwrap()];

    const X_SAMPLE: u32 = 3;
    const Y_SAMPLE: u32 = 3;
    let x_gap = one_x_len / (X_SAMPLE + 1);
    let y_gap = one_y_len / (Y_SAMPLE + 1);

    //let mut img = img.clone();
    //let lc = Rgba([255, 0, 0, 255]);

    for (idx, sy) in start_y.iter().enumerate() {
        let mut colr = HashMap::new();
        for i in 1..=X_SAMPLE {
            for j in 1..=Y_SAMPLE {
                let x = next_start_x + i * x_gap;
                let y = sy + j * y_gap;
                //draw_rect(&mut img, x, y, lc);
                let p = img.get_pixel(x, y);
                let c = get_color(&p).unwrap();
                if c == TetrColr::Black || c == TetrColr::Gray {
                    continue;
                }
                let entry = colr.entry(c).or_insert(0);
                *entry += 1;
            }
        }
        let mut colr: Vec<(i32, TetrColr)> = colr.iter().map(|x| (*x.1, *x.0)).collect();
        colr.sort();
        next_colr_vec.push(colr[colr.len() - 1].1);
    }

    Some((bits, next_colr_vec))
}
