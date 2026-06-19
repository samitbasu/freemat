//! Unit tests for `fm-io`: MAT round-trip, FFT vs known transforms, regex.

use fm_core::{Array, C64};

use crate::matfile::{MatVar, read_mat, write_mat};

/// Round-trip a single named value through write+read and return it back.
fn round_trip(value: Array) -> Array {
    let bytes = write_mat(&[MatVar {
        name: "x".into(),
        value,
    }]);
    let vars = read_mat(&bytes).expect("read");
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name, "x");
    vars[0].value.clone()
}

#[test]
fn roundtrip_double_scalar() {
    let v = round_trip(Array::double(3.5));
    assert_eq!(v.as_f64(), Some(3.5));
}

#[test]
fn roundtrip_double_matrix() {
    let orig = Array::double_matrix(&[2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let v = round_trip(orig.clone());
    assert_eq!(v.dims(), vec![2, 3]);
    assert_eq!(
        fm_interp::value::to_f64_vec(&v),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    );
}

#[test]
fn roundtrip_single() {
    let orig = Array::single_matrix(&[1, 3], vec![1.0, 2.0, 3.0]);
    let v = round_trip(orig);
    assert_eq!(v.class_name(), "single");
    assert_eq!(fm_interp::value::to_f64_vec(&v), vec![1.0, 2.0, 3.0]);
}

#[test]
fn roundtrip_int_types() {
    let orig = Array::int32_matrix(&[1, 4], vec![-5, 0, 7, 100]);
    let v = round_trip(orig);
    assert_eq!(v.class_name(), "int32");
    assert_eq!(
        fm_interp::value::to_f64_vec(&v),
        vec![-5.0, 0.0, 7.0, 100.0]
    );
}

#[test]
fn roundtrip_logical() {
    let orig = Array::bool_matrix(&[1, 3], vec![true, false, true]);
    let v = round_trip(orig);
    assert_eq!(v.class_name(), "logical");
    assert_eq!(fm_interp::value::to_f64_vec(&v), vec![1.0, 0.0, 1.0]);
}

#[test]
fn roundtrip_complex() {
    let orig = Array::complex64_matrix(&[1, 2], vec![C64::new(1.0, 2.0), C64::new(-3.0, 4.0)]);
    let v = round_trip(orig);
    assert!(v.is_complex());
    let data = fm_interp::value::to_c64_vec(&v);
    assert_eq!(data, vec![C64::new(1.0, 2.0), C64::new(-3.0, 4.0)]);
}

#[test]
fn roundtrip_char() {
    let orig = Array::char_string("hello");
    let v = round_trip(orig);
    assert_eq!(v.as_string().as_deref(), Some("hello"));
}

#[test]
fn roundtrip_cell() {
    let orig = Array::cell(
        &[1, 3],
        vec![
            Array::char_string("bert"),
            Array::double(std::f64::consts::PI),
            Array::double(12.0),
        ],
    );
    let v = round_trip(orig);
    let cells = v.as_cell().expect("cell");
    assert_eq!(cells.len(), 3);
}

#[test]
fn roundtrip_struct() {
    use fm_core::StructArray;
    let s = StructArray::scalar([
        ("a".to_string(), Array::double(1.0)),
        ("b".to_string(), Array::char_string("text")),
    ]);
    let v = round_trip(Array::struct_array(s));
    let st = v.as_struct().expect("struct");
    assert_eq!(st.field_names(), vec!["a", "b"]);
    assert_eq!(st.scalar_field("a").and_then(Array::as_f64), Some(1.0));
}

#[test]
fn roundtrip_multiple_vars() {
    let vars = vec![
        MatVar {
            name: "x".into(),
            value: Array::double(1.0),
        },
        MatVar {
            name: "y".into(),
            value: Array::double_matrix(&[1, 2], vec![2.0, 3.0]),
        },
    ];
    let bytes = write_mat(&vars);
    let back = read_mat(&bytes).expect("read");
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].name, "x");
    assert_eq!(back[1].name, "y");
}

// ---- FFT ----------------------------------------------------------------

#[test]
fn fft_of_impulse_is_flat() {
    // fft of [1 0 0 0] = [1 1 1 1].
    let x = Array::double_matrix(&[1, 4], vec![1.0, 0.0, 0.0, 0.0]);
    let out = crate::fft::fft1(&[x], false).unwrap();
    let data = fm_interp::value::to_c64_vec(&out[0]);
    for c in &data {
        assert!((c.re - 1.0).abs() < 1e-12 && c.im.abs() < 1e-12);
    }
}

#[test]
fn fft_ifft_roundtrip() {
    let x = Array::double_matrix(&[1, 4], vec![1.0, 2.0, 3.0, 4.0]);
    let f = crate::fft::fft1(&[x], false).unwrap();
    let back = crate::fft::fft1(&[f[0].clone()], true).unwrap();
    let data = fm_interp::value::to_f64_vec(&back[0]);
    let expect = [1.0, 2.0, 3.0, 4.0];
    for (a, b) in data.iter().zip(expect.iter()) {
        assert!((a - b).abs() < 1e-10, "{a} vs {b}");
    }
}

#[test]
fn fft_constant_is_dc() {
    // fft of ones(1,4) = [4 0 0 0].
    let x = Array::double_matrix(&[1, 4], vec![1.0, 1.0, 1.0, 1.0]);
    let out = crate::fft::fft1(&[x], false).unwrap();
    let data = fm_interp::value::to_c64_vec(&out[0]);
    assert!((data[0].re - 4.0).abs() < 1e-12);
    for c in &data[1..] {
        assert!(c.norm() < 1e-12);
    }
}

// ---- regex --------------------------------------------------------------

#[test]
fn regexp_match() {
    let out = crate::regexp::regexp(
        &[
            Array::char_string("abc123def"),
            Array::char_string(r"\d+"),
            Array::char_string("match"),
        ],
        false,
        1,
    )
    .unwrap();
    let cells = out[0].as_cell().expect("cell");
    assert_eq!(cells.len(), 1);
    assert_eq!(
        cells.iter().next().unwrap().as_string().as_deref(),
        Some("123")
    );
}

#[test]
fn regexp_start_default() {
    let out = crate::regexp::regexp(
        &[Array::char_string("a1b2c3"), Array::char_string(r"\d")],
        false,
        1,
    )
    .unwrap();
    let starts = fm_interp::value::to_f64_vec(&out[0]);
    assert_eq!(starts, vec![2.0, 4.0, 6.0]);
}

#[test]
fn regexprep_replace() {
    let out = crate::regexp::regexprep(
        &[
            Array::char_string("hello world"),
            Array::char_string(r"o"),
            Array::char_string("0"),
        ],
        1,
    )
    .unwrap();
    assert_eq!(out[0].as_string().as_deref(), Some("hell0 w0rld"));
}

#[test]
fn regexprep_backreference() {
    let out = crate::regexp::regexprep(
        &[
            Array::char_string("John Smith"),
            Array::char_string(r"(\w+)\s(\w+)"),
            Array::char_string("$2 $1"),
        ],
        1,
    )
    .unwrap();
    assert_eq!(out[0].as_string().as_deref(), Some("Smith John"));
}

// ---- sscanf -------------------------------------------------------------

#[test]
fn sscanf_numbers() {
    let out = crate::scanf::sscanf("1.34 54 5.67", "%g").unwrap();
    let v = fm_interp::value::to_f64_vec(&out[0]);
    assert_eq!(out[0].dims(), vec![3, 1]);
    assert!((v[0] - 1.34).abs() < 1e-12);
    assert_eq!(v[1], 54.0);
    assert!((v[2] - 5.67).abs() < 1e-12);
}

// ---- reading real MATLAB-written fixtures -------------------------------

/// Read a MAT fixture written by MATLAB/FreeMat (proves cross-tool reads).
/// Fixtures are checked into `tests/fixtures/` so the test is self-contained.
fn read_fixture(name: &str) -> Vec<MatVar> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let bytes = std::fs::read(&path).expect("read fixture");
    read_mat(&bytes).expect("parse fixture")
}

#[test]
fn read_real_uncompressed_fixture() {
    // A v6 (uncompressed) MAT file written by FreeMat on Linux — proves the
    // reader handles files produced by a different tool.
    let vars = read_fixture("test_file_v6.mat");
    assert!(!vars.is_empty());
}

#[test]
fn read_real_compressed_fixture_first_var() {
    // A v7 zlib-compressed MAT file written by MATLAB. Its later elements are an
    // object-reference cell dump (MATLAB whitebox internals) we don't fully
    // parse, but the first compressed variable must read cleanly — proving the
    // inflate + miMATRIX path against a real MATLAB-written file.
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/wbtest_abs_1_ref.mat"),
    )
    .expect("read");
    let first = crate::matfile::read_first(&bytes).expect("read first var");
    assert!(!first.name.is_empty());
}

#[test]
fn imwrite_imread_grayscale_bmp_roundtrip() {
    use fm_core::DataClass;
    use fm_interp::value::{build_real, to_f64_vec};

    // A 4x5 uint8 grayscale image with distinct values per pixel.
    let (rows, cols) = (4usize, 5usize);
    let vals: Vec<f64> = (0..rows * cols).map(|k| (k * 7 % 256) as f64).collect();
    let img = build_real(DataClass::UInt8, &[rows, cols], vals.clone());

    let path = std::env::temp_dir().join("fm_io_imwrite_test.bmp");
    let path_arr = Array::char_string(&path.to_string_lossy());

    crate::image_io::imwrite(&[img, path_arr.clone()]).expect("imwrite ok");
    let out = crate::image_io::imread(&[path_arr]).expect("imread ok");
    let _ = std::fs::remove_file(&path);

    let b = &out[0];
    assert_eq!(b.class(), DataClass::UInt8);
    assert_eq!(b.dims(), vec![rows, cols]);
    assert_eq!(to_f64_vec(b), vals);
}
