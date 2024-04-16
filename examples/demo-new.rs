use eframe::egui;
use egui_graph::graph::Camera;
use egui_graph::node::{EdgeEvent, SocketKind};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashSet;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();
    let name = "`egui_graph` demo";
    eframe::run_native(name, options, Box::new(|cc| Box::new(App::new(cc))))
}

struct App {
    state: State,
}

struct State {
    graph: Graph,
    camera: Camera,
    // view: egui_graph::View,
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
            camera: Default::default(),
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
        // if self.state.auto_layout {
        //     self.state.view.layout = layout(&self.state.graph, self.state.flow, ctx);
        // }
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

fn gui(ctx: &egui::Context, state: &mut State) {
    egui::containers::CentralPanel::default()
        .frame(egui::Frame::default())
        .show(ctx, |ui| {
            // graph_config(ui, state);
            graph(ui, state);
        });
}

fn graph(ui: &mut egui::Ui, state: &mut State) {
    egui_graph::graph::Graph::new("Demo Graph")
        // asdf
        .show(&mut state.camera, ui);
    // .nodes(|nctx, ui| nodes(nctx, ui, state))
    // .edges(|ectx, ui| edges(ectx, ui, state));
}

// fn nodes
