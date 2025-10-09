/*
 * Copyright (C) 2025 Phosh.mobi e.V.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Author: Arun Mani J <arun.mani@tether.to>
 */

/*
 * Transformation functions to scale a rectangle `src` without disturbing the aspect-ratio.
 *
 * All the functions take arguments `src_w`, `src_h`, `dst_w`, `dst_h` and return a struct with
 * values `new_w`, `new_h`, `x`, `y`, `w`, `h`, `dst_x`, `dst_y`.
 *
 * So to perform transformation on a `src` rectange of size `src_w ✕ src_h` on `dst` of size
 * `dst_w ✕ dst_h`, first scale `src` to `new_w ✕ new_h` and then copy the sub-rectangle
 * with top-left coordinate `(x, y)` of size `w ✕ h` to
 * `(dst_x, dst_y)` on `dst`.
 *
 * The sub-rectangle is always centered on the `dst`.
 */

#[derive(Debug, PartialEq)]
pub struct Transformation {
    pub new_w: i32,
    pub new_h: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub dst_x: i32,
    pub dst_y: i32,
}

/// Scales down `src` to the smallest size possible that fits `dst`.
#[allow(clippy::cast_possible_truncation)]
pub fn content_fit_scale_down(src_w: i32, src_h: i32, dst_w: i32, dst_h: i32) -> Transformation {
    let (new_w, new_h) = if (src_w > dst_w) || (src_h > dst_h) {
        let scale = (f64::from(dst_w) / f64::from(src_w)).min(f64::from(dst_h) / f64::from(src_h));
        let new_w = (scale * f64::from(src_w)) as i32;
        let new_h = (scale * f64::from(src_h)) as i32;
        (new_w, new_h)
    } else {
        (src_w, src_h)
    };

    let x = 0;
    let y = 0;
    let w = new_w;
    let h = new_h;
    let dst_x = (f64::from(dst_w - new_w) / 2.0) as i32;
    let dst_y = (f64::from(dst_h - new_h) / 2.0) as i32;

    Transformation {
        new_w,
        new_h,
        x,
        y,
        w,
        h,
        dst_x,
        dst_y,
    }
}

/// Scales up `src` to the smallest size possible that covers `dst`.
#[allow(clippy::cast_possible_truncation)]
pub fn content_fit_cover(src_w: i32, src_h: i32, dst_w: i32, dst_h: i32) -> Transformation {
    let (new_w, new_h) = if (src_w < dst_w) || (src_h < dst_h) {
        let scale = (f64::from(dst_w) / f64::from(src_w)).max(f64::from(dst_h) / f64::from(src_h));
        let new_w = (scale * f64::from(src_w)) as i32;
        let new_h = (scale * f64::from(src_h)) as i32;
        (new_w, new_h)
    } else {
        (src_w, src_h)
    };

    let x = (f64::from(new_w - dst_w) / 2.0) as i32;
    let y = (f64::from(new_h - dst_h) / 2.0) as i32;
    let w = dst_w.min(new_w - 2 * x);
    let h = dst_h.min(new_h - 2 * y);
    let dst_x = 0;
    let dst_y = 0;

    Transformation {
        new_w,
        new_h,
        x,
        y,
        w,
        h,
        dst_x,
        dst_y,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_content_fit_scale_down() {
        assert_eq!(
            content_fit_scale_down(100, 100, 100, 100),
            Transformation {
                new_w: 100,
                new_h: 100,
                x: 0,
                y: 0,
                w: 100,
                h: 100,
                dst_x: 0,
                dst_y: 0
            }
        );
        assert_eq!(
            content_fit_scale_down(400, 200, 100, 100),
            Transformation {
                new_w: 100,
                new_h: 50,
                x: 0,
                y: 0,
                w: 100,
                h: 50,
                dst_x: 0,
                dst_y: 25
            }
        );
        assert_eq!(
            content_fit_scale_down(40, 20, 100, 100),
            Transformation {
                new_w: 40,
                new_h: 20,
                x: 0,
                y: 0,
                w: 40,
                h: 20,
                dst_x: 30,
                dst_y: 40
            }
        );
    }

    #[test]
    fn test_content_fit_cover() {
        assert_eq!(
            content_fit_cover(100, 100, 100, 100),
            Transformation {
                new_w: 100,
                new_h: 100,
                x: 0,
                y: 0,
                w: 100,
                h: 100,
                dst_x: 0,
                dst_y: 0
            }
        );
        assert_eq!(
            content_fit_cover(400, 200, 100, 100),
            Transformation {
                new_w: 400,
                new_h: 200,
                x: 150,
                y: 50,
                w: 100,
                h: 100,
                dst_x: 0,
                dst_y: 0
            }
        );
        assert_eq!(
            content_fit_cover(40, 20, 100, 100),
            Transformation {
                new_w: 200,
                new_h: 100,
                x: 50,
                y: 0,
                w: 100,
                h: 100,
                dst_x: 0,
                dst_y: 0
            }
        );
    }
}
