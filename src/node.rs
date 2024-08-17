use crate::{NodeSockets, NodesCtx, SocketLayout};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

/// The default node widget.
///
/// A `Node` is a thin wrapper around a `Window` and allows for instantiating arbitrary widgets
/// internally.
pub struct Node {
    id: egui::Id,
    frame: Option<egui::Frame>,
    inputs: usize,
    outputs: usize,
    flow: egui::Direction,
    socket_radius: f32,
    socket_color: Option<egui::Color32>,
    max_width: Option<f32>,
    animation_time: f32,
}

/// Describes either an input or output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SocketKind {
    Input,
    Output,
}

/// Uniquely identifies a socket.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Socket {
    /// The node that owns this socket.
    pub node: egui::Id,
    /// Whether the socket is an input or output.
    pub kind: SocketKind,
    /// The index of the socket of this kind.
    pub index: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PositionedSocket {
    pub socket: Socket,
    /// Screen-space position of the socket.
    pub pos: egui::Pos2,
    /// The normal of the edge along which this socket resides.
    pub normal: egui::Vec2,
}

/// An extension around the `egui::Response` that indicates node selection.
pub struct NodeResponse {
    response: egui::Response,
    selection_changed: bool,
    selected: bool,
    removed: bool,
    /// Some event occurred related to the creation of an edge.
    edge_event: Option<EdgeEvent>,
}

/// Events related to the creation of an edge to or from a node.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeEvent {
    /// Occurs if a socket was just pressed to start creating an edge.
    Started { kind: SocketKind, index: usize },
    /// Occurs when the primary mouse button was released creating an edge ending at the specified
    /// socket.
    Ended { kind: SocketKind, index: usize },
    /// If there was an edge in progress starting from this node, this indicates that it was
    /// cancelled.
    Cancelled,
}

impl Node {
    /// Begin instantiating a new node widget.
    pub fn new(id_src: impl Hash) -> Self {
        Self::from_id(egui::Id::new(id_src))
    }

    // Construct the node directly from its `egui::Id`.
    // TODO: Should this be exposed?
    fn from_id(id: egui::Id) -> Self {
        Self {
            id,
            frame: None,
            max_width: None,
            socket_color: None,
            inputs: 0,
            outputs: 0,
            flow: egui::Direction::LeftToRight,
            socket_radius: 3.0,
            animation_time: 0.1,
        }
    }

    /// Specify the `Frame` used for the `Node`'s window.
    ///
    /// The default is retrieved via `egui_graph::node::default_frame(ui.style())`.
    pub fn frame(mut self, frame: egui::Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Optionally specify the max width of the `Node`'s window.
    ///
    /// By default, `ui.spacing().text_edit_width` is used.
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }

    pub fn inputs(mut self, n: usize) -> Self {
        self.inputs = n;
        self
    }

    pub fn outputs(mut self, n: usize) -> Self {
        self.outputs = n;
        self
    }

    /// The direction of dataflow through the graph.
    ///
    /// This determines which edges the inputs and outputs are distributed across.
    ///
    /// E.g. `LeftToRight` will have inputs on the left and outputs on the right.
    ///
    /// On vertical edges, inputs/outputs start at the top and end at the bottom.
    ///
    /// On horizontal edges, inputs/outputs start at the left and end at the right.
    ///
    /// Default direction is `LeftToRight`.
    pub fn flow(mut self, flow: egui::Direction) -> Self {
        self.flow = flow;
        self
    }

    /// The color of the input and output sockets.
    pub fn socket_color(mut self, color: egui::Color32) -> Self {
        self.socket_color = Some(color);
        self
    }

    /// The radius of the input and output sockets.
    pub fn socket_radius(mut self, radius: f32) -> Self {
        self.socket_radius = radius;
        self
    }

    /// The time taken (seconds) for the node to interpolate toward a new location.
    ///
    /// Default: `0.1`.
    pub fn animation_time(mut self, time: f32) -> Self {
        self.animation_time = time;
        self
    }

    /// Present the `Node`'s `Window` and add the given contents.
    pub fn show(
        self,
        view: &mut crate::View,
        ctx: &mut NodesCtx,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> NodeResponse {
        self.show_impl(view, ctx, ui, Box::new(content) as Box<_>)
    }

    fn show_impl<'a>(
        self,
        view: &mut crate::View,
        ctx: &mut NodesCtx,
        ui: &mut egui::Ui,
        content: Box<dyn FnOnce(&mut egui::Ui) + 'a>,
    ) -> NodeResponse {
        let crate::View {
            ref mut layout,
            ref camera,
        } = *view;

        //-----------------------------------------

        // Indicate that we've visited this node this update.
        ctx.visited.insert(self.id);

        // Determine the current position of the window relative to the graph origin.
        let target_pos_graph = layout.entry(self.id).or_insert_with(|| {
            // If the mouse is over the graph, add the node under the mouse.
            // Otherwise, add the node to the top-left.
            let mut pos = camera.pos() - ctx.graph_rect.center() + ui.spacing().item_spacing;
            if ui.rect_contains_pointer(ctx.graph_rect) {
                ui.input(|i| i.pointer.hover_pos()).map(|ptr| {
                    pos = ptr - ctx.graph_rect.center() + camera.pos().to_vec2()
                        - ui.spacing().interact_size * 0.5;
                });
            }
            egui::Pos2::new(pos.x, pos.y)
        });

        // Interpolate toward the desired position over time for auto-layout.
        let pos_graph = {
            let ctx = ui.ctx();
            let idx = self.id.with("x");
            let idy = self.id.with("y");
            let x = ctx.animate_value_with_time(idx, target_pos_graph.x, self.animation_time);
            let y = ctx.animate_value_with_time(idy, target_pos_graph.y, self.animation_time);
            egui::Pos2::new(x, y)
        };

        // Translate the graph position to a position within the UI.
        let pos_screen = camera.graph_to_screen(ctx.graph_rect, pos_graph);

        // The window should always be at least the interaction size.
        let min_item_spacing = ui.spacing().item_spacing.x.min(ui.spacing().item_spacing.y);
        let min_interact_len = ui
            .spacing()
            .interact_size
            .x
            .min(ui.spacing().interact_size.y);
        let mut min_size = egui::Vec2::splat(min_interact_len);
        // However, it should also always be at least large enough to comfortably show all
        // inlets/outlets.
        let max_sockets = std::cmp::max(self.inputs, self.outputs);
        let min_socket_gap = min_interact_len + min_item_spacing;
        let win_corner_radius = ui.visuals().window_rounding.ne;
        let socket_padding = win_corner_radius + min_interact_len * 0.5;
        let min_len = (max_sockets.max(1) - 1) as f32 * min_socket_gap + socket_padding * 2.0;
        if max_sockets > 1 {
            match self.flow {
                egui::Direction::LeftToRight | egui::Direction::RightToLeft => {
                    min_size.y = min_size.y.max(min_len);
                }
                egui::Direction::TopDown | egui::Direction::BottomUp => {
                    min_size.x = min_size.x.max(min_len);
                }
            }
        }
        // Retrieve the frame for the window.
        let mut frame = self.frame.unwrap_or_else(|| default_frame(ui.style()));

        let max_w = self.max_width.unwrap_or(ui.spacing().text_edit_width);
        let max_size = egui::Vec2::new(max_w, ctx.graph_rect.height());

        // Track changes in selection for the node response.
        let mut selection_changed = false;

        // Determine whether or not this node is within the selection rect.
        // NOTE: We use the size from last frame as we don't know the size until the user's content
        // is added... Is there a better way to handle this?
        let (mut selected, in_selection_rect) = {
            let gmem_arc = crate::memory(ui, ctx.graph_id);
            let mut gmem = gmem_arc.lock().expect("failed to lock graph temp memory");

            // FIXME
            let in_selection_rect = false;
            // let in_selection_rect = match ctx.selection_rect {
            //     None => false,
            //     Some(sel_rect) => {
            //         let size = gmem
            //             .node_sizes
            //             .get(&self.id)
            //             .cloned()
            //             .unwrap_or(egui::Vec2::ZERO);
            //         let rect = egui::Rect::from_min_size(pos_screen, size);
            //         sel_rect.intersects(rect)
            //     }
            // };

            // // Update the selection if the primary mouse button was just released.
            // if ctx.select {
            //     if in_selection_rect {
            //         if gmem.selection.nodes.insert(self.id) {
            //             selection_changed = true;
            //         }
            //     } else if !ui.input(|i| i.modifiers.ctrl) {
            //         if gmem.selection.nodes.remove(&self.id) {
            //             selection_changed = true;
            //         }
            //     }
            // }

            let selected = gmem.selection.nodes.contains(&self.id);

            (selected, in_selection_rect)
        };
        // Style the frame based on interaction.
        if selected {
            frame.stroke = ui.visuals().selection.stroke;
        } else if in_selection_rect {
            let color = ui.visuals().weak_text_color();
            frame.shadow.color = color;
            frame.stroke = ui.visuals().selection.stroke;
            frame.stroke.color = color;
        }

        // -----------------------------------------------------

        // The area for the the node on the graph.
        let window_layer = ui.layer_id();
        let mut response = egui::Area::new(self.id) // FIXME: Combine ID with graph ID.
            .default_pos(pos_screen)
            // .fixed_pos(pos_screen)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                // Clip the node to the graph rect.
                ui.set_clip_rect(camera.transform.inverse() * ctx.graph_rect);

                // The frame for the widget.
                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(ui.ctx().style().visuals.window_stroke)
                    .fill(ui.style().visuals.panel_fill)
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                        // // Ensure the ui is at least large enough to provide space for inputs/outputs.
                        // let gap = egui::Vec2::splat(win_corner_radius * 2.0);
                        // let min_size = min_size - gap;
                        // ui.set_min_size(min_size);

                        // Node widget.
                        content(ui);
                    });
            })
            .response;
        ui.ctx()
            .set_transform_layer(response.layer_id, camera.transform);
        ui.ctx().set_sublayer(window_layer, response.layer_id);

        // let mut response = egui::Window::new("")
        //     .id(self.id)
        //     .frame(frame)
        //     .resizable(false)
        //     // TODO: These `min_*` and `default_size` methods seem to be totally ignored? Should
        //     // fix this upstream, but for now we just set min size on the window's `Ui` instead.
        //     .min_width(min_size.x)
        //     .min_height(min_size.y)
        //     .default_size(min_size)
        //     // TODO: Only `max_size` seems to be considered here - `min_size` seems to be ignored.
        //     .resize(|resize| resize.max_size(max_size).min_size(min_size))
        //     .fixed_pos(pos_screen)
        //     .collapsible(false)
        //     .title_bar(false)
        //     .auto_sized()
        //     .constrain_to(egui::Rect::EVERYTHING)
        //     .show(ui.ctx(), move |ui| {
        //         // Ensure the ui is at least large enough to provide space for inputs/outputs.
        //         let gap = egui::Vec2::splat(win_corner_radius * 2.0);
        //         let min_size = min_size - gap;
        //         ui.set_min_size(min_size);
        //         // Set the user's content.
        //         content(ui);
        //     })
        //     .expect("node windows are always open")
        //     .response;

        // Update the stored data for this node and check for edge events.
        let mut edge_event = None;
        {
            let gmem_arc = crate::memory(ui, ctx.graph_id);
            let mut gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
            gmem.node_sizes.insert(self.id, response.rect.size());

            let ctrl_down = ui.input(|i| i.modifiers.ctrl);

            // If the window is pressed, select the node.
            let pointer = &ui.input(|i| i.pointer.clone());
            if response.is_pointer_button_down_on() && primary_pressed(pointer) {
                // If ctrl is down, check for deselection.
                if ctrl_down && gmem.selection.nodes.contains(&self.id) {
                    selection_changed = gmem.selection.nodes.remove(&self.id);
                    selected = false;
                } else {
                    selection_changed = gmem.selection.nodes.insert(self.id);
                    selected = true;
                    if let Some(ref mut pressed) = gmem.pressed {
                        if let crate::PressAction::DragNodes { ref mut node } = pressed.action {
                            *node = Some(crate::PressedNode {
                                id: self.id,
                                position_at_origin: pos_graph,
                            });
                        }
                    }
                }

            // If the primary button was pressed, check for edge events.
            } else if !response.is_pointer_button_down_on() && primary_pressed(pointer) {
                // If this node's socket was pressed, create a start event.
                if let Some(ref pressed) = gmem.pressed {
                    if let crate::PressAction::Socket(socket) = pressed.action {
                        if self.id == socket.node {
                            let kind = socket.kind;
                            let index = socket.index;
                            edge_event = Some(EdgeEvent::Started { kind, index });
                        }
                    }
                }

                // Also check for deselection.
                if !ctrl_down
                    && !gmem
                        .pressed
                        .as_ref()
                        .map(|p| p.over_selection_at_origin)
                        .unwrap_or(false)
                {
                    selection_changed = gmem.selection.nodes.remove(&self.id);
                    selected = false;
                }
            }
            // FIXME:
            // Check for edge creation / cancellation events.
            // } else if let Some(r) = ctx.socket_press_released {
            //     if let Some(c) = gmem.closest_socket {
            //         if r.kind == c.kind && self.id == r.node {
            //             edge_event = Some(EdgeEvent::Cancelled);
            //         } else if self.id == c.node && primary_released(&ui.input(|i| i.clone())) {
            //             let kind = c.kind;
            //             let index = c.index;
            //             edge_event = Some(EdgeEvent::Ended { kind, index });
            //         }
            //     } else if edge_event.is_none() {
            //         if self.id == r.node {
            //             edge_event = Some(EdgeEvent::Cancelled);
            //         }
            //     }
            // }
        }

        if response.rect.min != pos_screen {
            *target_pos_graph += response.rect.min - pos_screen;
            response.mark_changed();
        }

        // The inlets/outlets.
        if self.inputs > 0 || self.outputs > 0 {
            // Store the layout of the sockets for edge instantiation.
            let sockets = node_sockets(
                self.inputs,
                self.outputs,
                socket_padding,
                response.rect,
                self.flow,
            );
            let gmem_arc = crate::memory(ui, ctx.graph_id);
            let mut gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
            gmem.sockets.insert(self.id, sockets.clone());

            // Check whether or not this node has a pressed socket.
            let pressed_socket = gmem
                .pressed
                .as_ref()
                .and_then(|pressed| match pressed.action {
                    crate::PressAction::Socket(socket) if socket.node == self.id => {
                        Some((socket.kind, socket.index))
                    }
                    _ => None,
                });

            // Check if the mouse is closest to one of this node's sockets.
            let closest_socket = match gmem.closest_socket {
                Some(closest) if closest.node == self.id => {
                    // If there is a pressed socket, only highlight if this socket is of the
                    // opposite kind.
                    match gmem.pressed.as_ref().map(|p| &p.action) {
                        Some(crate::PressAction::Socket(socket)) if closest.kind == socket.kind => {
                            None
                        }
                        _ => Some((closest.kind, closest.index)),
                    }
                }
                _ => None,
            };

            // Whether or not to paint the highlight.
            let paint_highlight = |kind, ix| {
                if let Some((k, i)) = pressed_socket {
                    if k == kind && i == ix {
                        return true;
                    }
                }
                if let Some((k, i)) = closest_socket {
                    if k == kind && i == ix {
                        return true;
                    }
                }
                false
            };

            let color = self.socket_color.unwrap_or(ui.visuals().text_color());
            let hl_size = (self.socket_radius + 4.0).max(4.0);
            let mut in_pos = sockets.input.layout.start;
            let mut out_pos = sockets.output.layout.start;
            for ix in 0..self.inputs {
                if paint_highlight(SocketKind::Input, ix) {
                    ui.painter()
                        .circle_filled(in_pos, hl_size, color.linear_multiply(0.25));
                }
                ui.painter()
                    .circle_filled(in_pos, self.socket_radius, color);
                in_pos += sockets.input.layout.step;
            }
            for ix in 0..self.outputs {
                if paint_highlight(SocketKind::Output, ix) {
                    ui.painter()
                        .circle_filled(out_pos, hl_size, color.linear_multiply(0.25));
                }
                ui.painter()
                    .circle_filled(out_pos, self.socket_radius, color);
                out_pos += sockets.output.layout.step;
            }
        }

        // If the delete or backspace key was pressed and the node is selected, remove it.
        let removed = if selected
            && ui.input(|i| i.key_pressed(egui::Key::Delete) | i.key_pressed(egui::Key::Backspace))
        {
            // Remove ourselves from the selection.
            let gmem_arc = crate::memory(ui, ctx.graph_id);
            let mut gmem = gmem_arc.lock().expect("failed to lock graph temp memory");
            selection_changed = gmem.selection.nodes.remove(&self.id);
            selected = false;
            true
        } else {
            false
        };

        if selection_changed || removed || edge_event.is_some() {
            response.mark_changed();
        }

        NodeResponse {
            response,
            selection_changed,
            selected,
            removed,
            edge_event,
        }
    }
}

impl NodeResponse {
    /// Whether or not the selection changed and, if so, whether or not the node is now selected.
    pub fn selection(&self) -> Option<bool> {
        if self.selection_changed {
            Some(self.selected)
        } else {
            None
        }
    }

    /// Whether or not the node is selected.
    pub fn selected(&self) -> bool {
        self.selected
    }

    /// Whether or not the node was removed from the graph.
    ///
    /// This occurs if the `Delete` key is pressed while the node is selected.
    pub fn removed(&self) -> bool {
        self.removed
    }

    /// Consume the node response and produce the inner `egui::Response`.
    pub fn into_inner(self) -> egui::Response {
        self.response
    }

    /// Whether or not any events occurred related to the creation of an edge.
    pub fn edge_event(&self) -> Option<EdgeEvent> {
        self.edge_event
    }
}

impl Deref for NodeResponse {
    type Target = egui::Response;
    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl DerefMut for NodeResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

/// The default frame styling used for the `Node`'s `Window`.
pub fn default_frame(style: &egui::Style) -> egui::Frame {
    let mut frame = egui::Frame::window(style);
    frame.shadow.offset = egui::Vec2::ZERO;
    frame.stroke.width = 0.0;
    frame
}

/// The layout of the input and output socket layout.
fn input_output_layout(
    inputs: usize,
    outputs: usize,
    socket_padding: f32,
    rect: egui::Rect,
    flow: egui::Direction,
) -> (SocketLayout, SocketLayout) {
    let in_gap = |len: f32| {
        if inputs > 1 {
            len / (inputs - 1) as f32
        } else {
            0.0
        }
    };
    let out_gap = |len: f32| {
        if outputs > 1 {
            len / (outputs - 1) as f32
        } else {
            0.0
        }
    };
    let y_len = || rect.height() - socket_padding * 2.0;
    let x_len = || rect.width() - socket_padding * 2.0;
    match flow {
        egui::Direction::LeftToRight => {
            let len = y_len();
            let start = rect.min.y + socket_padding;
            let input = SocketLayout {
                start: egui::Pos2::new(rect.min.x, start),
                step: egui::Vec2::new(0.0, in_gap(len)),
            };
            let output = SocketLayout {
                start: egui::Pos2::new(rect.max.x, start),
                step: egui::Vec2::new(0.0, out_gap(len)),
            };
            (input, output)
        }
        egui::Direction::RightToLeft => {
            let len = y_len();
            let start = rect.min.y + socket_padding;
            let input = SocketLayout {
                start: egui::Pos2::new(rect.max.x, start),
                step: egui::Vec2::new(0.0, in_gap(len)),
            };
            let output = SocketLayout {
                start: egui::Pos2::new(rect.min.x, start),
                step: egui::Vec2::new(0.0, out_gap(len)),
            };
            (input, output)
        }
        egui::Direction::TopDown => {
            let len = x_len();
            let start = rect.min.x + socket_padding;
            let input = SocketLayout {
                start: egui::Pos2::new(start, rect.min.y),
                step: egui::Vec2::new(in_gap(len), 0.0),
            };
            let output = SocketLayout {
                start: egui::Pos2::new(start, rect.max.y),
                step: egui::Vec2::new(out_gap(len), 0.0),
            };
            (input, output)
        }
        egui::Direction::BottomUp => {
            let len = x_len();
            let start = rect.min.x + socket_padding;
            let input = SocketLayout {
                start: egui::Pos2::new(start, rect.max.y),
                step: egui::Vec2::new(in_gap(len), 0.0),
            };
            let output = SocketLayout {
                start: egui::Pos2::new(start, rect.min.y),
                step: egui::Vec2::new(out_gap(len), 0.0),
            };
            (input, output)
        }
    }
}

/// Construct the `NodeSockets` describing the default layout of inputs and
/// outputs.
fn node_sockets(
    inputs: usize,
    outputs: usize,
    socket_padding: f32,
    rect: egui::Rect,
    flow: egui::Direction,
) -> crate::NodeSockets {
    let (input_layout, output_layout) =
        input_output_layout(inputs, outputs, socket_padding, rect, flow);

    // Store the layout of the sockets for edge instantiation.
    NodeSockets {
        flow,
        input: crate::Sockets {
            count: inputs,
            layout: input_layout,
        },
        output: crate::Sockets {
            count: outputs,
            layout: output_layout,
        },
    }
}

fn only_primary_down(pointer: &egui::PointerState) -> bool {
    pointer.button_down(egui::PointerButton::Primary)
        && !pointer.button_down(egui::PointerButton::Middle)
        && !pointer.button_down(egui::PointerButton::Secondary)
}

fn primary_pressed(pointer: &egui::PointerState) -> bool {
    pointer.any_pressed() && only_primary_down(pointer)
}

fn primary_released(input: &egui::InputState) -> bool {
    input.pointer.any_released()
        && input.events.iter().any(|e| match e {
            egui::Event::PointerButton {
                button: egui::PointerButton::Primary,
                pressed: false,
                ..
            } => true,
            _ => false,
        })
}
