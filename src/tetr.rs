use std::collections::HashMap;

use crate::colr::*;
use lazy_static::lazy_static;

pub struct ColDes {
    rot_id: u32,
    // start, len
    col: Vec<(u32, u32)>,
}
pub struct TetrBlk {
    rots: Vec<ColDes>,
}

fn get_clock_rot90(bits: &Vec<Vec<i32>>) -> Vec<Vec<i32>> {
    let mut ret = Vec::new();
    let cols = bits[0].len();
    let rows = bits.len();
    for i in 0..cols {
        let mut tmp = Vec::new();
        for j in (0..rows).rev() {
            tmp.push(bits[j][i]);
        }
        ret.push(tmp);
    }
    ret
}

fn get_shape_col(bits: &Vec<Vec<i32>>) -> Vec<(u32, u32)> {
    let mut ret = Vec::new();
    let c = bits[0].len();
    let r = bits[0].len();
    for i in 0..c {
        let mut idx = 0 as u32;
        for j in 0..r {
            if bits[r - j][i] == 1 {
                break;
            } else {
                idx += 1;
            }
        }
        assert!(r > idx as usize);
        let mut cnt = 0 as u32;
        for j in idx as usize..r {
            if bits[r - j][i] == 1 {
                cnt += 1;
            } else {
                break;
            }
        }
        ret.push((idx, cnt));
    }
    ret
}

lazy_static! {
    static ref COLR2BLK: HashMap<TetrColr, TetrBlk> = {
        let mut shapes = Vec::new();
        shapes.push((TetrColr::Purple, vec![vec![0, 1, 0], vec![1, 1, 1]]));
        shapes.push((TetrColr::Red, vec![vec![1, 1, 0], vec![0, 1, 1]]));
        shapes.push((TetrColr::Cyan, vec![vec![1, 1, 1, 1]]));
        shapes.push((TetrColr::Blue, vec![vec![1, 0, 0], vec![1, 1, 1]]));
        shapes.push((TetrColr::Green, vec![vec![0, 1, 1], vec![1, 1, 0]]));
        shapes.push((TetrColr::Orange, vec![vec![0, 0, 1], vec![1, 1, 1]]));
        shapes.push((TetrColr::Yellow, vec![vec![1, 1], vec![1, 1]]));

        let mut mp = HashMap::new();
        for s in shapes.iter() {
            let mut rots = Vec::new();

            let mut sp = s.1.clone();
            for r in 0..4 {
                rots.push(ColDes {
                    rot_id: r,
                    col: get_shape_col(&sp),
                });
                sp = get_clock_rot90(&sp);
            }
            mp.insert(s.0, TetrBlk { rots });
        }
        mp
    };
}

#[derive(Clone)]
pub struct BitsRowDes {
    len: u32,
    cnt: u32,
}

pub fn bits2RowDes(bits: &Vec<Vec<bool>>) -> Vec<BitsRowDes> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape_col_test() {
        for x in COLR2BLK.iter() {
            assert!(x.1.rots.len() == 4);
            // check 0
            for y in x.1.rots.iter() {
                let mut cont_0 = false;
                for z in y.col.iter() {
                    if z.0 == 0 {
                        cont_0 = true;
                    }
                }
                assert!(cont_0);
            }
        }
    }
}
