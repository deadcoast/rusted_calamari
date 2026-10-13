//! The sorter's disk side: the same calls the app makes on a keystroke.

use image::{Rgb, RgbImage};
use octo::config::{self, Cat};
use octo::ops::{self, Op};
use octo::tree::Tree;
use std::fs;
use std::path::PathBuf;

fn lib(tag: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("sorter-{tag}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("1")).unwrap();
    for i in 0..4 {
        let mut im = RgbImage::new(6, 4);
        im.put_pixel(0, 0, Rgb([255, 0, 0]));
        im.save(root.join(format!("img_{i}.png"))).unwrap();
    }
    fs::write(root.join("1/img_0.png"), b"collision").unwrap();
    root
}

/// What `App::classify` does.
fn classify(t: &mut Tree, active: usize, name: &str, cat: usize) -> (usize, String) {
    let dir = t.dir(active);
    let cname = t.cats[cat].name.clone();
    let n = ops::move_into(&dir.join(name), &dir.join(&cname), name).unwrap();
    t.take_file(active, name).unwrap();
    let d = t.ensure_child(active, &cname);
    t.put_file(d, &n);
    (d, n)
}

#[test]
fn throw_moves_file_assigns_cid_and_undo_restores() {
    let root = lib("throw");
    let mut t = Tree::scan(&root, config::defaults()).unwrap();
    let z = t
        .cats
        .iter()
        .position(|c| c.key.as_deref() == Some("z"))
        .unwrap();
    assert_eq!(t.nodes[0].files.len(), 4);

    let (d, n) = classify(&mut t, 0, "img_0.png", z);
    assert_eq!(
        n, "img_0 (2).png",
        "name collision gets a suffix, nothing is overwritten"
    );
    assert!(root.join("1/img_0 (2).png").exists());
    assert_eq!(fs::read(root.join("1/img_0.png")).unwrap(), b"collision");
    assert_eq!(t.nodes[0].files.len(), 3);
    assert!(
        t.nodes[d].files.iter().all(|f| f.cat == Some(z)),
        "in-app cid follows the slot"
    );

    // A slot that did not exist yet is created on the first throw.
    let spc = t.cats.iter().position(|c| c.name == "_float_").unwrap();
    let (fd, _) = classify(&mut t, 0, "img_1.png", spc);
    assert!(root.join("_float_/img_1.png").exists());
    assert_eq!(t.nodes[fd].own_cat, Some(spc));

    // Undo the first throw.
    let back = ops::move_into(&root.join("1").join(&n), &root, "img_0.png").unwrap();
    t.take_file(d, &n).unwrap();
    let i = t.put_file(0, &back);
    assert_eq!(back, "img_0.png");
    assert_eq!(t.nodes[0].files[i].name, "img_0.png");
    assert!(root.join("img_0.png").exists());
}

#[test]
fn rotate_flip_and_restore_bytes() {
    let root = lib("edit");
    let p = root.join("img_2.png");
    let orig = ops::save_ops(&p, &[Op::RotCw]).unwrap();
    let im = image::open(&p).unwrap().to_rgb8();
    assert_eq!((im.width(), im.height()), (4, 6));
    assert_eq!(
        *im.get_pixel(3, 0),
        Rgb([255, 0, 0]),
        "top-left pixel lands top-right after cw"
    );
    ops::save_ops(&p, &[Op::FlipH]).unwrap();
    let im = image::open(&p).unwrap().to_rgb8();
    assert_eq!(*im.get_pixel(0, 0), Rgb([255, 0, 0]));
    fs::write(&p, &orig).unwrap();
    assert_eq!(image::open(&p).unwrap().width(), 6);
}

#[test]
fn categories_round_trip_and_recat() {
    let root = lib("cats");
    let mut t = Tree::scan(&root, config::defaults()).unwrap();
    let before = t.nodes[0].kids.len();
    fs::create_dir_all(root.join("street")).unwrap();
    t.cats.push(Cat {
        key: Some("b".into()),
        name: "street".into(),
        cid: (240, 163, 94),
        tier: 'b',
    });
    config::save(&root, &t.cats).unwrap();
    assert_eq!(config::load(&root).len(), t.cats.len());
    let s = t.ensure_child(0, "street");
    t.recat();
    assert_eq!(t.nodes[s].own_cat, Some(t.cats.len() - 1));
    assert_eq!(t.nodes[0].kids.len(), before + 1);
}

#[test]
fn manifest_and_export_set() {
    let root = lib("export");
    let mut t = Tree::scan(&root, config::defaults()).unwrap();
    let x = t
        .cats
        .iter()
        .position(|c| c.key.as_deref() == Some("x"))
        .unwrap();
    classify(&mut t, 0, "img_3.png", x);
    let m = ops::manifest(&t);
    assert!(m.lines().any(|l| l == "2/img_3.png,2,c,#92CC7C"), "{m}");
    let dest = root.with_extension("out");
    let _ = fs::remove_dir_all(&dest);
    let n = ops::export_set(&t, &dest).unwrap();
    assert_eq!(n, 2);
    assert!(dest.join("2/img_3.png").exists() && dest.join("1/img_0.png").exists());
}

#[test]
fn crop_then_export_as_other_formats() {
    let root = lib("crop");
    let src = root.join("img_1.png");
    // 6x4 image, red pixel at (0,0). Rotate right -> 4x6, red at (3,0).
    // Crop the right half of that -> 2x6, red at (1,0).
    let ops = [Op::RotCw, Op::Crop([0.5, 0.0, 1.0, 1.0])];
    assert_eq!(ops::dims(6, 4, &ops), [2, 6]);
    for ext in ["png", "jpg", "bmp", "tiff", "webp"] {
        let dest = root.join(format!("out.{ext}"));
        ops::export_ops(&src, &ops, &dest).unwrap();
        let im = image::open(&dest).unwrap().to_rgb8();
        assert_eq!((im.width(), im.height()), (2, 6), "{ext}");
        if ext != "jpg" {
            assert_eq!(*im.get_pixel(1, 0), Rgb([255, 0, 0]), "{ext}");
        }
    }
    assert!(ops::export_ops(&src, &[], &root.join("out.xyz")).is_err());
    // The source is untouched by an export.
    assert_eq!(image::open(&src).unwrap().width(), 6);
}
