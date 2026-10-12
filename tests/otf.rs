use octo::tide::{balance, click, Trip, View};
use octo::layout::Ctx;
use octo::{config, render, Side, Style, Tree};
use std::fs;
use std::path::PathBuf;

fn lib(tag: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("otf-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mk = |d: &str, n: usize| {
        let p = if d == "." { root.clone() } else { root.join(d) };
        fs::create_dir_all(&p).unwrap();
        for i in 0..n {
            fs::write(p.join(format!("f{i}.jpg")), b"").unwrap();
        }
    };
    mk(".", 3);
    mk("_float_", 2);
    mk("1", 4);
    mk("2", 1);
    mk("a/_float_", 1);
    mk("a/3", 2);
    for k in ["p", "q", "r", "s"] {
        mk(&format!("a/x/{k}"), 1);
    }
    for i in 0..12 {
        mk(&format!("many/m{i:02}"), 1);
    }
    root
}

fn tree(tag: &str) -> Tree {
    let root = lib(tag);
    let mut t = Tree::scan(&root, config::load(&root)).unwrap();
    balance(&mut t, Style::Arm, &Ctx::default());
    t
}

fn id(t: &Tree, p: &str) -> usize {
    t.find(p).unwrap().0
}

#[test]
fn categories_form_one_beak_row_per_tier() {
    let t = tree("beak");
    let f = render(&t, Style::Arm, &Ctx::default());
    let text: Vec<String> = f.lines.iter().map(|l| l.text()).collect();
    assert!(text.iter().any(|l| l.contains("a.[_float_:2]")), "{text:#?}");
    assert!(text.iter().any(|l| l.contains("c.[1:4, 2:1]")), "{text:#?}");
    // Categories are not arms.
    assert!(!text.iter().any(|l| l.contains("[1]")));
}

#[test]
fn eight_arms_then_ink_pages() {
    let mut t = tree("ink");
    let mut v = View { rows: 60, scroll: 0 };
    let many = id(&t, "many");
    click(&mut t, &mut v, Style::Arm, &Ctx::default(), many, false);
    let f = render(&t, Style::Arm, &Ctx::default());
    let shown = f.lines.iter().filter(|l| l.text().contains("[m0") || l.text().contains("[m1")).count();
    assert_eq!(shown, 7, "seven arms + ink when there is no beak");
    assert!(f.ink_row(many).is_some());
    assert_eq!(click(&mut t, &mut v, Style::Arm, &Ctx::default(), many, true), Trip::Paged { page: 1 });
    let f = render(&t, Style::Arm, &Ctx::default());
    assert!(f.lines.iter().any(|l| l.text().contains("[m07]")));
}

#[test]
fn click_keeps_row_pinned_when_it_fits() {
    let mut t = tree("pin");
    let mut v = View { rows: 30, scroll: 0 };
    let a = id(&t, "a");
    let before = render(&t, Style::Arm, &Ctx::default()).row(a).unwrap() - v.scroll;
    assert!(matches!(click(&mut t, &mut v, Style::Arm, &Ctx::default(), a, false), Trip::Fit { .. }));
    let after = render(&t, Style::Arm, &Ctx::default()).row(a).unwrap() - v.scroll;
    assert_eq!(before, after);
}

#[test]
fn snag_below_mirrors_arms_upward_with_flipped_corners() {
    let mut t = tree("mirror");
    let a = id(&t, "a");
    t.nodes[a].open = true;
    let x = id(&t, "a/x");
    let row = render(&t, Style::Arm, &Ctx::default()).row(x).unwrap();
    // Put x on the last viewport row so nothing fits below it.
    assert!(row >= 4, "fixture needs room above x, row {row}");
    let mut v = View { rows: row + 1, scroll: 0 };
    let tr = click(&mut t, &mut v, Style::Arm, &Ctx::default(), x, false);
    assert!(matches!(tr, Trip::Mirrored { side: Side::Up, .. }), "{tr:?}");
    let f = render(&t, Style::Arm, &Ctx::default());
    let r = f.row(x).unwrap();
    assert_eq!(r - v.scroll, row, "row stays pinned");
    let farthest = f.lines.iter().position(|l| l.text().contains("[p]")).unwrap();
    assert!(farthest < r);
    assert!(f.lines[farthest].text().contains("┌─ [p]") || f.lines[farthest].text().contains("├─ [p]"));
    let top = f.lines.iter().position(|l| l.text().contains("[s]")).unwrap();
    assert!(f.lines[top].text().contains("┌─ [s]"), "farthest mirrored arm takes the ┌ corner");
}

#[test]
fn reopening_a_tall_subtree_retracts_its_own_open_arms() {
    let mut t = tree("retract");
    for p in ["a", "a/x"] {
        let i = id(&t, p);
        t.nodes[i].open = true;
    }
    let a = id(&t, "a");
    let mut v = View { rows: 6, scroll: 0 };
    click(&mut t, &mut v, Style::Arm, &Ctx::default(), a, false); // fold
    let tr = click(&mut t, &mut v, Style::Arm, &Ctx::default(), a, false); // reopen: x's 4 arms + beak no longer fit
    assert!(matches!(tr, Trip::Retracted { .. }), "{tr:?}");
    assert!(!t.nodes[id(&t, "a/x")].open);
}

#[test]
fn every_style_renders_every_label_once() {
    for style in [Style::Arm, Style::Beak, Style::Boxed] {
        let mut t = tree(&format!("styles-{style:?}"));
        for i in 0..t.nodes.len() {
            t.nodes[i].open = true;
        }
        let f = render(&t, style, &Ctx::default());
        for (i, n) in t.nodes.iter().enumerate() {
            if n.own_cat.is_some() || n.parent == Some(id(&t, "many")) {
                continue;
            }
            assert_eq!(f.lines.iter().filter(|l| l.heads.contains(&i)).count(), 1, "{style:?} {}", n.name);
        }
        let w = f.lines.iter().map(|l| l.width()).max().unwrap();
        assert!(w < 200);
    }
}

#[test]
fn line_keys_are_unique_in_every_style_and_level() {
    use octo::layout::{REFLOW, Ctx as C};
    for style in [Style::Arm, Style::Beak, Style::Boxed] {
        let mut t = tree(&format!("keys-{style:?}"));
        for i in 0..t.nodes.len() {
            t.nodes[i].open = true;
        }
        for l in 0..REFLOW.len() {
            let f = render(&t, style, &C::viewer(id(&t, "a"), 0).level(l));
            let mut keys: Vec<_> = f.lines.iter().map(|x| x.key).collect();
            let n = keys.len();
            keys.sort_by_key(|k| format!("{k:?}"));
            keys.dedup();
            assert_eq!(keys.len(), n, "{style:?} level {l}");
        }
    }
}

#[test]
fn narrow_width_reflows_beak_onto_the_spine() {
    use octo::layout::fit;
    let mut t = tree("reflow");
    let a = id(&t, "a");
    t.nodes[a].open = true;
    let cx = Ctx::viewer(a, 0);
    let wide = render(&t, Style::Arm, &cx);
    let (f, lv) = fit(&t, Style::Arm, &cx, wide.width());
    assert_eq!(lv, 0, "a tree that fits keeps its shape");
    assert_eq!(f.width(), wide.width());
    let (f, lv) = fit(&t, Style::Arm, &cx, 24);
    assert!(lv >= 3, "level {lv}");
    assert!(f.width() < wide.width());
    let text: Vec<String> = f.lines.iter().map(|l| l.text()).collect();
    assert!(text.iter().any(|l| l.contains("╞═ a.[") || l.contains("╘═ a.[")), "{text:#?}");
    // Rows still resolve to their nodes.
    assert!(f.row(a).is_some());
}

#[test]
fn tide_pins_the_clicked_row_in_the_reflowed_shape() {
    let mut t = tree("tide-cols");
    let a = id(&t, "a");
    t.nodes[a].open = true;
    let x = id(&t, "a/x");
    let cx = Ctx { cols: 26, ..Ctx::viewer(a, 0) };
    let mut v = View { rows: 30, scroll: 0 };
    let before = render(&t, Style::Arm, &cx).row(x).unwrap() - v.scroll;
    click(&mut t, &mut v, Style::Arm, &cx, x, false);
    let f = render(&t, Style::Arm, &cx);
    assert!(t.nodes[x].open);
    assert_eq!(f.row(x).unwrap() - v.scroll, before, "the row under the pointer stays put");
    assert!(f.lines.iter().any(|l| l.text().contains("╞═") || l.text().contains("╘═")), "drawn stacked at this width");
}
