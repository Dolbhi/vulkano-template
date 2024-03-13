use egui_winit_vulkano::egui::{self, Color32, Context, FontId, RichText};

pub enum MenuOption {
    None,
    LoadLevel(i32),
    Quit,
}

pub fn main_menu(ctx: &Context) -> MenuOption {
    let mut result = MenuOption::None;
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
                        RichText::new("RUSTY RENDERER!!")
                            .font(FontId::proportional(40.0))
                            .color(Color32::WHITE),
                    );
                    if ui
                        .button(
                            RichText::new("Load Level 1")
                                .font(FontId::proportional(40.0))
                                .color(Color32::WHITE),
                        )
                        .clicked()
                    {
                        result = MenuOption::LoadLevel(1);
                    }
                    if ui
                        .button(
                            RichText::new("Load Level 2")
                                .font(FontId::proportional(40.0))
                                .color(Color32::WHITE),
                        )
                        .clicked()
                    {
                        result = MenuOption::LoadLevel(2);
                    }
                    if ui
                        .button(
                            RichText::new("Quit")
                                .font(FontId::proportional(40.0))
                                .color(Color32::WHITE),
                        )
                        .clicked()
                    {
                        result = MenuOption::Quit;
                    }
                });
        });

    result
}

pub fn pause_menu(ctx: &Context, quit_callback: impl FnOnce() -> ()) {
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
                    if ui
                        .button(
                            RichText::new("Quit")
                                .font(FontId::proportional(40.0))
                                .color(Color32::WHITE),
                        )
                        .clicked()
                    {
                        // ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        quit_callback();
                    }
                });
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
