//! Image read/write builtins: `imwrite` / `imread`, backed by the `image`
//! crate. FreeMat stores images as `M×N` (grayscale) or `M×N×3` (RGB) arrays
//! in column-major order; the on-disk file format is taken from the filename
//! extension.

use fm_core::{Array, DataClass};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_real, to_f64_vec};
use image::{DynamicImage, GrayImage, RgbImage};

fn err(msg: impl Into<String>) -> Signal {
    Signal::Error(InterpError::msg(msg.into()))
}

/// `imwrite(A, filename)` — write image `A` to `filename` (format from the
/// extension). `A` is `M×N` grayscale or `M×N×3` RGB; integer arrays are written
/// verbatim (clamped to `0..=255`), floating arrays are scaled from `[0,1]`.
pub fn imwrite(args: &[Array]) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return Err(err("imwrite requires an image and a filename"));
    }
    let a = &args[0];
    let path = args[1]
        .as_string()
        .ok_or_else(|| err("imwrite: filename must be a string"))?;

    let dims = a.dims();
    let (rows, cols) = (dims[0], *dims.get(1).unwrap_or(&1));
    let planes = dims.get(2).copied().unwrap_or(1);
    if dims.len() > 3 || !(planes == 1 || planes == 3) {
        return Err(err(
            "imwrite: image must be M-by-N (grayscale) or M-by-N-by-3 (RGB)",
        ));
    }

    // FreeMat/MATLAB: floating images live in [0,1]; integer images are raw.
    let scale = matches!(a.class(), DataClass::Double | DataClass::Float);
    let vals = to_f64_vec(a);
    let to_u8 = |v: f64| -> u8 {
        let s = if scale { v * 255.0 } else { v };
        s.round().clamp(0.0, 255.0) as u8
    };
    // Column-major element (row, col, plane).
    let at = |row: usize, col: usize, plane: usize| -> u8 {
        to_u8(vals[row + col * rows + plane * rows * cols])
    };

    let dynimg = if planes == 1 {
        let mut img = GrayImage::new(cols as u32, rows as u32);
        for row in 0..rows {
            for col in 0..cols {
                img.put_pixel(col as u32, row as u32, image::Luma([at(row, col, 0)]));
            }
        }
        DynamicImage::ImageLuma8(img)
    } else {
        let mut img = RgbImage::new(cols as u32, rows as u32);
        for row in 0..rows {
            for col in 0..cols {
                let px = image::Rgb([at(row, col, 0), at(row, col, 1), at(row, col, 2)]);
                img.put_pixel(col as u32, row as u32, px);
            }
        }
        DynamicImage::ImageRgb8(img)
    };

    dynimg
        .save(&path)
        .map_err(|e| err(format!("imwrite: cannot write '{path}': {e}")))?;
    Ok(vec![])
}

/// `imread(filename)` — read an image file into a `uint8` array: `M×N` for a
/// grayscale image, `M×N×3` for a color image.
pub fn imread(args: &[Array]) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return Err(err("imread requires a filename"));
    }
    let path = args[0]
        .as_string()
        .ok_or_else(|| err("imread: filename must be a string"))?;
    let img = image::open(&path).map_err(|e| err(format!("imread: cannot read '{path}': {e}")))?;
    let (cols, rows) = (img.width() as usize, img.height() as usize);

    // Decode to RGB, then decide grayscale vs color by content: a grayscale
    // source returns M×N, a color source returns M×N×3. We test the content
    // (rather than the file's declared color type) because some formats — e.g.
    // 8-bit BMP — store grayscale through a color palette, which decoders expand
    // to RGB; MATLAB still hands such an image back as a 2-D grayscale array.
    let rgb = img.to_rgb8();
    let is_gray = rgb.pixels().all(|p| p.0[0] == p.0[1] && p.0[1] == p.0[2]);

    if is_gray {
        let mut data = vec![0.0f64; rows * cols];
        for row in 0..rows {
            for col in 0..cols {
                data[row + col * rows] = f64::from(rgb.get_pixel(col as u32, row as u32).0[0]);
            }
        }
        Ok(vec![build_real(DataClass::UInt8, &[rows, cols], data)])
    } else {
        let mut data = vec![0.0f64; rows * cols * 3];
        for row in 0..rows {
            for col in 0..cols {
                let px = rgb.get_pixel(col as u32, row as u32).0;
                for (plane, &c) in px.iter().enumerate() {
                    data[row + col * rows + plane * rows * cols] = f64::from(c);
                }
            }
        }
        Ok(vec![build_real(DataClass::UInt8, &[rows, cols, 3], data)])
    }
}
