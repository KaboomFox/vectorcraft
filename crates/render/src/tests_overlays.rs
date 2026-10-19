//! What live-effect plug-ins show besides their geometry ([`plugin_overlays`]): the overlays of the
//! effects drawing runs, in the order it runs them, and never in what the renderer draws.

use serde_json::json;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, AppearanceItem, Effect, Node, NodeKind};
use vectorcraft_geom::{PathData, Point, SubPath};
use vectorcraft_plugins::overlay::{FORMAT, Line, Overlay};
use vectorcraft_plugins::wat as tpl;

use super::*;

/// An overlay of one line from (`x`, 10) to (`x`, 90), in `color`, 2 pt wide.
fn line_at(x: f32, color: [u8; 4]) -> Overlay {
    Overlay { lines: vec![Line { color, width: 2.0, points: vec![[x, 10.0], [x, 90.0]] }] }
}

/// Installs live-effect plug-in `id`, which shows `overlay` and keeps the geometry it is given;
/// → its effect record.
fn showing(id: &str, overlay: &Overlay) -> Effect {
    let output = json!({"overlay": {"format": FORMAT, "data": vectorcraft_doc::base64::encode(&overlay.encode())}});
    let module = wat::parse_str(tpl::returning(&tpl::manifest(id, "effect", "{}"), &output.to_string())).unwrap();
    vectorcraft_plugins::registry::install_bytes(&module).unwrap();
    Effect { id: vectorcraft_plugins::effect::effect_id(id), params: json!({}), visible: true }
}

/// A red 20 × 20 square at (10, 10) with no stroke.
fn square() -> Node {
    let corners = [Point::new(10.0, 10.0), Point::new(30.0, 10.0), Point::new(30.0, 30.0), Point::new(10.0, 30.0)];
    Node::path(NodeId(1), PathData::single(SubPath::polyline(&corners, true)), Appearance::basic(Paint::solid(Color::rgb(1.0, 0.0, 0.0)), Paint::None, 0.0))
}

#[test]
fn overlays_come_in_the_order_drawing_runs_the_effects() {
    let (a, b, c) = (line_at(50.0, [0, 0, 255, 255]), line_at(60.0, [0, 255, 0, 255]), line_at(70.0, [255, 0, 255, 255]));
    let mut n = square();
    assert!(plugin_overlays(&n).is_empty(), "no plug-in effect, no overlay");
    n.appearance.effects = vec![showing("test.render.overlay.a", &a)];
    // The fill's own effects run after the object's, on the object's geometry.
    if let Some(AppearanceItem::Fill(f)) = n.appearance.items.first_mut() {
        f.effects = vec![showing("test.render.overlay.b", &b)];
    }
    let shown = |n: &Node| plugin_overlays(n).iter().map(|o| (**o).clone()).collect::<Vec<_>>();
    assert_eq!(shown(&n), [a.clone(), b.clone()]);
    // A hidden effect, and the effects of a fill that doesn't paint, don't run when it is drawn.
    n.appearance.effects.push(Effect { visible: false, ..showing("test.render.overlay.c", &c) });
    assert_eq!(shown(&n), [a.clone(), b.clone()]);
    if let Some(AppearanceItem::Fill(f)) = n.appearance.items.first_mut() {
        f.paint = Paint::None;
    }
    assert_eq!(shown(&n), [a.clone()]);
    // Guides are drawn without their effects; other objects show none.
    let mut guide = n.clone();
    if let NodeKind::Path { guide: g, .. } = &mut guide.kind {
        *g = true;
    }
    assert!(plugin_overlays(&guide).is_empty());
    for id in ["test.render.overlay.a", "test.render.overlay.b", "test.render.overlay.c"] {
        vectorcraft_plugins::registry::remove(id);
    }
}

/// The renderer draws the geometry, which an overlay alone leaves as it was, and never the
/// overlay: exports, prints and thumbnails all go through it.
#[test]
fn the_renderer_draws_no_overlay() {
    let mut d = Document::new(100.0, 100.0);
    let mut n = square();
    n.id = d.alloc_id();
    let layer = d.layers[0].id;
    d.insert(Some(layer), 0, n.clone()).unwrap();
    let opts = RenderOptions { background: Some([255, 255, 255, 255]), ..Default::default() };
    let before = Renderer::new().render(&d, 100, 100, Affine::IDENTITY, &opts);
    let fx = showing("test.render.overlay.drawn", &line_at(50.0, [0, 0, 255, 255]));
    let id = n.id;
    d.node_mut(id).unwrap().appearance.effects = vec![fx];
    let after = Renderer::new().render(&d, 100, 100, Affine::IDENTITY, &opts);
    assert_eq!(after.pixel(50, 50), [255, 255, 255, 255], "no line where the overlay is");
    assert_eq!(after.pixels, before.pixels, "the square as it was");
    assert_eq!(plugin_overlays(d.node(id).unwrap()).len(), 1);
    vectorcraft_plugins::registry::remove("test.render.overlay.drawn");
}
