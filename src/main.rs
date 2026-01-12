mod helpers;
mod actions;
mod wireguard_config;

use gtk::prelude::*;
use gtk::gio;
use gtk::{Application, ApplicationWindow, Button};
use gtk::glib;
use std::env;
use std::rc::Rc;
use crate::actions::AppConfig;

fn main() -> glib::ExitCode {

    let app = Application::builder()
        .application_id("com.autoguard.autoguard")
        .build();

    app.connect_activate(|app| {
        helpers::load_css();

        let res_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/resources.gresource"));
        let resource = gio::Resource::from_data(&glib::Bytes::from(res_bytes))
            .expect("Failed to load GResource");
        gio::resources_register(&resource);

        // Load UI from GtkBuilder
        let builder = gtk::Builder::from_resource("/com/autoguard/autoguard/ui/main_window.ui");

        let window: ApplicationWindow = builder.object("main_window").unwrap();
        window.set_application(Some(app));

        let update_button: Button = builder.object("update_button").unwrap();
        let configure_button: Button = builder.object("configure_button").unwrap();
        let vbox: gtk::Box = builder.object("root").unwrap();

        let cfg: AppConfig = confy::load("autoguard", None).unwrap();

        let state = Rc::new(actions::AppState {
            window: window.clone(),
            vbox: vbox.clone(),
            button: update_button.clone(),
        });

        update_button.connect_clicked({
            let state = state.clone();
            move |_| actions::update_allowed_ips(&state)
        });

        configure_button.connect_clicked({
            let state = state.clone();
            move |_| actions::show_settings_dialog(&state)
        });

        window.present();
    });

    app.run()
}
