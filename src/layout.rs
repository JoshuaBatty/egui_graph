//! Items related to automated layout of nodes.

use crate::Layout;
use egui::Direction;
use layout::{
    core::{base::Orientation, geometry::Point, style::*},
    std_shapes::shapes::*,
    topo::{layout::VisualGraph, placer::Placer},
};
use std::collections::HashMap;

/// Constructs a layout for a directed graph based on given edges and node sizes.
///
/// Returns a `Layout` object representing the positions of nodes in the graph.
pub fn layout(
    nodes: impl IntoIterator<Item = (egui::Id, egui::Vec2)>,
    edges: impl IntoIterator<Item = (egui::Id, egui::Id)>,
    flow: egui::Direction,
) -> Layout {
    let orientation = match flow {
        Direction::LeftToRight | Direction::RightToLeft => Orientation::LeftToRight,
        Direction::TopDown | Direction::BottomUp => Orientation::TopToBottom,
    };
    let mut vg = VisualGraph::new(orientation);

    let mut handles = HashMap::new();
    let mut ids = Vec::new();
    for (id, size) in nodes {
        let shape = ShapeKind::new_box("");
        let style = StyleAttr::simple();
        let size = Point::new(size.x.into(), size.y.into());
        eprintln!("size: {:?}", size);
        let node = Element::create(shape, style, orientation, size);
        let handle = vg.add_node(node);
        handles.insert(id, handle);
        ids.push((handle, id));
    }

    for (a, b) in edges {
        let edge = Arrow::default();
        let na = handles[&a];
        let nb = handles[&b];
        vg.add_edge(edge, na, nb);
    }

    vg.to_valid_dag();
    vg.split_text_edges();
    let disable_opts = false;
    vg.split_long_edges(disable_opts);
    Placer::new(&mut vg).layout(true);

    // `layout-rs` lays out nodes down and to the right starting from the middle.
    // We find the bounding box and shift the nodes up and to the right in order
    // to centre them under the default `camera` position.
    let mut min: Option<[f64; 2]> = None;
    let mut max: Option<[f64; 2]> = None;
    let mut layout = Layout::new();
    for (handle, id) in ids {
        let mut pos = vg.pos(handle);
        // pos.set_x(pos.center().x * 2.0);
        // pos.set_y(pos.center().y * 2.0);
        
        let with_halo = false;
        let (tl, br) = pos.bbox(with_halo);
        eprintln!("pos: {:?} | br: {:?} | tl: {:?}", pos, br, tl);
        let pos = [tl.x as f32, tl.y as f32].into();
        layout.insert(id, pos);
        if let Some([x, y]) = min.as_mut() {
            *x = (*x).min(tl.x);
            *y = (*y).min(tl.y);
        } else {
            min = Some([tl.x, tl.y]);
        }
        if let Some([x, y]) = max.as_mut() {
            *x = (*x).max(br.x);
            *y = (*y).max(br.y);
        } else {
            max = Some([br.x, br.y]);
        }
    }
    if let (Some(min), Some(max)) = (min, max) {
        let half_w = ((max[0] - min[0]) / 2.0) as f32;
        let half_h = ((max[1] - min[1]) / 2.0) as f32;
        for p in layout.values_mut() {
            p.x -= half_w;
            p.y -= half_h;
        }
    }

    layout
}
