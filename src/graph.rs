use egui::emath::TSTransform;
use std::hash::Hash;

pub struct Graph {
    id: egui::Id,
    background: bool,
}

#[derive(Debug, Default)]
pub struct Camera {
    transform: TSTransform,
}

pub struct Show;

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

    pub fn show(self, camera: &mut Camera, ui: &mut egui::Ui) -> Show {
        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());

        // Allow dragging the background as well.
        if response.dragged_by(egui::PointerButton::Middle) {
            camera.transform.translation += response.drag_delta();
        }

        // Plot-like reset
        if response.double_clicked() {
            camera.transform = TSTransform::default();
        }

        let transform =
            TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) * camera.transform;

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Note: doesn't catch zooming / panning if a button in this PanZoom container is hovered.
            if response.hovered() {
                let pointer_in_layer = transform.inverse() * pointer;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // Zoom in on pointer:
                camera.transform = camera.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                camera.transform = TSTransform::from_translation(pan_delta) * camera.transform;
            }
        }

        // Paint the background rect.
        if self.background {
            let vis = ui.style().noninteractive();
            let stroke = egui::Stroke {
                width: 0.0,
                ..vis.bg_stroke
            };
            let fill = vis.bg_fill;
            ui.painter().rect(rect, 0.0, fill, stroke);
        }

        // Paint some subtle dots to check camera movement.
        let half_size = rect.size() * 0.5;
        let visible_rect =
            egui::Rect::from_center_size((-camera.transform.translation).to_pos2(), rect.size());
        let dot_step = ui.spacing().interact_size.y * camera.transform.scaling;
        let vis = ui.style().noninteractive();
        let x_dots =
            (visible_rect.min.x / dot_step) as i32..=(visible_rect.max.x / dot_step) as i32;
        let y_dots =
            (visible_rect.min.y / dot_step) as i32..=(visible_rect.max.y / dot_step) as i32;
        let x_start = half_size.x + camera.transform.translation.x;
        let y_start = half_size.y + camera.transform.translation.y;
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

        let id = egui::Area::new(id.with("subarea"))
            .default_pos(egui::Pos2::new(0.0, 0.0))
            // Need to cover up the pan_zoom demo window,
            // but may also cover over other windows.
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(transform.inverse() * rect);
                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(ui.ctx().style().visuals.window_stroke)
                    .fill(ui.style().visuals.panel_fill)
                    .show(ui, |ui| {
                        ui.style_mut().wrap = Some(false);

                        // Node widget.
                        ui.label(format!("{:?}", camera));
                        // callback(ui, self)
                    });
            })
            .response
            .layer_id;
        ui.ctx().set_transform_layer(id, transform);

        Show
    }
}

// fn dot_grid(

/// Combines the given id src with the `TypeId` of the `Graph` to produce a unique `egui::Id`.
pub fn id(id_src: impl Hash) -> egui::Id {
    egui::Id::new((std::any::TypeId::of::<Graph>(), id_src))
}
