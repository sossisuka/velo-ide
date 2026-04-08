mod ui;

use std::{borrow::Cow, fs, path::PathBuf};

use gpui::{
    px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use ui::app::{Icons, VeloIde};

fn main() {
    let file_icons_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("icons")
        .join("bearded");
    let fonts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("fonts");
    let activity_icons_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("icons"))
        .unwrap_or_else(|| PathBuf::from("icons"));

    Application::new().run(move |cx: &mut App| {
        let font_files = [
            "poppins_regular.ttf",
            "poppins_medium.ttf",
            "poppins_semibold.ttf",
            "montserrat_regular.ttf",
            "montserrat_medium.ttf",
            "montserrat_semibold.ttf",
        ];
        let loaded_fonts = font_files
            .iter()
            .filter_map(|name| fs::read(fonts_dir.join(name)).ok().map(Cow::Owned))
            .collect::<Vec<_>>();
        if !loaded_fonts.is_empty() {
            let _ = cx.text_system().add_fonts(loaded_fonts);
        }

        let window_bounds = Bounds::centered(None, size(px(1360.0), px(860.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Velo IDE".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |_window, cx| {
                cx.new(|cx| VeloIde::new(Icons::from_dirs(&file_icons_dir, &activity_icons_dir), cx))
            },
        )
        .expect("failed to open Velo window");
    });
}
