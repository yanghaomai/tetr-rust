use std::collections::HashMap;

use crate::{colr::*, constants::YCNT};
use lazy_static::lazy_static;

#[derive(Debug)]
pub struct ColDes {
    rot_id: u32,
    // start, len
    col: Vec<(u32, u32)>,
}
pub struct TetrBlk {
    pos: [i32; 4],
    rots: Vec<ColDes>,
    rot_ids: Vec<i32>,
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
    let r = bits.len();
    for i in 0..c {
        let mut idx = 0 as u32;
        for j in 0..r {
            //println!("{} {} {j}", r - j - 1, i);
            if bits[r - j - 1][i] == 1 {
                break;
            } else {
                idx += 1;
            }
        }
        assert!(r > idx as usize);
        let mut cnt = 0 as u32;
        for j in idx as usize..r {
            if bits[r - j - 1][i] == 1 {
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
        shapes.push((
            TetrColr::Purple,
            (vec![3, 4, 3, 3], vec![vec![0, 1, 0], vec![1, 1, 1]]),
            vec![0, 1, 2, 3],
        ));
        shapes.push((
            TetrColr::Red,
            (vec![3, 4, 3, 3], vec![vec![1, 1, 0], vec![0, 1, 1]]),
            vec![0, 1],
        ));
        shapes.push((TetrColr::Cyan, (vec![3, 5, 3, 5], vec![vec![1, 1, 1, 1]]),
        vec![0, 1],));
        shapes.push((
            TetrColr::Blue,
            (vec![3, 4, 3, 3], vec![vec![1, 0, 0], vec![1, 1, 1]]),
            vec![0, 1,2,3],
        ));
        shapes.push((
            TetrColr::Green,
            (vec![3, 4, 3, 3], vec![vec![0, 1, 1], vec![1, 1, 0]]),
            vec![0, 1],
        ));
        shapes.push((
            TetrColr::Orange,
            (vec![3, 4, 3, 3], vec![vec![0, 0, 1], vec![1, 1, 1]]),
            vec![0, 1,2,3],
        ));
        shapes.push((
            TetrColr::Yellow,
            (vec![4, 4, 4, 4], vec![vec![1, 1], vec![1, 1]]),
            vec![0],
        ));

        let mut mp = HashMap::new();
        for s in shapes.iter() {
            let mut rots = Vec::new();

            let mut sp = s.1 .1.clone();
            for r in 0..4 {
                rots.push(ColDes {
                    rot_id: r,
                    col: get_shape_col(&sp),
                });
                sp = get_clock_rot90(&sp);
            }
            let pos_vec: Result<[i32; 4], _> = s.1 .0.clone().try_into();
            //println!("{:?} {:?}", s.0, rots);
            mp.insert(
                s.0,
                TetrBlk {
                    rots,
                    pos: pos_vec.unwrap(),
                    rot_ids: s.2.clone(),
                },
            );
        }
        mp
    };
}

pub fn tetr_init_static() {
    lazy_static::initialize(&COLR2BLK);
}

pub fn get_start_pos(c: TetrColr, rot_idx: usize) -> i32 {
    let a = COLR2BLK.get(&c).unwrap();
    a.pos[rot_idx]
}

pub fn get_rot_ids(c: TetrColr) -> Vec<usize> {
    let a = COLR2BLK.get(&c).unwrap();
    a.rot_ids.iter().map(|x| *x as usize).collect()
}

#[derive(Debug, Clone)]
pub struct BitsDes {
    pub rd: Vec<BitsRowDes>,
    pub cd: Vec<BitsColDes>,
}

#[derive(Clone, Debug)]
pub struct BitsColDes {
    pub len: u32,
    pub cnt: u32,
    pub first_hole: i32,
}

#[derive(Clone, Debug)]
pub struct BitsRowDes {
    pub cnt: u32,
}

pub fn bits2des(bits: &Vec<Vec<bool>>) -> BitsDes {
    BitsDes {
        rd: bits2rowdes(bits),
        cd: bits2coldes(bits),
    }
}

fn bits2rowdes(bits: &Vec<Vec<bool>>) -> Vec<BitsRowDes> {
    let mut ret = Vec::new();
    let c = bits[0].len();
    let r = bits.len();
    for i in 0..r {
        let cnt = {
            let mut tmp = 0;
            for j in 0..c {
                if bits[r - 1 - i][j] {
                    tmp += 1;
                }
            }
            tmp
        };
        ret.push(BitsRowDes { cnt })
    }
    ret
}

fn bits2coldes(bits: &Vec<Vec<bool>>) -> Vec<BitsColDes> {
    let mut ret = Vec::new();
    let c = bits[0].len();
    let r = bits.len();
    for i in 0..c {
        let mut first_fill = None;
        let mut first_hole = None;
        let mut fill_cnt = 0;
        for j in 0..r {
            if bits[j][i] {
                if first_fill == None {
                    first_fill = Some(j);
                }
                fill_cnt += 1;
            } else {
                if first_fill != None && first_hole == None {
                    first_hole = Some(j);
                }
            }
        }
        ret.push(BitsColDes {
            len: match first_fill {
                None => 0,
                Some(x) => (r - x) as u32,
            },
            first_hole: match first_hole {
                None => 0,
                Some(x) => (r - x) as i32,
            },
            cnt: fill_cnt,
        });
    }
    ret
}

pub fn block_add(
    bd: &BitsDes,
    c: TetrColr,
    rot_idx: usize,
    pos_idx: usize,
) -> Option<(BitsDes, u32, usize)> {
    let cd = &bd.cd;
    let rd = &bd.rd;
    assert!(rot_idx < 4);
    assert!((pos_idx as usize) < cd.len());

    let col_des = &COLR2BLK.get(&c).unwrap().rots[rot_idx];
    let blk_cols = col_des.col.len();
    if blk_cols + pos_idx > cd.len() {
        return None;
    }
    let mut max_h = None;
    for i in 0..blk_cols {
        let col_pos = pos_idx + i;
        if cd[col_pos].len >= col_des.col[i].0 {
            if max_h.is_none() {
                max_h = Some(cd[col_pos].len - col_des.col[i].0);
            } else {
                max_h = Some(max_h.unwrap().max(cd[col_pos].len - col_des.col[i].0));
            }
        }
    }

    let mut cd = cd.clone();
    let mut rd = rd.clone();
    let max_h = max_h.unwrap();
    let mut remove_rows = Vec::new();
    let mut block_max_hight = 0;
    for i in 0..blk_cols {
        let col_pos = pos_idx + i;
        let one_col_des = col_des.col[i];
        //println!("BEFORE {:?}", mbits[col_pos]);
        cd[col_pos].cnt += one_col_des.1;
        cd[col_pos].len = max_h + one_col_des.0 + one_col_des.1;
        if cd[col_pos].len >= YCNT {
            return None;
        }
        //println!("AFTER {:?}", mbits[col_pos]);
        assert!(cd[col_pos].len >= cd[col_pos].cnt);
        for j in (max_h + one_col_des.0) as usize..cd[col_pos].len as usize {
            rd[j].cnt += 1;
            assert!(rd[j].cnt <= cd.len() as u32);
            if rd[j].cnt == cd.len() as u32 {
                remove_rows.push(j);
            }
        }

        block_max_hight = block_max_hight.max(cd[col_pos].len);
    }
    for i in cd.iter_mut() {
        i.len -= remove_rows.len() as u32;
        i.cnt -= remove_rows.len() as u32;
    }
    block_max_hight -= remove_rows.len() as u32;
    remove_rows.sort();
    remove_rows.reverse();
    for i in 1..remove_rows.len() {
        assert_ne!(remove_rows[i], remove_rows[i - 1]);
    }
    for i in remove_rows.iter() {
        rd.remove(*i);
        rd.push(BitsRowDes { cnt: 0 });
    }

    Some((BitsDes { rd, cd }, block_max_hight, remove_rows.len()))
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
