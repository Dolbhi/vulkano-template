use egui_winit_vulkano::egui::{self, Color32, Context, FontId, Layout, RichText, Style};

pub enum MenuOption {
    None,
    LoadLevel(i32),
    QuitLevel,
    Quit,
}

pub fn set_style(style: &mut Style) {
    style.spacing.item_spacing = (20.0, 20.0).into();
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(40.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(30.0));

    // style.visuals.text_color();
}

pub fn main_menu(ctx: &Context, option_selected: &mut MenuOption) {
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
                    ui.allocate_ui_with_layout(
                        (300.0, 500.0).into(),
                        Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.heading(RichText::new("RUSTY RENDERER!!").color(Color32::WHITE));
                            if ui
                                .button(RichText::new("Load Level 1").color(Color32::WHITE))
                                .clicked()
                            {
                                *option_selected = MenuOption::LoadLevel(0);
                            }
                            if ui
                                .button(RichText::new("Load Level 2").color(Color32::WHITE))
                                .clicked()
                            {
                                *option_selected = MenuOption::LoadLevel(1);
                            }
                            if ui
                                .button(RichText::new("Quit").color(Color32::WHITE))
                                .clicked()
                            {
                                *option_selected = MenuOption::Quit;
                            }
                        },
                    )
                })
        });
}

pub fn pause_menu(ctx: &Context, option_selected: &mut MenuOption) {
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
                    ui.allocate_ui_with_layout(
                        (300.0, 500.0).into(),
                        Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.heading(RichText::new("Paused").color(Color32::WHITE));
                            if ui
                                .button(RichText::new("Quit Level").color(Color32::WHITE))
                                .clicked()
                            {
                                *option_selected = MenuOption::QuitLevel;
                            }
                            if ui
                                .button(RichText::new("Quit To Desktop").color(Color32::WHITE))
                                .clicked()
                            {
                                *option_selected = MenuOption::Quit;
                            }
                        },
                    );
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
