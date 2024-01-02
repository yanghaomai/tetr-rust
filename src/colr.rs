use lazy_static::lazy_static;
use screenshots::image::Rgba;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TetrColr {
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

pub fn get_color(p: &Rgba<u8>) -> Option<TetrColr> {
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

pub fn is_black(p: &Rgba<u8>) -> bool {
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

        for x in COLR_ARR.iter().enumerate() {
            assert_eq!(x.0, x.1.c as usize);
        }
    }
}
