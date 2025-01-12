use eframe::egui;
use egui_graph::node::{EdgeEvent, SocketKind};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashSet;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();
    let name = "`egui_graph` demo";
    eframe::run_native(name, options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

struct App {
    state: State,
}

struct State {
    graph: Graph,
    view: egui_graph::View,
    interaction: Interaction,
    flow: egui::Direction,
    socket_radius: f32,
    socket_color: egui::Color32,
    wire_width: f32,
    wire_color: egui::Color32,
    auto_layout: bool,
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

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_fonts(egui::FontDefinitions::default());
        let graph = new_graph();
        let state = State {
            graph,
            view: Default::default(),
            interaction: Default::default(),
            socket_color: ctx.style().visuals.weak_text_color(),
            socket_radius: 3.0,
            wire_width: 1.0,
            wire_color: ctx.style().visuals.weak_text_color(),
            flow: egui::Direction::TopDown,
            auto_layout: true,
        };
        App { state }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        gui(ctx, &mut self.state);
        if self.state.auto_layout {
            self.state.view.layout = layout(&self.state.graph, self.state.flow, ctx);
        }
    }
}

fn new_graph() -> Graph {
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
    graph
}

fn node(name: impl ToString, kind: NodeKind) -> Node {
    let name = name.to_string();
    Node { name, kind }
}

fn layout(graph: &Graph, flow: egui::Direction, ctx: &egui::Context) -> egui_graph::Layout {
    ctx.memory(|m| {
        let nodes = graph.node_indices().map(|n| {
            let id = egui::Id::new(n);
            let size = m
                .area_rect(id)
                .map(|a| a.size())
                .unwrap_or([200.0, 50.0].into());
            (id, size)
        });
        let edges = graph
            .edge_indices()
            .filter_map(|e| graph.edge_endpoints(e))
            .map(|(a, b)| (egui::Id::new(a), egui::Id::new(b)));
        egui_graph::layout(nodes, edges, flow)
    })
}

fn gui(ctx: &egui::Context, state: &mut State) {
    egui::containers::CentralPanel::default()
        .frame(egui::Frame::default())
        .show(ctx, |ui| {
            graph_config(ui, state);
            graph(ui, state);
        });
}

fn graph(ui: &mut egui::Ui, state: &mut State) {
    egui_graph::Graph::new("Demo Graph")
        .show(&mut state.view, ui)
        .nodes(|nctx, ui| nodes(nctx, ui, state))
        .edges(|ectx, ui| edges(ectx, ui, state));
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
                        ui.add(egui::DragValue::new(f).range(0.0..=255.0));
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

    let mouse_pos = ui.input(|i| i.pointer.interact_pos().unwrap_or_default());
    let click = ui.input(|i| i.pointer.any_released());
    let shift_held = ui.input(|i| i.modifiers.shift);
    let mut clicked_on_edge = false;
    let selection_threshold = state.wire_width * 8.0; // Threshold for selecting the edge

    for e in indices {
        let (na, nb) = state.graph.edge_endpoints(e).unwrap();
        let (output, input) = *state.graph.edge_weight(e).unwrap();
        let a = egui::Id::new(na);
        let b = egui::Id::new(nb);
        let a_out = ectx.output(ui, a, output).unwrap();
        let b_in = ectx.input(ui, b, input).unwrap();
        let bezier = egui_graph::bezier::Cubic::from_edge_points(a_out, b_in);
        let dist_per_pt = 5.0;
        let pts: Vec<_> = bezier.flatten(dist_per_pt).collect();

        // Check if mouse is over the bezier curve
        let closest_point = bezier.closest_point(dist_per_pt, egui::Pos2::from(mouse_pos));
        let distance_to_mouse = closest_point.distance(egui::Pos2::from(mouse_pos));
        if distance_to_mouse < selection_threshold && click {
            clicked_on_edge = true;
            // If Shift is not held, clear previous selection
            if !shift_held {
                state.interaction.selection.edges.clear();
            }
            // Add the clicked edge to the selection
            state.interaction.selection.edges.insert(e);
        }

        let wire_stroke = if state.interaction.selection.edges.contains(&e) {
            egui::Stroke {
                width: state.wire_width * 4.0,
                color: state.wire_color.linear_multiply(1.5),
            }
        } else {
            stroke
        };

        // Draw the bezier curve
        ui.ctx()
            .layer_painter(ectx.graph_bg_layer())
            .add(egui::Shape::line(pts.clone(), wire_stroke));
    }

    if click && !clicked_on_edge {
        // Click occurred on the canvas, clear the selection
        state.interaction.selection.edges.clear();
    }

    // Draw the in-progress edge if there is one.
    if let Some(edge) = ectx.in_progress(ui) {
        let bezier = edge.bezier_cubic();
        let dist_per_pt = 5.0;
        let pts = bezier.flatten(dist_per_pt).collect();
        ui.painter().add(egui::Shape::line(pts, stroke));
    }

    // Remove selected edges if delete/backspace is pressed
    if ui.input(|i| i.key_pressed(egui::Key::Delete) | i.key_pressed(egui::Key::Backspace)) {
        state.interaction.selection.edges.iter().for_each(|e| {
            state.graph.remove_edge(*e);
        });
        state.interaction.selection.nodes.clear();
    }
}

fn graph_config(ui: &mut egui::Ui, state: &mut State) {
    let mut frame = egui::Frame::window(ui.style());
    frame.shadow.offset = egui::Vec2::ZERO;
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
                ui.checkbox(&mut state.auto_layout, "Automatic Layout");
                ui.separator();
                ui.add_enabled_ui(!state.auto_layout, |ui| {
                    if ui.button("Layout Once").clicked() {
                        state.view.layout = layout(&state.graph, state.flow, ui.ctx());
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label("Flow:");
                ui.radio_value(&mut state.flow, egui::Direction::LeftToRight, "Right");
                ui.radio_value(&mut state.flow, egui::Direction::TopDown, "Down");
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
}
