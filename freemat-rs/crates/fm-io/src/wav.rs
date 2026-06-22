//! Minimal RIFF/WAVE PCM codec for `wavread` / `wavwrite`.
//!
//! Hand-rolls the 44-byte canonical WAV header plus a PCM `data` chunk rather
//! than pulling in an audio dependency. Both 8-bit unsigned and 16-bit signed
//! integer PCM are supported on read; `wavwrite` emits 16-bit signed PCM (the
//! FreeMat default), which is enough for round-tripping a normalized signal.
//!
//! Sample matrices follow the FreeMat/MATLAB convention: rows are sample frames
//! and columns are channels, with values normalized to `[-1, 1]`.

use std::fs;

use fm_core::Array;
use fm_interp::error::Flow;
use fm_interp::value::to_f64_vec;

use crate::builtins::err;

/// `wavwrite(y, fs, filename)` (also `wavwrite(y, filename)` defaulting `fs` to
/// 8000) — write samples `y` (columns = channels, values in `[-1, 1]`) as
/// 16-bit PCM at sample rate `fs`.
pub fn wavwrite(args: &[Array]) -> Flow<Vec<Array>> {
    let (y, fs, filename) = match args.len() {
        2 => match args[1].as_string() {
            Some(f) => (&args[0], 8000.0_f64, f),
            None => return err("wavwrite: filename must be a string"),
        },
        3 | 4 => {
            let fs = args[1].as_f64().unwrap_or(8000.0);
            match args[args.len() - 1].as_string() {
                Some(f) => (&args[0], fs, f),
                None => return err("wavwrite: filename must be a string"),
            }
        }
        _ => return err("wavwrite: unrecognized form of wavwrite call"),
    };

    let dims = y.dims();
    // A vector is treated as a single channel (a column), matching `y(:)`.
    let (frames, channels) = if dims.len() <= 2 {
        let r = *dims.first().unwrap_or(&0);
        let c = *dims.get(1).unwrap_or(&1);
        if r <= 1 || c <= 1 {
            (r.max(c), 1usize) // row/column vector -> mono
        } else {
            (r, c)
        }
    } else {
        return err("wavwrite: y must be a matrix (frames x channels)");
    };

    // `to_f64_vec` yields column-major data: all of channel 0, then channel 1…
    let data = to_f64_vec(y);
    let fs = fs.round() as u32;
    let n_bits: u16 = 16;
    let bytes_per_sample = (n_bits / 8) as usize;
    let block_align = (channels * bytes_per_sample) as u16;
    let byte_rate = fs * u32::from(block_align);
    let data_len = frames * channels * bytes_per_sample;

    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_len);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    buf.extend_from_slice(&(channels as u16).to_le_bytes());
    buf.extend_from_slice(&fs.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&n_bits.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_len as u32).to_le_bytes());

    // Interleave: for each frame, write one sample per channel.
    for frame in 0..frames {
        for ch in 0..channels {
            let idx = ch * frames + frame; // column-major index into `data`
            let v = data.get(idx).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            let s = (v * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
            buf.extend_from_slice(&s.to_le_bytes());
        }
    }

    if fs::write(&filename, &buf).is_err() {
        return err(format!("wavwrite: could not write {filename}"));
    }
    Ok(vec![])
}

/// `[y, fs, nbits] = wavread(filename)` — read a PCM WAV file. Samples are
/// returned normalized to `[-1, 1]` as a `frames x channels` matrix; `fs` is the
/// sample rate and `nbits` the bit depth.
pub fn wavread(args: &[Array]) -> Flow<Vec<Array>> {
    let Some(mut filename) = args.first().and_then(Array::as_string) else {
        return err("wavread requires a filename argument");
    };
    if !filename.to_lowercase().ends_with(".wav") && !std::path::Path::new(&filename).exists() {
        filename.push_str(".wav");
    }
    let Ok(buf) = fs::read(&filename) else {
        return err(format!("wavread: could not read {filename}"));
    };
    if buf.len() < 44 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" {
        return err(format!("{filename} is not a WAV file"));
    }

    // Walk the chunks after the 12-byte RIFF/WAVE header.
    let mut pos = 12usize;
    let mut channels = 1u16;
    let mut fs = 0u32;
    let mut n_bits = 16u16;
    let mut audio_format = 1u16;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= buf.len() {
        let id = &buf[pos..pos + 4];
        let size =
            u32::from_le_bytes([buf[pos + 4], buf[pos + 5], buf[pos + 6], buf[pos + 7]]) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(buf.len());
        if id == b"fmt " && body_end - body_start >= 16 {
            let b = &buf[body_start..];
            audio_format = u16::from_le_bytes([b[0], b[1]]);
            channels = u16::from_le_bytes([b[2], b[3]]);
            fs = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
            n_bits = u16::from_le_bytes([b[14], b[15]]);
        } else if id == b"data" {
            data = Some(&buf[body_start..body_end]);
        }
        // Chunks are word-aligned (pad to even size).
        pos = body_start + size + (size & 1);
    }

    let Some(data) = data else {
        return err(format!("{filename}: did not find 'data' chunk"));
    };
    let channels = channels.max(1) as usize;
    let bytes_per_sample = (n_bits / 8) as usize;
    if bytes_per_sample == 0 {
        return err("wavread: unsupported WAV format");
    }
    let total_samples = data.len() / bytes_per_sample;
    let frames = total_samples / channels;

    // Decode interleaved samples into a column-major (frames x channels) matrix.
    let mut mat = vec![0.0_f64; frames * channels];
    for frame in 0..frames {
        for ch in 0..channels {
            let i = (frame * channels + ch) * bytes_per_sample;
            let v = match (n_bits, audio_format) {
                (8, _) => {
                    // 8-bit WAV PCM is unsigned, centered at 128.
                    f64::from(data[i]) / 255.0 * 2.0 - 1.0
                }
                (16, _) => f64::from(i16::from_le_bytes([data[i], data[i + 1]])) / 32768.0,
                (32, 3) => f64::from(f32::from_le_bytes([
                    data[i],
                    data[i + 1],
                    data[i + 2],
                    data[i + 3],
                ])),
                (32, _) => {
                    f64::from(i32::from_le_bytes([
                        data[i],
                        data[i + 1],
                        data[i + 2],
                        data[i + 3],
                    ])) / 2_147_483_648.0
                }
                _ => return err("wavread: unsupported WAV bit depth"),
            };
            mat[ch * frames + frame] = v; // column-major slot
        }
    }

    Ok(vec![
        Array::double_matrix(&[frames, channels], mat),
        Array::double(f64::from(fs)),
        Array::double(f64::from(n_bits)),
    ])
}
