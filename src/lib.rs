use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use egui::emath::TSTransform;
#[cfg(feature = "layout")]
pub use layout::layout;

pub mod bezier;
#[cfg(feature = "layout")]
pub mod layout;
pub mod node;

/// The main interface for the `Graph` widget.
pub struct Graph {
    background: bool,
    id: egui::Id,
}

/// State related to the graph UI.
#[derive(Clone, Default)]
pub struct GraphTempMemory {
    /// The most recently recorded size of each node.
    ///
    /// Primarily used to check for node selection, as we don't know the size of
    /// the node until the contents have been instantiated.
    node_sizes: NodeSizes,
    /// The currently selected nodes and edges.
    selection: Selection,
    /// Whether or not the primary button was pressed on the graph area and is still down.
    ///
    /// Used for tracking selection and dragging.
    pressed: Option<Pressed>,
    /// Collect information about the layout of each node's sockets during node instantiation.
    ///
    /// This is used to provide the position and normal of each socket when instantiating edges.
    sockets: HashMap<egui::Id, NodeSockets>,
    /// The socket that is currently closest to the mouse.
    ///
    /// Always `Some` while the pointer is over the graph area, `None` otherwise.
    closest_socket: Option<node::Socket>,
}

type NodeSizes = HashMap<egui::Id, egui::Vec2>;

#[derive(Clone, Default)]
struct Selection {
    /// The set of currently selected nodes.
    nodes: HashSet<egui::Id>,
    /// The set of currently selected edges.
    edges: HashSet<(node::Socket, node::Socket)>,
}

/// State related to the last press of the primary pointer button over the graph.
#[derive(Clone, Debug)]
struct Pressed {
    /// Whether or not the pointer is currently over one of the selected nodes.
    ///
    /// This is used to assist with determining whether or not nodes should deselect. E.g. if
    /// multiple nodes are selected and a non-selected node is pressed, then we should deselect the
    /// originally selected nodes. However, if a selected node is pressed, then the selection
    /// should stay the same and a drag will begin.
    over_selection_at_origin: bool,
    /// The origin of the pointer over the graph at the begining of the press.
    ///
    /// Position is in graph coordinates (relative to the centre of the graph). This is to allow
    /// for selections/drags larger than the visible area of the graph (e.g. if the camera is moved
    /// during selection to cover a larger area).
    origin_pos: egui::Pos2,
    /// Position is in graph coordinates (relative to the centre of the graph).
    current_pos: egui::Pos2,
    /// The action performed by this press.
    action: PressAction,
}

#[derive(Clone, Debug)]
enum PressAction {
    /// A node was pressed and a drag is taking place.
    DragNodes {
        /// The node that was pressed to initiate the drag.
        ///
        /// We don't know exactly which until the node itself emits the pressed event, so this
        /// remains `None` until we know.
        node: Option<PressedNode>,
    },
    /// The graph was pressed and we are performing a selection.
    Select,
    /// A node's socket was pressed in order to start creating a connection.
    Socket(node::Socket),
}

#[derive(Clone, Debug)]
struct PressedNode {
    /// Unique Id of th node.
    id: egui::Id,
    /// The position of the node over the graph at the origin of the press.
    position_at_origin: egui::Pos2,
}

/// Configuration for the graph.
// TODO: Consider storing this in graph widget "memory"?
// The thing is, it might be nice to let the user modify these externally.
#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct View {
    pub camera: Camera,
    pub layout: Layout,
}

/// The location of the top-left of each node relative to the centre of the graph area.
pub type Layout = HashMap<egui::Id, egui::Pos2>;

/// A top-down camera-like view over the area of the graph.
#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Camera {
    /// Location of the camera relative to the center of the graph.
    pub transform: TSTransform,
}

/// The context returned by the `Graph` widget. Allows for setting nodes and edges.
pub struct Show<'a> {
    /// Useful for accessing the `GraphTempMemory`.
    graph_id: egui::Id,
    /// The rect allocated in the UI for the graph.
    graph_rect: egui::Rect,
    /// The layer of the rect allocated for the graph.
    graph_layer: egui::LayerId,
    /// The layer of the background response returned from the dot grid.
    graph_bg_layer: egui::LayerId,
    /// Track all nodes that were visited this update.
    ///
    /// We will use this to remove old node state on `drop`.
    visited: HashSet<egui::Id>,
    /// The child UI of the `Graph` widget for instantiating nodes and edges.
    ui: &'a mut egui::Ui,
}

// /// The full area covered by the `Graph` within the UI.
// full_rect: egui::Rect,
// /// The visible area of the graph's canvas.
// visible_rect: egui::Rect,
// /// If a selection is being performed with the mouse, this is the covered area.
// selection_rect: Option<egui::Rect>,
// /// Whether or not the primary mouse button was just released to perform the selection.
// select: bool,
// /// Whether or not the primary mouse button was just released to end edge creation.
// socket_press_released: Option<node::Socket>,

/// Information about the inputs and outputs for a particular node.
#[derive(Clone)]
pub struct NodeSockets {
    flow: egui::Direction,
    input: Sockets,
    output: Sockets,
}

/// The description of how the inputs and outputs are laid out for a particular node.
#[derive(Clone)]
pub struct Sockets {
    count: usize,
    layout: SocketLayout,
}

/// Inlet/outlet layout.
#[derive(Clone)]
struct SocketLayout {
    start: egui::Pos2,
    step: egui::Vec2,
}

/// A context to assist with the instantiation of node widgets.
pub struct NodesCtx<'a> {
    graph_id: egui::Id,
    graph_rect: egui::Rect,
    graph_layer: egui::LayerId,
    graph_bg_layer: egui::LayerId,
    visited: &'a mut HashSet<egui::Id>,
    // full_rect: egui::Rect,
    // selection_rect: Option<egui::Rect>,
    // select: bool,
    // socket_press_released: Option<node::Socket>,
}

/// A context to assist with the instantiation of edge widgets.
pub struct EdgesCtx {
    graph_id: egui::Id,
    graph_bg_layer: egui::LayerId,
    graph_rect: egui::Rect,
    // visible_rect: egui::Rect,
}

/// The set of detected graph interaction for a single graph widget update prior
/// to node interaction.
struct GraphInteraction {
    pressed: Option<Pressed>,
    socket_press_released: Option<node::Socket>,
    select: bool,
    selection_rect: Option<egui::Rect>,
    drag_nodes_delta: egui::Vec2,
}

impl Graph {
    /// Begin building the new graph widget.
    pub fn new(id_src: impl Hash) -> Self {
        Self {
            background: true,
            id: id(id_src),
        }
    }

    /// Whether or not to fill the background. Default is `true`.
    pub fn background(mut self, show: bool) -> Self {
        self.background = show;
        self
    }

    /// Begin showing the parts of the Graph.
    pub fn show<'ui>(self, view: &mut View, ui: &'ui mut egui::Ui) -> Show<'ui> {
        let (id, graph_rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(graph_rect, id, egui::Sense::click_and_drag());

        // Allow dragging the background as well.
        if response.dragged_by(egui::PointerButton::Middle) {
            view.camera.transform.translation += response.drag_delta();
        }

        // Plot-like reset
        if response.double_clicked() {
            view.camera.transform = TSTransform::default();
        }

        let transform = TSTransform::from_translation(ui.min_rect().left_top().to_vec2())
            * view.camera.transform;

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Note: doesn't catch zooming / panning if a button in this PanZoom container is hovered.
            if response.hovered() {
                let pointer_in_layer = transform.inverse() * pointer;
                let (zoom_delta, pan_delta) =
                    ui.ctx().input(|i| (i.zoom_delta(), i.smooth_scroll_delta));

                // Zoom in on pointer:
                view.camera.transform = view.camera.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                view.camera.transform =
                    TSTransform::from_translation(pan_delta) * view.camera.transform;
            }
        }

        // Paint the background rect.
        if self.background {
            paint_background(graph_rect, ui);
        }

        // Paint some subtle dots to check camera movement.
        let bg_response = dot_grid(id, graph_rect, view.camera.transform, ui);

        // ------------------

        // The full area to be occuppied by the graph.
        let full_rect = graph_rect;

        // Draw the selection rectangle if there is one.
        let mut selection_rect = None;
        let mut select = false;
        let mut socket_press_released = None;

        let ptr_on_graph = response.contains_pointer();

        // Check for selection rectangle and node dragging.
        let gmem_arc = memory(ui, self.id);
        let mut gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
        let pointer = ui.input(|i| i.pointer.clone());
        if let Some(ptr_screen) = pointer.interact_pos().or(pointer.hover_pos()) {
            // The location of the pointer over the graph.
            let ptr_graph = view.camera.screen_to_graph(full_rect, ptr_screen);

            // Check for the closest socket.
            let closest_socket = match ptr_on_graph {
                true => find_closest_socket(ptr_screen, full_rect, view, &gmem, ui)
                    .map(|(socket, _dist_sqrd)| socket),
                false => None,
            };

            // Check for graph interactions.
            let interaction = graph_interaction(
                full_rect,
                &gmem.node_sizes,
                &gmem.selection,
                &*view,
                &pointer,
                closest_socket,
                ptr_on_graph,
                ptr_screen,
                ptr_graph,
                gmem.pressed.as_ref(),
                ui,
            );

            // Apply drag delta to all selected nodes.
            if interaction.drag_nodes_delta != egui::Vec2::ZERO {
                for &n_id in &gmem.selection.nodes {
                    if let Some(pos) = view.layout.get_mut(&n_id) {
                        *pos += interaction.drag_nodes_delta;
                    }
                }
            }
            gmem.pressed = interaction.pressed;
            gmem.closest_socket = closest_socket;
            selection_rect = interaction.selection_rect;
            select = interaction.select;
            socket_press_released = interaction.socket_press_released;

            // FIXME: Remove this? Dragging handled by Area?
            // If the pointer is down and near an edge of the rect, move the camera in that
            // direction.
            // if drag_moves_camera(gmem.pressed.as_ref(), ptr_graph) {
            //     view.camera.pos += drag_moves_camera_velocity(full_rect, ptr_screen, ui);
            // }
        }

        // FIXME: Camera handled by Area?
        // // Check if we should drag or scroll the camera position.
        // if !ptr_in_use && ptr_on_graph {
        //     ui.input(|i| {
        //         // Middle mouse button moves camera.
        //         if i.pointer.is_moving() && i.pointer.button_down(egui::PointerButton::Middle) {
        //             view.camera.pos -= pointer.delta();
        //         }
        //     });
        // }

        // Draw the selection area if there is one.
        // TODO: Do this when `Show` is `drop`ped or finalised to draw over nodes, edges.
        if let Some(sel_rect) = selection_rect {
            paint_selection_area(sel_rect, ui);
        }

        // // Create a child UI over the full surface of the graph widget.
        // let mut ui = ui.child_ui(full_rect, *ui.layout(), None);
        // ui.set_clip_rect(full_rect);

        Show {
            graph_id: self.id,
            graph_rect,
            graph_layer: response.layer_id,
            graph_bg_layer: bg_response.layer_id,
            visited: Default::default(),
            ui,
            // full_rect,
            // visible_rect,
            // selection_rect,
            // select,
            // socket_press_released,
        }
    }
}

/// Interpret some basic interactions from the state of the graph and recent input.
fn graph_interaction(
    full_rect: egui::Rect,
    node_sizes: &NodeSizes,
    selection: &Selection,
    view: &View,
    pointer: &egui::PointerState,
    closest_socket: Option<node::Socket>,
    ptr_on_graph: bool,
    ptr_screen: egui::Pos2,
    ptr_graph: egui::Pos2,
    pressed: Option<&Pressed>,
    ui: &egui::Ui,
) -> GraphInteraction {
    let mut select = false;
    let mut socket_press_released = None;
    let mut drag_nodes_delta = egui::Vec2::ZERO;
    let mut selection_rect = None;

    // Check for selecting/dragging.
    let pressed: Option<Pressed> = if let Some(pressed) = pressed {
        match pressed.action {
            PressAction::DragNodes {
                node: Some(ref node),
            } => {
                // Determine the drag delta.
                let delta = ptr_graph - pressed.origin_pos;
                let target = node.position_at_origin + delta;
                if let Some(current) = view.layout.get(&node.id) {
                    drag_nodes_delta = target - *current;
                }
            }
            PressAction::Select => {
                let min = view.camera.graph_to_screen(full_rect, pressed.origin_pos);
                let max = ptr_screen;
                selection_rect = Some(egui::Rect::from_two_pos(min, max));
            }
            _ => (),
        }

        // The press action has ended.
        if pointer.any_released() && !pointer.button_down(egui::PointerButton::Primary) {
            match pressed.action {
                PressAction::Select => select = true,
                PressAction::Socket(socket) => socket_press_released = Some(socket),
                _ => (),
            }
            None
        } else {
            Some(Pressed {
                current_pos: ptr_graph,
                ..pressed.clone()
            })
        }
    // Check for the beginning of a socket press or rectangular selection.
    } else if ptr_on_graph
        && pointer.button_down(egui::PointerButton::Primary)
        && pointer.any_pressed()
    {
        // Choose which press action based on whether or not a socket was pressed.
        let action = match closest_socket {
            Some(socket) => PressAction::Socket(socket),
            None => {
                let min = ptr_screen;
                let max = ptr_screen;
                selection_rect = Some(egui::Rect::from_two_pos(min, max));
                PressAction::Select
            }
        };

        let pressed = Pressed {
            over_selection_at_origin: false,
            origin_pos: ptr_graph,
            current_pos: ptr_graph,
            action,
        };
        Some(pressed)

    // Otherwise, check if we should start dragging nodes.
    } else if full_rect.contains(ptr_screen)
        && pointer.button_down(egui::PointerButton::Primary)
        && pointer.any_pressed()
    {
        // Check if the mouse is over a selected node.
        let mut over_any = false;
        let mut over_selected = false;
        let aim_radius = ui.input(|i| i.aim_radius());
        for (&n_id, &size) in node_sizes {
            let pos = view.layout.get(&n_id).cloned().unwrap_or(egui::Pos2::ZERO);
            let r = egui::Rect::from_min_size(pos, size);
            if r.expand(aim_radius).contains(ptr_graph) {
                over_any = true;
                if selection.nodes.contains(&n_id) {
                    over_selected = true;
                    break;
                }
            }
        }
        if over_any {
            Some(Pressed {
                over_selection_at_origin: over_selected,
                origin_pos: ptr_graph,
                current_pos: ptr_graph,
                action: PressAction::DragNodes { node: None },
            })
        } else {
            pressed.cloned()
        }

    // Otherwise, pass through existing state.
    } else {
        pressed.cloned()
    };

    GraphInteraction {
        pressed,
        socket_press_released,
        select,
        selection_rect,
        drag_nodes_delta,
    }
}

/// Find the socket that is closest to the given point.
///
/// Returns the socket alongside the squared distance from the socket.
fn find_closest_socket(
    pos_screen: egui::Pos2,
    full_rect: egui::Rect,
    view: &View,
    gmem: &GraphTempMemory,
    ui: &egui::Ui,
) -> Option<(node::Socket, f32)> {
    // TODO: if we wanted to be super efficient, we could maintain a quadtree of
    // nodes and sockets...
    let mut closest_socket = None;
    let socket_radius = ui
        .spacing()
        .interact_size
        .x
        .min(ui.spacing().interact_size.y);
    let socket_radius_sq = socket_radius * socket_radius;
    for (&n_id, &n_graph) in &view.layout {
        // Only check visible nodes.
        let n_screen = view.camera.graph_to_screen(full_rect, n_graph);
        let size = match gmem.node_sizes.get(&n_id) {
            None => continue,
            Some(&size) => size,
        };
        let rect = egui::Rect::from_min_size(n_screen, size);
        if !full_rect.intersects(rect) {
            continue;
        }
        let sockets = match gmem.sockets.get(&n_id) {
            None => continue,
            Some(sockets) => sockets,
        };

        // Check inputs.
        for (ix, (p, _)) in sockets.inputs().enumerate() {
            let dist_sq = pos_screen.distance_sq(p);
            if dist_sq < socket_radius_sq {
                let socket = node::Socket {
                    node: n_id,
                    kind: node::SocketKind::Input,
                    index: ix,
                };
                closest_socket = match closest_socket {
                    None => Some((socket, dist_sq)),
                    Some((_, d_sq)) if dist_sq < d_sq => Some((socket, dist_sq)),
                    _ => closest_socket,
                }
            }
        }

        // Check outputs.
        for (ix, (p, _)) in sockets.outputs().enumerate() {
            let dist_sq = pos_screen.distance_sq(p);
            if dist_sq < socket_radius_sq {
                let socket = node::Socket {
                    node: n_id,
                    kind: node::SocketKind::Output,
                    index: ix,
                };
                closest_socket = match closest_socket {
                    None => Some((socket, dist_sq)),
                    Some((_, d_sq)) if dist_sq < d_sq => Some((socket, dist_sq)),
                    _ => closest_socket,
                }
            }
        }
    }

    closest_socket
}

/// Whether or not the given `Pressed` state implies that the camera should move
/// due to the pointer being down near the edge of the rect.
fn drag_moves_camera(pressed: Option<&Pressed>, ptr_graph: egui::Pos2) -> bool {
    match pressed.as_ref() {
        None => false,
        Some(p) => {
            let action_ok =
                !matches!(p.action, PressAction::DragNodes { ref node, .. } if node.is_none());
            action_ok && p.origin_pos != ptr_graph
        }
    }
}

/// The velocity of the camera movement caused by dragging near the edge of
/// the rect.
fn drag_moves_camera_velocity(
    full_rect: egui::Rect,
    ptr_screen: egui::Pos2,
    ui: &egui::Ui,
) -> egui::Vec2 {
    let max_vel = 8.0;
    let mid = full_rect.center();
    let move_dist = ui
        .spacing()
        .interact_size
        .x
        .max(ui.spacing().interact_size.y);
    let x_vel = if ptr_screen.x < mid.x {
        (ptr_screen.x - (full_rect.min.x + move_dist)).min(0.0) * max_vel / move_dist
    } else if ptr_screen.x > mid.x {
        (ptr_screen.x - (full_rect.max.x - move_dist)).max(0.0) * max_vel / move_dist
    } else {
        0.0
    };
    let y_vel = if ptr_screen.y < mid.y {
        (ptr_screen.y - (full_rect.min.y + move_dist)).min(0.0) * max_vel / move_dist
    } else if ptr_screen.y > mid.y {
        (ptr_screen.y - (full_rect.max.y - move_dist)).max(0.0) * max_vel / move_dist
    } else {
        0.0
    };
    egui::Vec2::new(x_vel, y_vel)
}

// Paint the background rect.
fn paint_background(full_rect: egui::Rect, ui: &mut egui::Ui) {
    let vis = ui.style().noninteractive();
    let stroke = egui::Stroke {
        width: 0.0,
        ..vis.bg_stroke
    };
    let fill = vis.bg_fill;
    ui.painter().rect(full_rect, 0.0, fill, stroke);
}

// Paint some subtle dots to make it easy to track camera movement with no nodes.
// The given transform is the camera's transform used to transform the layer.
fn dot_grid(
    graph_id: egui::Id,
    graph_rect: egui::Rect,
    transform: TSTransform,
    ui: &egui::Ui,
) -> egui::Response {
    let graph_layer = ui.layer_id();
    let response = egui::Area::new(graph_id.with("background"))
        .default_pos(egui::Pos2::new(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            // Clip the view to the viewable area.
            let clip_rect = transform.inverse() * graph_rect;
            ui.set_clip_rect(clip_rect);

            // Use the interaction size as a rough gap.
            let dot_step = ui.spacing().interact_size.y;
            let vis = ui.style().noninteractive();
            let rect = graph_rect;
            let x_dots = (rect.min.x / dot_step) as i32..=(rect.max.x / dot_step) as i32;
            let y_dots = (rect.min.y / dot_step) as i32..=(rect.max.y / dot_step) as i32;
            let x_start = (clip_rect.min.x / dot_step).floor() * dot_step;
            let y_start = (clip_rect.min.y / dot_step).floor() * dot_step;

            for x_dot in x_dots {
                for y_dot in y_dots.clone() {
                    let x = x_start + x_dot as f32 * dot_step;
                    let y = y_start + y_dot as f32 * dot_step;
                    let r = egui::Rect::from_center_size([x, y].into(), [1.0; 2].into());
                    let color = vis.bg_stroke.color;
                    let stroke = egui::Stroke {
                        width: 0.0,
                        ..vis.bg_stroke
                    };
                    ui.painter().rect(r, 0.0, color, stroke);
                }
            }
        })
        .response;
    let graph_bg_layer = response.layer_id;
    ui.ctx().set_transform_layer(graph_bg_layer, transform);
    ui.ctx().set_sublayer(graph_layer, graph_bg_layer);
    response
}

/// Paint the selection area rectangle.
fn paint_selection_area(sel_rect: egui::Rect, ui: &mut egui::Ui) {
    let color = ui.visuals().weak_text_color();
    let fill = color.linear_multiply(0.125);
    let width = 1.0;
    let stroke = egui::Stroke { width, color };
    ui.painter().rect(sel_rect, 0.0, fill, stroke);
}

impl Camera {
    /// Convert the given point `pos` from graph space (position is relative to centre of
    /// graph) to screen space (where the point is currently visible within the UI).
    pub fn graph_to_screen(&self, graph_rect: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
        self.transform * pos
        // graph_to_screen(self.pos(), graph_rect, pos)
    }

    /// Convert the given point `pos` from screen space (where the point is currently
    /// visible within the UI) to graph space (position is relative to centre of graph).
    pub fn screen_to_graph(&self, graph_rect: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
        self.transform.inverse() * pos
        // let transform = TSTransform::from_translation(ui.min_rect().left_top().to_vec2())
        //     * view.camera.transform;
        // screen_to_graph(self.pos(), graph_rect, pos)
    }

    // FIXME: Remove this - just added temporarily to minimize breakage while
    // switching camera to transform.
    pub fn pos(&self) -> egui::Pos2 {
        self.transform.inverse().translation.to_pos2()
    }
}

impl NodeSockets {
    /// The screen position and normal of the input at the given index.
    ///
    /// Returns `None` if there is no input at the given index.
    pub fn input(&self, ix: usize) -> Option<(egui::Pos2, egui::Vec2)> {
        if self.input.count <= ix {
            return None;
        }
        let pos = self.input.layout.start + self.input.layout.step * ix as f32;
        let norm = match self.flow {
            egui::Direction::LeftToRight => egui::Vec2::new(-1.0, 0.0),
            egui::Direction::RightToLeft => egui::Vec2::new(1.0, 0.0),
            egui::Direction::TopDown => egui::Vec2::new(0.0, -1.0),
            egui::Direction::BottomUp => egui::Vec2::new(0.0, 1.0),
        };
        Some((pos, norm))
    }

    /// The screen position and normal of the input at the given index.
    ///
    /// Returns `None` if there is no output at the given index.
    pub fn output(&self, ix: usize) -> Option<(egui::Pos2, egui::Vec2)> {
        if self.output.count <= ix {
            return None;
        }
        let pos = self.output.layout.start + self.output.layout.step * ix as f32;
        let norm = match self.flow {
            egui::Direction::LeftToRight => egui::Vec2::new(1.0, 0.0),
            egui::Direction::RightToLeft => egui::Vec2::new(-1.0, 0.0),
            egui::Direction::TopDown => egui::Vec2::new(0.0, 1.0),
            egui::Direction::BottomUp => egui::Vec2::new(0.0, -1.0),
        };
        Some((pos, norm))
    }

    /// Produces an iterator yielding the position and normal for each input.
    pub fn inputs<'a>(&'a self) -> impl 'a + Iterator<Item = (egui::Pos2, egui::Vec2)> {
        (0..self.input.count).filter_map(move |ix| self.input(ix))
    }

    /// Produces an iterator yielding the position and normal for each output.
    pub fn outputs<'a>(&'a self) -> impl 'a + Iterator<Item = (egui::Pos2, egui::Vec2)> {
        (0..self.output.count).filter_map(move |ix| self.output(ix))
    }
}

impl<'ui> Show<'ui> {
    /// Instantiate the nodes of the graph.
    pub fn nodes(mut self, content: impl FnOnce(&mut NodesCtx, &mut egui::Ui)) -> Self {
        {
            let Self {
                graph_id,
                graph_rect,
                graph_layer,
                graph_bg_layer,
                ref mut visited,
                ref mut ui,
                ..
            } = self;
            let mut ctx = NodesCtx {
                graph_id,
                graph_rect,
                graph_layer,
                graph_bg_layer,
                visited,
                // selection_rect,
                // select,
                // socket_press_released,
            };
            content(&mut ctx, ui);
        }
        self
    }

    /// Instantiate the edges of the graph.
    pub fn edges(mut self, content: impl FnOnce(&mut EdgesCtx, &mut egui::Ui)) -> Self {
        {
            let Self {
                graph_id,
                graph_rect,
                graph_bg_layer,
                ref mut ui,
                // visible_rect,
                ..
            } = self;
            let mut ctx = EdgesCtx {
                graph_id,
                graph_rect,
                graph_bg_layer,
                // visible_rect,
            };
            content(&mut ctx, ui);
        }
        self
    }

    /// If a node didn't appear this update, it's likely because the user has removed the node from
    /// their graph, so we should stop tracking it.
    fn prune_unused_nodes(&mut self) {
        let Self {
            graph_id,
            ref visited,
            ref mut ui,
            ..
        } = *self;
        let gmem_arc = memory(ui, graph_id);
        let mut gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
        gmem.node_sizes.retain(|k, _| visited.contains(k));
        gmem.selection.nodes.retain(|k| visited.contains(k));
        if let Some(socket) = gmem.closest_socket.as_ref() {
            if !visited.contains(&socket.node) {
                gmem.closest_socket = None;
            }
        }
        if let Some(pressed) = gmem.pressed.as_ref() {
            match pressed.action {
                PressAction::DragNodes {
                    node: Some(PressedNode { id: n, .. }),
                }
                | PressAction::Socket(node::Socket { node: n, .. })
                    if !visited.contains(&n) =>
                {
                    gmem.pressed = None
                }
                _ => (),
            }
        }
    }
}

impl EdgesCtx {
    /// Retrieves the position and normal of the specified input for the given node.
    ///
    /// Returns `None` if either the `node` or `input` do not exist.
    pub fn input(
        &self,
        ui: &egui::Ui,
        node: egui::Id,
        input: usize,
    ) -> Option<(egui::Pos2, egui::Vec2)> {
        let gmem_arc = crate::memory(ui, self.graph_id);
        let gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
        gmem.sockets
            .get(&node)
            .and_then(|sockets| sockets.input(input))
    }

    /// Retrieves the position and normal of the specified output for the given node.
    ///
    /// Returns `None` if either the `node` or `output` do not exist.
    pub fn output(
        &self,
        ui: &egui::Ui,
        node: egui::Id,
        output: usize,
    ) -> Option<(egui::Pos2, egui::Vec2)> {
        let gmem_arc = memory(ui, self.graph_id);
        let gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
        gmem.sockets
            .get(&node)
            .and_then(|sockets| sockets.output(output))
    }

    /// If the user is in the progress of creating an edge, this returns the relevant info.
    pub fn in_progress(&self, ui: &egui::Ui) -> Option<EdgeInProgress> {
        let gmem_arc = memory(ui, self.graph_id);
        let gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
        let pressed = gmem.pressed.as_ref()?;
        let start = match pressed.action {
            PressAction::Socket(socket) => {
                let sockets = gmem.sockets.get(&socket.node)?;
                let (pos, normal) = match socket.kind {
                    node::SocketKind::Input => sockets.input(socket.index)?,
                    node::SocketKind::Output => sockets.output(socket.index)?,
                };
                node::PositionedSocket {
                    socket,
                    pos,
                    normal,
                }
            }
            _ => return None,
        };
        let (end_pos, end_socket) = match gmem.closest_socket {
            Some(socket) if socket.kind != start.socket.kind => {
                let sockets = gmem.sockets.get(&socket.node)?;
                let (pos, normal) = match socket.kind {
                    node::SocketKind::Input => sockets.input(socket.index)?,
                    node::SocketKind::Output => sockets.output(socket.index)?,
                };
                (pos, Some((socket.kind, normal)))
            }
            _ => {
                // FIXME
                let pos = egui::Pos2::new(0.0, 0.0);
                // let cam_pos = self.visible_rect.center();
                // let pos = graph_to_screen(cam_pos, self.graph_rect, pressed.current_pos);
                (pos, None)
            }
        };
        Some(EdgeInProgress {
            start,
            end_pos,
            end_socket,
        })
    }

    /// The full rect occuppied by the graph widget.
    pub fn graph_rect(&self) -> egui::Rect {
        self.graph_rect
    }

    /// The layer of the background response returned from the dot grid.
    ///
    /// Edges should be painted on this layer to inherit the correct transform.
    pub fn graph_bg_layer(&self) -> egui::LayerId {
        self.graph_bg_layer
    }
}

pub struct EdgeInProgress {
    /// The socket at the start end of the edge.
    pub start: node::PositionedSocket,
    /// The end position of the edge in progress.
    ///
    /// If there is no socket within the interaction radius, this will be the pointer position.
    /// Otherwise, this will be the position of the closest socket who's `SocketKind` is opposite
    /// to `start.kind`.
    pub end_pos: egui::Pos2,
    /// The closest socket who's `SocketKind` is opposite to `start.kind`.
    ///
    /// This is `None` in the case that there are no sockets within the interaction radius.
    pub end_socket: Option<(node::SocketKind, egui::Vec2)>,
}

impl EdgeInProgress {
    pub fn bezier_cubic(&self) -> bezier::Cubic {
        let start = (self.start.pos, self.start.normal);
        let end_normal = self
            .end_socket
            .as_ref()
            .map(|&(_, n)| n)
            .unwrap_or(-self.start.normal);
        let end = (self.end_pos, end_normal);
        bezier::Cubic::from_edge_points(start, end)
    }
}

impl<'ui> Drop for Show<'ui> {
    fn drop(&mut self) {
        self.prune_unused_nodes();
    }
}

/// Combines the given id src with the `TypeId` of the `Graph` to produce a unique `egui::Id`.
pub fn id(id_src: impl Hash) -> egui::Id {
    egui::Id::new((std::any::TypeId::of::<Graph>(), id_src))
}

/// Short-hand for retrieving access to the graph's temporary memory from the `Ui`.
fn memory(ui: &egui::Ui, graph_id: egui::Id) -> Arc<Mutex<GraphTempMemory>> {
    ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<Arc<Mutex<GraphTempMemory>>>(graph_id)
            .clone()
    })
}

fn graph_to_screen(cam_pos: egui::Pos2, graph_rect: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
    let half_rect_size = graph_rect.size() * 0.5;
    let v = pos - cam_pos.to_vec2() + half_rect_size;
    egui::Pos2::new(v.x, v.y)
}

fn screen_to_graph(cam_pos: egui::Pos2, graph_rect: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
    let half_rect_size = graph_rect.size() * 0.5;
    let v = pos - half_rect_size + cam_pos.to_vec2();
    egui::Pos2::new(v.x, v.y)
}
