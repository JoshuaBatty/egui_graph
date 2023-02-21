use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use egui_graph::node::{EdgeEvent, SocketKind};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashSet;

fn main() {
    App::new()
        .init_resource::<State>()
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_startup_system(initialize)
        .add_system(update)
        .run();
}

fn initialize(mut commands: Commands, mut egui: ResMut<EguiContext>) {
    egui.ctx_mut().set_fonts(egui::FontDefinitions::default());
    egui.ctx_mut().set_style(style());

    // The graph we want to inspect/edit.
    let mut graph = Graph::new();
    let a = graph.add_node(node("Foo", NodeKind::Label));
    let b = graph.add_node(node("Bar", NodeKind::Button));
    let c = graph.add_node(node("Baz", NodeKind::Slider(0.5)));
    let d = graph.add_node(node("Qux", NodeKind::DragValue(20.0)));
    let comment = "Nodes are a thin wrapper around the `egui::Window`, \
        allowing you to set arbitrary widgets.";
    let e = graph.add_node(node("Fiz", NodeKind::Comment(comment.to_string())));
    graph.add_edge(a, c, (0, 0));
    graph.add_edge(a, d, (1, 1));
    graph.add_edge(b, d, (0, 2));
    graph.add_edge(c, d, (0, 0));
    graph.add_edge(d, e, (0, 0));

    // Describes the camera position and layout of the nodes.
    let mut view = egui_graph::View::default();

    // We don't have any fancy automatic layout yet, so set some reasonable initial positions.
    view.layout.insert(egui::Id::new(a), [-400.0, 0.0].into());
    view.layout.insert(egui::Id::new(b), [-150.0, 100.0].into());
    view.layout
        .insert(egui::Id::new(c), [-200.0, -100.0].into());
    view.layout.insert(egui::Id::new(d), [50.0, 0.0].into());
    view.layout.insert(egui::Id::new(e), [200.0, 0.0].into());

    let color = egui::Color32::from_gray(140);
    commands.insert_resource(State {
        graph,
        view,
        socket_color: color,
        wire_color: color,
        ..State::default()
    });
}

fn update(mut egui_context: ResMut<EguiContext>, mut state: ResMut<State>) {
    egui::containers::CentralPanel::default()
        .frame(egui::Frame::default())
        .show(&egui_context.ctx_mut(), |ui| {
            egui_graph::Graph::new("Demo Graph")
                .show(&mut state.view, ui)
                .nodes(|nctx, ui| nodes(nctx, ui, &mut state))
                .edges(|ectx, ui| edges(ectx, ui, &mut state));

            // Overlay some configuration for the graph.
            let mut frame = egui::Frame::window(ui.style());
            frame.shadow.extrusion = 0.0;
            egui::Window::new("Graph Config")
                .frame(frame)
                .anchor(
                    egui::Align2::LEFT_TOP,
                    ui.spacing().window_margin.left_top(),
                )
                .collapsible(false)
                .title_bar(false)
                .auto_sized()
                .show(ui.ctx(), |ui| {
                    ui.label("GRAPH CONFIG");
                    ui.horizontal(|ui| {
                        ui.label("Flow:");
                        ui.radio_value(&mut state.flow, egui::Direction::LeftToRight, "Right");
                        ui.radio_value(&mut state.flow, egui::Direction::TopDown, "Down");
                        ui.radio_value(&mut state.flow, egui::Direction::RightToLeft, "Left");
                        ui.radio_value(&mut state.flow, egui::Direction::BottomUp, "Up");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Wire curvature:");
                        ui.add(egui::Slider::new(&mut state.wire_curvature, 0.0..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Wire width:");
                        ui.add(egui::Slider::new(&mut state.wire_width, 0.5..=10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Socket radius:");
                        ui.add(egui::Slider::new(&mut state.socket_radius, 1.0..=10.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Wire color:");
                        ui.color_edit_button_srgba(&mut state.wire_color);
                        ui.label("Socket color:");
                        ui.color_edit_button_srgba(&mut state.socket_color);
                    });
                });
        });
}

#[derive(Resource)]
struct State {
    graph: Graph,
    view: egui_graph::View,
    interaction: Interaction,
    flow: egui::Direction,
    socket_radius: f32,
    socket_color: egui::Color32,
    wire_width: f32,
    wire_color: egui::Color32,
    wire_curvature: f32,
}

#[derive(Default)]
struct Interaction {
    selection: Selection,
    edge_in_progress: Option<(NodeIndex, SocketKind, usize)>,
}

#[derive(Default)]
struct Selection {
    nodes: HashSet<NodeIndex>,
    edges: HashSet<EdgeIndex>,
}

type Graph = petgraph::stable_graph::StableGraph<Node, (usize, usize)>;

struct Node {
    name: String,
    kind: NodeKind,
}

enum NodeKind {
    Label,
    Button,
    Slider(f32),
    DragValue(f32),
    Comment(String),
}

fn nodes(nctx: &mut egui_graph::NodesCtx, ui: &mut egui::Ui, state: &mut State) {
    let indices: Vec<_> = state.graph.node_indices().collect();
    for n in indices {
        let inputs = state
            .graph
            .edges_directed(n, petgraph::Incoming)
            .fold(0, |max, e| std::cmp::max(max, e.weight().1 + 1));
        let outputs = state
            .graph
            .edges_directed(n, petgraph::Outgoing)
            .fold(0, |max, e| std::cmp::max(max, e.weight().0 + 1));
        let node = &mut state.graph[n];
        let graph_view = &mut state.view;
        let response = egui_graph::node::Node::new(n)
            .inputs(inputs)
            .outputs(outputs)
            .flow(state.flow)
            .socket_radius(state.socket_radius)
            .socket_color(state.socket_color)
            .show(graph_view, nctx, ui, |ui| match node.kind {
                NodeKind::Label => {
                    ui.label(&node.name);
                }
                NodeKind::Button => {
                    ui.horizontal(|ui| {
                        if ui.button(&node.name).clicked() {
                            println!("{}", node.name);
                        }
                    });
                }
                NodeKind::DragValue(ref mut f) => {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(f).clamp_range(0.0..=255.0));
                    });
                }
                NodeKind::Slider(ref mut f) => {
                    ui.horizontal(|ui| ui.add(egui::Slider::new(f, 0.0..=1.0)));
                }
                NodeKind::Comment(ref mut text) => {
                    ui.text_edit_multiline(text);
                }
            });

        if response.changed() {
            // Keep track of the selected nodes.
            if let Some(selected) = response.selection() {
                if selected {
                    assert!(state.interaction.selection.nodes.insert(n));
                } else {
                    assert!(state.interaction.selection.nodes.remove(&n));
                }
            }

            // Check for an edge event.
            if let Some(ev) = response.edge_event() {
                dbg!(&ev);
                match ev {
                    EdgeEvent::Started { kind, index } => {
                        state.interaction.edge_in_progress = Some((n, kind, index));
                    }
                    EdgeEvent::Ended { kind, index } => {
                        // Create the edge.
                        if let Some((src, _, ix)) = state.interaction.edge_in_progress.take() {
                            let (a, b, w) = match kind {
                                SocketKind::Input => (src, n, (ix, index)),
                                SocketKind::Output => (n, src, (index, ix)),
                            };
                            // Check that this edge doesn't already exist.
                            if !state
                                .graph
                                .edges(a)
                                .any(|e| e.target() == b && *e.weight() == w)
                            {
                                state.graph.add_edge(a, b, w);
                            }
                        }
                    }
                    EdgeEvent::Cancelled => {
                        state.interaction.edge_in_progress = None;
                    }
                }
            }

            // If the delete key was pressed while selected, remove it.
            if response.removed() {
                state.graph.remove_node(n);
            }
        }
    }
}

fn edges(ectx: &mut egui_graph::EdgesCtx, ui: &mut egui::Ui, state: &mut State) {
    // Draw the attached edges.
    let indices: Vec<_> = state.graph.edge_indices().collect();
    let stroke = egui::Stroke {
        width: state.wire_width,
        color: state.wire_color,
    };
    let wire_curvature = Some(state.wire_curvature);
    for e in indices {
        let (na, nb) = state.graph.edge_endpoints(e).unwrap();
        let (output, input) = *state.graph.edge_weight(e).unwrap();
        let a = egui::Id::new(na);
        let b = egui::Id::new(nb);
        let a_out = ectx.output(ui, a, output).unwrap();
        let b_in = ectx.input(ui, b, input).unwrap();
        let bezier = egui_graph::bezier::Cubic::from_edge_points(a_out, b_in, wire_curvature);
        let dist_per_pt = 5.0;
        let pts: Vec<_> = bezier.flatten(dist_per_pt).collect();
        ui.painter().add(egui::Shape::line(pts.clone(), stroke));
    }

    // Draw the in-progress edge if there is one.
    if let Some(edge) = ectx.in_progress(ui) {
        let bezier = edge.bezier_cubic(wire_curvature);
        let dist_per_pt = 5.0;
        let pts = bezier.flatten(dist_per_pt).collect();
        ui.painter().add(egui::Shape::line(pts, stroke));
    }
}

fn node(name: impl ToString, kind: NodeKind) -> Node {
    let name = name.to_string();
    Node { name, kind }
}

// TODO: Remove this. Just use defaults for example.
fn style() -> egui::Style {
    let mut style = egui::Style::default();
    style.spacing = egui::style::Spacing {
        item_spacing: egui::Vec2::splat(8.0),
        button_padding: egui::Vec2::new(4.0, 2.0),
        interact_size: egui::Vec2::new(56.0, 24.0),
        indent: 10.0,
        icon_width: 20.0,
        icon_spacing: 1.0,
        ..style.spacing
    };
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::WHITE;
    style.visuals.extreme_bg_color = egui::Color32::from_gray(12);
    style.visuals.faint_bg_color = egui::Color32::from_gray(24);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_gray(36);
    style.visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::BLACK;
    style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
    style
}

impl Default for State {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            view: Default::default(),
            interaction: Default::default(),
            socket_color: Default::default(),
            socket_radius: 3.0,
            wire_width: 1.0,
            wire_curvature: 0.5,
            wire_color: Default::default(),
            flow: egui::Direction::LeftToRight,
        }
    }
}
