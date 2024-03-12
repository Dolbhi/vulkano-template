use egui_winit_vulkano::egui::{self, Color32, Context, FontId, RichText};

pub fn pause_menu(ctx: &Context) {
    egui::Area::new("Pause Menu")
        .fixed_pos((500., 300.))
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_gray(50))
                .inner_margin(20.)
                .outer_margin(20.)
                .rounding(5.)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Paused")
                            .font(FontId::proportional(40.0))
                            .color(Color32::WHITE),
                    );
                });
            ui.label("KILL ME");
        });
}

pub fn test_area(ctx: &Context) {
    egui::Area::new("Pause Menu")
        .default_pos((500., 300.))
        .movable(true)
        .constrain(true)
        // .default_rect(window_rect)
        // .resizable(true)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::GREEN)
                .inner_margin(10.)
                .outer_margin(10.)
                .rounding(5.)
                .show(ui, |ui| {
                    ui.add_sized((100., 100.), egui::Label::new("SIZED BOI"));
                    ui.label("inside frame");
                });
            ui.label("KILL ME");
        });
}
