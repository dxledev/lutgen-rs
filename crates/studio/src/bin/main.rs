#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
#[derive(bpaf::Bpaf)]
#[bpaf(options, version)]
struct Cli {
    /// Verbosity level, repeat for more (ie, -vvv for maximum logging)
    #[bpaf(
        short, req_flag(()), count,
        map(|l| {
            use log::LevelFilter::*;
            [Warn, Info, Debug, Trace][l.clamp(0, 3)]
        })
    )]
    verbosity: log::LevelFilter,
    /// Image to open on startup
    #[bpaf(positional, guard(|p| p.exists(), "Image path not found"), optional)]
    input: Option<std::path::PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
fn configure_unix_backend(native_options: &mut eframe::NativeOptions) {
    #[cfg(target_os = "linux")]
    {
        use winit::platform::wayland::EventLoopBuilderExtWayland as _;
        use winit::platform::x11::EventLoopBuilderExtX11 as _;

        let backend = std::env::var("LUTGEN_STUDIO_UNIX_BACKEND").ok();
        let on_hyprland = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();
        let has_x11 = std::env::var_os("DISPLAY").is_some();

        let prefer_x11 = match backend.as_deref() {
            Some("x11") => true,
            Some("wayland") => false,
            _ => on_hyprland && has_x11,
        };

        if prefer_x11 {
            native_options.event_loop_builder = Some(Box::new(|builder| {
                builder.with_x11();
            }));
            log::info!(
                "Using X11 backend for lutgen-studio{}",
                if backend.is_some() {
                    " (requested by LUTGEN_STUDIO_UNIX_BACKEND)"
                } else {
                    " (Hyprland workaround; set LUTGEN_STUDIO_UNIX_BACKEND=wayland to override)"
                }
            );
        } else if matches!(backend.as_deref(), Some("wayland")) {
            native_options.event_loop_builder = Some(Box::new(|builder| {
                builder.with_wayland();
            }));
            log::info!(
                "Using Wayland backend for lutgen-studio (requested by LUTGEN_STUDIO_UNIX_BACKEND)"
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() -> eframe::Result {
    let args = cli().run();

    env_logger::builder()
        .filter_level(args.verbosity)
        .parse_env("RUST_LOG")
        .init();

    let mut native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lutgen Studio")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../assets/lutgen.png"))
                    .expect("Failed to load icon"),
            ),
        persist_window: true,
        centered: true,
        ..Default::default()
    };
    configure_unix_backend(&mut native_options);

    eframe::run_native(
        "lutgen-studio",
        native_options,
        Box::new(|cc| Ok(Box::new(lutgen_studio::App::new(cc, args.input)))),
    )
}

#[cfg(target_arch = "wasm32")]
pub fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(lutgen_studio::App::new(cc, None)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                },
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                },
            }
        }
    });
}
