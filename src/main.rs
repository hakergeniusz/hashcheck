// Hashcheck - Check hashes for your files
// SPDX-License-Identifier: MIT

mod about;
mod checksum;
mod config;
mod feedback;
mod hash_row;
mod settings;
mod toggle_dialog;
mod utils;
mod window;

use adw::prelude::*;
use gtk::gio;
use gtk::glib;

fn main() -> glib::ExitCode {
    // Handle flags GApplication would reject ourselves.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("hashcheck {}", config::VERSION);
        return glib::ExitCode::SUCCESS;
    }

    let app = adw::Application::builder()
        .application_id(config::APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    settings::init();
    setup_actions(&app);
    setup_accels(&app);

    app.connect_startup(|_| utils::load_css());
    app.connect_activate(|app| window::open_window_with_file(app, None));
    app.connect_open(|app, files, _| open_files(app, files));

    // GApplication errors out on unknown flags, so hand it a cleaned-up argv.
    let clean_args: Vec<String> = std::iter::once("hashcheck".to_string())
        .chain(args.into_iter().skip(1).filter(|a| !a.starts_with('-')))
        .collect();
    let run_args: Vec<&str> = clean_args.iter().map(|s| s.as_str()).collect();
    app.run_with_args(&run_args)
}

fn setup_actions(app: &adw::Application) {
    let about = gio::SimpleAction::new("about", None);
    about.connect_activate(glib::clone!(#[weak] app, move |_, _| about::show_dialog(&app)));
    app.add_action(&about);

    let shortcuts = gio::SimpleAction::new("shortcuts", None);
    shortcuts.connect_activate(glib::clone!(#[weak] app, move |_, _| about::show_shortcuts(&app)));
    app.add_action(&shortcuts);

    let hashinfo = gio::SimpleAction::new("hashinfo", None);
    hashinfo.connect_activate(|_, _| {
        let _ = gio::AppInfo::launch_default_for_uri(config::ARTICLE, gio::AppLaunchContext::NONE);
    });
    app.add_action(&hashinfo);

    let new_window = gio::SimpleAction::new("new-window", None);
    new_window.connect_activate(glib::clone!(#[weak] app, move |_, _| app.activate()));
    app.add_action(&new_window);

    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate(glib::clone!(#[weak] app, move |_, _| app.quit()));
    app.add_action(&quit);
}

fn setup_accels(app: &adw::Application) {
    app.set_accels_for_action("win.open-file", &["<Ctrl>O"]);
    app.set_accels_for_action("window.close", &["<Ctrl>W"]);
    app.set_accels_for_action("app.new-window", &["<Ctrl>N"]);
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
    app.set_accels_for_action("app.about", &["F1"]);
}

fn open_files(app: &adw::Application, files: &[gio::File]) {
    if files.is_empty() {
        window::open_window_with_file(app, None);
        return;
    }
    // Each file gets its own window, invalid paths are skipped.
    for file in files {
        if file.path().is_some_and(|p| p.is_file()) {
            window::open_window_with_file(app, Some(file.clone()));
        }
    }
}
