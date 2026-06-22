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

#[test]
fn fftshift_even_vector() {
    // fftshift([1 2 3 4]) == [3 4 1 2].
    let x = Array::double_matrix(&[1, 4], vec![1.0, 2.0, 3.0, 4.0]);
    let out = crate::fft::fftshift(&[x], true).unwrap();
    let data = fm_interp::value::to_f64_vec(&out[0]);
    assert_eq!(data, vec![3.0, 4.0, 1.0, 2.0]);
}

#[test]
fn ifftshift_roundtrip_odd() {
    // ifftshift(fftshift([1 2 3 4 5])) == [1 2 3 4 5] (odd length).
    let x = Array::double_matrix(&[1, 5], vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let shifted = crate::fft::fftshift(&[x], true).unwrap();
    let back = crate::fft::fftshift(&[shifted[0].clone()], false).unwrap();
    let data = fm_interp::value::to_f64_vec(&back[0]);
    assert_eq!(data, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
}

#[test]
fn fftn_matches_fft2() {
    // fftn of a 2-D matrix equals fft2 of it.
    let m = Array::double_matrix(&[3, 4], (0..12).map(|i| (i as f64) * 1.5 + 1.0).collect());
    let a = crate::fft::fftn(&[m.clone()], false).unwrap();
    let b = crate::fft::fft2(&[m], false).unwrap();
    let da = fm_interp::value::to_c64_vec(&a[0]);
    let db = fm_interp::value::to_c64_vec(&b[0]);
    assert_eq!(a[0].dims(), b[0].dims());
    assert_eq!(da.len(), db.len());
    for (x, y) in da.iter().zip(db.iter()) {
        assert!((x - y).norm() < 1e-9, "{x:?} vs {y:?}");
    }
}

#[test]
fn hilbert_analytic_signal() {
    // real(hilbert(x)) == x; result is complex with the known MATLAB value.
    let x = Array::double_matrix(&[1, 4], vec![1.0, 2.0, 3.0, 4.0]);
    let out = crate::fft::hilbert(&[x], 1).unwrap();
    let data = fm_interp::value::to_c64_vec(&out[0]);
    let re: Vec<f64> = data.iter().map(|c| c.re).collect();
    for (a, b) in re.iter().zip([1.0, 2.0, 3.0, 4.0].iter()) {
        assert!((a - b).abs() < 1e-10, "{a} vs {b}");
    }
    // MATLAB: hilbert([1 2 3 4]) imag parts == [1 -1 -1 1].
    let expect_im = [1.0, -1.0, -1.0, 1.0];
    for (c, e) in data.iter().zip(expect_im.iter()) {
        assert!((c.im - e).abs() < 1e-10, "im {} vs {e}", c.im);
    }
    // Must actually be complex (nonzero imaginary part).
    assert!(data.iter().any(|c| c.im.abs() > 1e-9));
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

// ---- path / separator builtins ------------------------------------------

mod path_builtins {
    use fm_core::Array;
    use fm_interp::Interpreter;
    use fm_parser::Span;

    /// A fresh interpreter with the `fm-io` builtins registered.
    fn interp() -> Interpreter {
        let mut i = Interpreter::new();
        crate::register_io(&mut i);
        i
    }

    /// Call a builtin by name and return its first output as a string.
    fn call_str(i: &mut Interpreter, name: &str, args: &[Array]) -> String {
        let out = i
            .call_function(name, args, 1, "", Span::empty(0))
            .expect("call ok");
        out.first().and_then(Array::as_string).unwrap_or_default()
    }

    #[test]
    fn separators_are_single_chars() {
        let mut i = interp();
        let fs = call_str(&mut i, "filesep", &[]);
        assert_eq!(fs, std::path::MAIN_SEPARATOR.to_string());
        // dirsep is the directory-separator alias of filesep.
        assert_eq!(call_str(&mut i, "dirsep", &[]), fs);
        let ps = call_str(&mut i, "pathsep", &[]);
        assert_eq!(ps, if cfg!(windows) { ";" } else { ":" });
    }

    #[test]
    fn addpath_then_getpath_reflects_dir() {
        let mut i = interp();
        assert_eq!(call_str(&mut i, "getpath", &[]), "");
        i.call_function(
            "addpath",
            &[Array::char_string("/tmp/foo")],
            0,
            "",
            Span::empty(0),
        )
        .expect("addpath ok");
        let p = call_str(&mut i, "getpath", &[]);
        assert_eq!(p, "/tmp/foo");
        assert_eq!(call_str(&mut i, "path", &[]), "/tmp/foo");

        // Prepend keeps the new dir first; append (-end) puts it last.
        i.call_function(
            "addpath",
            &[Array::char_string("/tmp/bar")],
            0,
            "",
            Span::empty(0),
        )
        .expect("addpath ok");
        let sep = if cfg!(windows) { ";" } else { ":" };
        assert_eq!(
            call_str(&mut i, "getpath", &[]),
            format!("/tmp/bar{sep}/tmp/foo")
        );

        i.call_function(
            "addpath",
            &[Array::char_string("/tmp/baz"), Array::char_string("-end")],
            0,
            "",
            Span::empty(0),
        )
        .expect("addpath ok");
        assert_eq!(
            call_str(&mut i, "getpath", &[]),
            format!("/tmp/bar{sep}/tmp/foo{sep}/tmp/baz")
        );
    }

    #[test]
    fn setpath_replaces_and_path_with_two_args_concats() {
        let mut i = interp();
        let sep = if cfg!(windows) { ";" } else { ":" };
        i.call_function(
            "setpath",
            &[Array::char_string(&format!("/a{sep}/b"))],
            0,
            "",
            Span::empty(0),
        )
        .expect("setpath ok");
        assert_eq!(call_str(&mut i, "getpath", &[]), format!("/a{sep}/b"));

        // path(p1, p2) concatenates the two.
        i.call_function(
            "path",
            &[Array::char_string("/x"), Array::char_string("/y")],
            0,
            "",
            Span::empty(0),
        )
        .expect("path ok");
        assert_eq!(call_str(&mut i, "getpath", &[]), format!("/x{sep}/y"));
    }

    #[test]
    fn rehash_rescan_pathtool_run_clean() {
        let mut i = interp();
        for name in ["rehash", "rescan", "pathtool"] {
            i.call_function(name, &[], 0, "", Span::empty(0))
                .unwrap_or_else(|_| panic!("{name} ok"));
        }
    }

    #[test]
    fn what_returns_struct_without_error() {
        let mut i = interp();
        let out = i
            .call_function("what", &[], 1, "", Span::empty(0))
            .expect("what ok");
        let s = &out[0];
        assert_eq!(s.class_name(), "struct");
        assert_eq!(s.dims(), vec![1, 1]);
    }
}

// ---- directory / file builtins ------------------------------------------

mod fs_builtins {
    use std::path::PathBuf;

    use fm_core::Array;
    use fm_interp::Interpreter;
    use fm_parser::Span;

    fn interp() -> Interpreter {
        let mut i = Interpreter::new();
        crate::register_io(&mut i);
        i
    }

    /// A unique temporary directory path for this test process (not yet created).
    fn unique_tmp() -> PathBuf {
        let mut p = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!("fm_io_fs_test_{}_{}", std::process::id(), stamp));
        p
    }

    #[test]
    fn mkdir_copyfile_fileattrib_rmdir_roundtrip() {
        let mut i = interp();
        let root = unique_tmp();
        let root_str = root.to_string_lossy().into_owned();

        // mkdir(root) — creates it, returns success flag 1.
        let out = i
            .call_function(
                "mkdir",
                &[Array::char_string(&root_str)],
                1,
                "",
                Span::empty(0),
            )
            .expect("mkdir ok");
        assert_eq!(out[0].as_f64(), Some(1.0));
        assert!(root.is_dir());

        // mkdir(root, "sub") — two-arg form creates a subdir.
        let out = i
            .call_function(
                "mkdir",
                &[Array::char_string(&root_str), Array::char_string("sub")],
                1,
                "",
                Span::empty(0),
            )
            .expect("mkdir sub ok");
        assert_eq!(out[0].as_f64(), Some(1.0));
        let sub = root.join("sub");
        assert!(sub.is_dir());

        // mkdir on an existing dir does not fail (still returns 1).
        let out = i
            .call_function(
                "mkdir",
                &[Array::char_string(&root_str)],
                1,
                "",
                Span::empty(0),
            )
            .expect("mkdir existing ok");
        assert_eq!(out[0].as_f64(), Some(1.0));

        // Write a file, then copyfile it; the copy must exist with same content.
        let src = root.join("a.txt");
        std::fs::write(&src, b"hello fileio").expect("write src");
        let dst = root.join("b.txt");
        let out = i
            .call_function(
                "copyfile",
                &[
                    Array::char_string(&src.to_string_lossy()),
                    Array::char_string(&dst.to_string_lossy()),
                ],
                1,
                "",
                Span::empty(0),
            )
            .expect("copyfile ok");
        assert_eq!(out[0].as_f64(), Some(1.0));
        assert!(dst.is_file());
        assert_eq!(std::fs::read(&dst).expect("read dst"), b"hello fileio");

        // fileattrib(root) — success + directory flag set.
        let out = i
            .call_function(
                "fileattrib",
                &[Array::char_string(&root_str)],
                3,
                "",
                Span::empty(0),
            )
            .expect("fileattrib ok");
        assert_eq!(out[0].as_f64(), Some(1.0));
        let attribs = &out[1];
        assert_eq!(attribs.class_name(), "struct");
        let dir_flag = attribs
            .as_struct()
            .and_then(|s| s.field("directory"))
            .and_then(|v| v.first())
            .and_then(Array::as_f64)
            .expect("directory field");
        assert_eq!(dir_flag, 1.0);

        // fileattrib on a missing path returns success 0 (no throw).
        let missing = root.join("does_not_exist");
        let out = i
            .call_function(
                "fileattrib",
                &[Array::char_string(&missing.to_string_lossy())],
                3,
                "",
                Span::empty(0),
            )
            .expect("fileattrib missing ok");
        assert_eq!(out[0].as_f64(), Some(0.0));

        // rmdir(root, 's') — recursive remove; the tree must be gone.
        let out = i
            .call_function(
                "rmdir",
                &[Array::char_string(&root_str), Array::char_string("s")],
                1,
                "",
                Span::empty(0),
            )
            .expect("rmdir ok");
        assert_eq!(out[0].as_f64(), Some(1.0));
        assert!(!root.exists());

        // Defensive cleanup in case any assertion above left the tree behind.
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `rawwrite`/`fflush`/`rawread` round-trip: write int16 values to a temp
    /// file, read them back, and confirm equality.
    #[test]
    fn rawwrite_fflush_rawread_roundtrip() {
        let mut i = interp();
        let path = unique_tmp().with_extension("raw");
        let path_str = path.to_string_lossy().into_owned();

        // rawwrite(fname, x) — write a few int16 values.
        let data = fm_interp::value::build_real(
            fm_core::DataClass::Int16,
            &[1, 4],
            vec![10.0, -20.0, 300.0, -4000.0],
        );
        i.call_function(
            "rawwrite",
            &[Array::char_string(&path_str), data],
            0,
            "",
            Span::empty(0),
        )
        .expect("rawwrite ok");
        assert!(path.is_file());

        // rawread(fname, size, precision) — read them back as int16.
        let out = i
            .call_function(
                "rawread",
                &[
                    Array::char_string(&path_str),
                    Array::double_matrix(&[1, 2], vec![4.0, 1.0]),
                    Array::char_string("int16"),
                ],
                1,
                "",
                Span::empty(0),
            )
            .expect("rawread ok");
        assert_eq!(out[0].class_name(), "int16");
        assert_eq!(
            fm_interp::value::to_f64_vec(&out[0]),
            vec![10.0, -20.0, 300.0, -4000.0]
        );

        // fflush on a write handle must succeed cleanly (returns nothing).
        let opened = i
            .call_function(
                "fopen",
                &[Array::char_string(&path_str), Array::char_string("w")],
                1,
                "",
                Span::empty(0),
            )
            .expect("fopen ok");
        let fid = opened[0].clone();
        let flushed = i
            .call_function("fflush", std::slice::from_ref(&fid), 0, "", Span::empty(0))
            .expect("fflush ok");
        assert!(flushed.is_empty());
        i.call_function("fclose", &[fid], 0, "", Span::empty(0))
            .expect("fclose ok");

        let _ = std::fs::remove_file(&path);
    }

    /// `getline(fid)` reads the first line of a multi-line file (not the
    /// interactive stdin/prompt path).
    #[test]
    fn getline_from_file_handle() {
        let mut i = interp();
        let path = unique_tmp().with_extension("txt");
        let path_str = path.to_string_lossy().into_owned();
        std::fs::write(&path, b"first line\nsecond line\n").expect("write txt");

        let opened = i
            .call_function(
                "fopen",
                &[Array::char_string(&path_str), Array::char_string("r")],
                1,
                "",
                Span::empty(0),
            )
            .expect("fopen ok");
        let fid = opened[0].clone();

        let out = i
            .call_function("getline", std::slice::from_ref(&fid), 1, "", Span::empty(0))
            .expect("getline ok");
        assert_eq!(out[0].as_string().as_deref(), Some("first line"));

        i.call_function("fclose", &[fid], 0, "", Span::empty(0))
            .expect("fclose ok");
        let _ = std::fs::remove_file(&path);
    }
}
