mod ui;

use std::path::PathBuf;

use gpui::{
    px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use ui::app::{Icons, VeloIde};

fn main() {
    let icons_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("icons")
        .join("bearded");

    Application::new().run(move |cx: &mut App| {
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
            move |_window, cx| cx.new(|cx| VeloIde::new(Icons::from_dir(&icons_dir), cx)),
        )
        .expect("failed to open Velo window");
    });
}
