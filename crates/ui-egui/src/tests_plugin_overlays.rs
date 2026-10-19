//! The canvas draws what live-effect plug-ins show besides their geometry, over the art; Outline
//! view draws none, nor do hidden objects (`canvas::plugin_effect_overlay`).

use egui::{Color32, Pos2, Shape, vec2};
use serde_json::json;
use vectorcraft_engine::Session;
use vectorcraft_plugins::overlay::{FORMAT, Line, Overlay};
use vectorcraft_plugins::wat as tpl;

use crate::VectorcraftApp;

fn app() -> VectorcraftApp {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 400, "height": 300})).unwrap();
    app
}

/// The points of each line one canvas frame draws in `color`.
fn drawn(app: &mut VectorcraftApp, ctx: &egui::Context, color: Color32) -> Vec<Vec<Pos2>> {
    let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
    let mut out = ctx.run_ui(raw, |ui| crate::canvas::show(app, ui));
    out.textures_delta.clear();
    let solid = egui::epaint::ColorMode::Solid(color);
    out.shapes
        .into_iter()
        .filter_map(|c| match c.shape {
            Shape::Path(p) if p.stroke.color == solid => Some(p.points),
            _ => None,
        })
        .collect()
}

#[test]
fn the_canvas_draws_what_a_live_effect_shows() {
    let mut app = app();
    let color = [10, 20, 250, 255];
    let overlay = Overlay { lines: vec![Line { color, width: 0.0, points: vec![[50.0, 50.0], [150.0, 50.0], [150.0, 120.0]] }] };
    let output = json!({"overlay": {"format": FORMAT, "data": vectorcraft_format::base64_encode(&overlay.encode())}});
    let module = wat::parse_str(tpl::returning(&tpl::manifest("test.ui.overlay", "effect", "{}"), &output.to_string())).unwrap();
    app.run("plugin.install", json!({"dataBase64": vectorcraft_format::base64_encode(&module)})).unwrap();
    let id = app.run("shape.rectangle", json!({"x": 40, "y": 40, "width": 60, "height": 60})).unwrap()["id"].clone();
    let ctx = egui::Context::default();
    let blue = Color32::from_rgba_unmultiplied(10, 20, 250, 255);
    assert!(drawn(&mut app, &ctx, blue).is_empty(), "no effect yet");
    app.run("effect.apply", json!({"effect": "plugin.test.ui.overlay"})).unwrap();
    let lines = drawn(&mut app, &ctx, blue);
    assert_eq!(lines.iter().map(Vec::len).collect::<Vec<_>>(), [3], "one line of three points");
    let [a, b, c] = [lines[0][0], lines[0][1], lines[0][2]];
    let as_drawn = (a.y - b.y).abs() < 0.01 && b.x > a.x && (c.x - b.x).abs() < 0.01 && (c.y - b.y).abs() > 1.0;
    assert!(as_drawn, "as the plug-in drew it: {:?}", lines[0]);
    // Outline view draws no effects, and a hidden object nothing.
    app.run("view.outline", json!({})).unwrap();
    assert!(drawn(&mut app, &ctx, blue).is_empty(), "Outline view");
    app.run("view.outline", json!({})).unwrap();
    assert_eq!(drawn(&mut app, &ctx, blue).len(), 1);
    app.run("object.hide", json!({"ids": [id]})).unwrap();
    assert!(drawn(&mut app, &ctx, blue).is_empty(), "hidden");
    vectorcraft_plugins::registry::remove("test.ui.overlay");
}

/// `CRAFT_UI_OVERLAY_EFFECT_WASM=<folder>` writes a live effect to try in the app
/// (`pen-preview-test-effect.wasm`): it keeps the geometry and shows a pen hatching a 200 × 150 pt
/// rectangle at (100, 80), the pen's strokes in blue and its moves between them in orange.
#[test]
fn overlay_test_effect_for_pictures() {
    let Some(dir) = std::env::var_os("CRAFT_UI_OVERLAY_EFFECT_WASM") else { return };
    let (x0, y0, x1, y1) = (104.0_f32, 86.0, 296.0, 226.0);
    let mut lines = Vec::new();
    let mut from: Option<[f32; 2]> = None;
    for (row, y) in (0..).map(|i| (i, y0 + 10.0 * i as f32)).take_while(|&(_, y)| y <= y1) {
        let (a, b) = if row % 2 == 0 { ([x0, y], [x1, y]) } else { ([x1, y], [x0, y]) };
        if let Some(f) = from {
            lines.push(Line { color: [255, 140, 0, 255], width: 0.0, points: vec![f, a] });
        }
        lines.push(Line { color: [30, 110, 255, 255], width: 0.75, points: vec![a, b] });
        from = Some(b);
    }
    let overlay = Overlay { lines };
    let output = json!({"overlay": {"format": FORMAT, "data": vectorcraft_format::base64_encode(&overlay.encode())}});
    let manifest = r#"{"id":"org.example.penpreview","name":"Pen Path Preview (test plug-in)","version":"1","kind":"effect",
        "description":"Shows how a pen would hatch the object, without changing it."}"#;
    let module = wat::parse_str(tpl::returning(manifest, &output.to_string())).unwrap();
    std::fs::write(std::path::PathBuf::from(dir).join("pen-preview-test-effect.wasm"), module).unwrap();
}
