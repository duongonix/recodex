use tauri::{
    App,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

const OPEN_MAIN_ID: &str = "open-main";
const OPEN_CONTROLS_ID: &str = "open-controls";
const QUIT_ID: &str = "quit";

pub fn install(app: &App) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, OPEN_MAIN_ID, "Open RecoDex", true, None::<&str>)?;
    let open_controls = MenuItem::with_id(
        app,
        OPEN_CONTROLS_ID,
        "Show Recording Controls",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_main, &open_controls, &quit])?;

    TrayIconBuilder::new()
        .icon(tauri::include_image!("icons/32x32.png"))
        .menu(&menu)
        .tooltip("RecoDex")
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_MAIN_ID => crate::shortcut::show_main_window(app),
            OPEN_CONTROLS_ID => crate::shortcut::show_quick_access(app),
            QUIT_ID => request_quit(app),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn request_quit(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if crate::commands::recording_shutdown::stop_before_quit(&app)
            .await
            .is_ok()
        {
            app.exit(0);
        } else {
            crate::shortcut::show_main_window(&app);
        }
    });
}
