//! Attributes in other applications' XML namespaces (https://www.w3.org/TR/xml-names/) survive a
//! round trip through VectorCraft: kept on import, written back on export with their namespaces
//! declared on the root.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use usvg::roxmltree;
use vectorcraft_doc::{Document, ForeignAttribute, Node, NodeKind};
use vectorcraft_svg::{ExportOptions, export, export_full, import};

const PLOTTER: &str = "https://example.org/plotter";
const CUTTER: &str = "https://example.org/cutter";

fn svg(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:plotter="{PLOTTER}" xmlns:cut="{CUTTER}"
            xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
            width="100" height="100">{body}</svg>"#
    )
}

fn named<'a>(doc: &'a Document, name: &str) -> &'a Node {
    let mut found = None;
    doc.walk(|n| {
        if n.name.as_deref() == Some(name) {
            found = Some(n);
        }
    });
    found.unwrap_or_else(|| panic!("no object {name}"))
}

fn foreign(doc: &Document, name: &str) -> Vec<ForeignAttribute> {
    named(doc, name).extras().attrs.as_deref().map(|a| a.foreign.clone()).unwrap_or_default()
}

fn attr(ns: &str, prefix: &str, name: &str, value: &str) -> ForeignAttribute {
    ForeignAttribute { ns: ns.into(), prefix: prefix.into(), name: name.into(), value: value.into() }
}

fn parsed(text: &str) -> roxmltree::Document<'_> {
    roxmltree::Document::parse(text).unwrap()
}

#[test]
fn other_applications_attributes_survive_a_round_trip() {
    let doc = import(&svg(r#"<g id="frame" plotter:pen="2"><rect id="shape" width="20" height="20" plotter:pen="3" plotter:speed="40"
            cut:passes="2" inkscape:connector-curvature="0" sodipodi:nodetypes="cccc" data-x="1"/></g>"#))
    .unwrap();
    let shape = [attr(PLOTTER, "plotter", "pen", "3"), attr(PLOTTER, "plotter", "speed", "40"), attr(CUTTER, "cut", "passes", "2")];
    assert_eq!(foreign(&doc, "shape"), shape, "Inkscape's own attributes aren't another application's");
    // A top-level group is a layer, and keeps them too (a plotter's pen per layer).
    assert!(matches!(named(&doc, "frame").kind, NodeKind::Layer { .. }), "{:?}", named(&doc, "frame").kind);
    assert_eq!(foreign(&doc, "frame"), [attr(PLOTTER, "plotter", "pen", "2")]);
    // Written back on the same elements, each namespace declared once on the root.
    let text = export(&doc, &ExportOptions::default());
    let xml = parsed(&text);
    let root = xml.root_element();
    assert_eq!((root.lookup_namespace_uri(Some("plotter")), root.lookup_namespace_uri(Some("cut"))), (Some(PLOTTER), Some(CUTTER)));
    let element = |id: &str| xml.descendants().find(|n| n.attribute("id") == Some(id)).unwrap();
    assert_eq!((element("shape").attribute((PLOTTER, "speed")), element("shape").attribute((CUTTER, "passes"))), (Some("40"), Some("2")));
    assert_eq!(element("frame").attribute((PLOTTER, "pen")), Some("2"));
    let back = import(&text).unwrap();
    assert_eq!((foreign(&back, "shape"), foreign(&back, "frame")), (shape.to_vec(), vec![attr(PLOTTER, "plotter", "pen", "2")]));
    // And through the native file.
    let attrs = named(&doc, "shape").extras().attrs.as_deref().unwrap().clone();
    let json = serde_json::to_value(&attrs).unwrap();
    assert_eq!(json["foreign"][0], serde_json::json!({"ns": PLOTTER, "prefix": "plotter", "name": "pen", "value": "3"}));
    assert_eq!(serde_json::from_value::<vectorcraft_doc::ObjectAttributes>(json).unwrap(), attrs);
}

/// Two namespaces under one prefix, a prefix the export uses itself and one XML keeps for itself
/// are each written under a prefix of their own; names and values XML can't hold are left out
/// with a warning.
#[test]
fn prefixes_are_made_unique_and_bad_attributes_left_out() {
    let mut doc = import(&svg(r#"<rect id="a" width="20" height="20"/><rect id="b" x="30" width="20" height="20"/>"#)).unwrap();
    let set = |doc: &mut Document, name: &str, attrs: Vec<ForeignAttribute>| {
        let id = named(doc, name).id;
        doc.node_mut(id).unwrap().edit_attrs(|a| a.foreign = attrs);
    };
    set(&mut doc, "a", vec![attr("urn:one", "p", "x", "1"), attr("urn:three", "xlink", "y", "3"), attr("urn:four", "xmlish", "z", "4")]);
    set(
        &mut doc,
        "b",
        vec![
            attr("urn:two", "p", "x", "2"),
            attr("urn:one", "p", "w", "5"),
            attr("urn:one", "p", "bad name", "6"),
            attr("urn:one", "p", "v", "\u{1}"),
        ],
    );
    let out = export_full(&doc, &ExportOptions::default());
    let text = String::from_utf8(out.bytes()).unwrap();
    let xml = parsed(&text);
    let element = |id: &str| xml.descendants().find(|n| n.attribute("id") == Some(id)).unwrap();
    assert_eq!(element("a").attribute(("urn:one", "x")), Some("1"));
    assert_eq!(element("b").attribute(("urn:two", "x")), Some("2"));
    assert_eq!(element("b").attribute(("urn:one", "w")), Some("5"));
    assert_eq!(element("a").attribute(("urn:three", "y")), Some("3"));
    assert_eq!(element("a").attribute(("urn:four", "z")), Some("4"));
    let root = xml.root_element();
    let prefixes: Vec<_> = ["urn:one", "urn:two", "urn:three", "urn:four"].map(|ns| root.lookup_prefix(ns)).to_vec();
    assert_eq!(prefixes, [Some("p"), Some("p2"), Some("xlink2"), Some("ns")]);
    assert!(out.warnings.iter().any(|w| w.contains("invalid XML names or characters")), "{:?}", out.warnings);
    assert_eq!(element("b").attributes().filter(|a| a.namespace() == Some("urn:one")).count(), 1, "only w");
}

/// The file is untrusted: at most 64 such attributes per element, each value at most 64 KiB.
#[test]
fn foreign_attributes_are_capped() {
    let many: String = (0..70).map(|i| format!(r#" plotter:a{i}="{i}""#)).collect();
    let long = "x".repeat(64 * 1024 + 1);
    let doc = import(&svg(&format!(
        r#"<rect id="many" width="20" height="20"{many}/><rect id="long" width="5" height="5" plotter:note="{long}" plotter:pen="1"/>"#
    )))
    .unwrap();
    assert_eq!(foreign(&doc, "many").len(), 64);
    assert_eq!(foreign(&doc, "long"), [attr(PLOTTER, "plotter", "pen", "1")]);
}
