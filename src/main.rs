mod backends;
mod settings;
mod ui;
mod utils;

use gtk4::prelude::*;
use gtk4::Application;
use std::sync::{Arc, Mutex};
use ui::window::AppWindow;

const APP_ID: &str = "org.acreetionos.appstore";

#[async_std::main]
async fn main() -> glib::ExitCode {
    let settings = Arc::new(Mutex::new(settings::Settings::load()));

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        AppWindow::new(app, settings.clone());
    });

    app.run()
}
