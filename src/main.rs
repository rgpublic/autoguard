mod aux;
mod actions;
mod wireguard_config;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, HeaderBar, Box as GtkBox, Button, Orientation, Label};
use gtk::glib;
use std::rc::Rc;

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.autoguard.autoguard")
        .build();

    app.connect_activate(|app| {
        aux::load_css();

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(450)
            .default_height(300)
            .resizable(false)
            .title("AutoGuard")
            .build();

        let header = HeaderBar::new();
        header.set_show_title_buttons(true);

        let empty_label = Label::new(None);
        header.set_title_widget(Some(&empty_label));

        // Create a horizontal box that fills the header bar
        let hbox = gtk::Box::new(Orientation::Horizontal, 0);
        hbox.set_hexpand(true);

        // Left‑aligned title
        let title_label = gtk::Label::new(Some("AutoGuard"));
        title_label.add_css_class("title");
        title_label.set_xalign(0.0); // left
        title_label.set_halign(gtk::Align::Start);

        // Add title to the box
        hbox.append(&title_label);

        // Add the box to the start of the header bar
        header.pack_start(&hbox);


        header.set_show_title_buttons(true);

        let configure_button = Button::from_icon_name("emblem-system-symbolic");
        configure_button.set_tooltip_text(Some("Configure"));
        configure_button.connect_clicked(|_| {
            eprintln!("Configure clicked!");
        });

        header.pack_end(&configure_button);
        window.set_titlebar(Some(&header));

        // Main content
        let root = GtkBox::new(Orientation::Vertical, 0);

        let vbox = GtkBox::new(Orientation::Vertical, 10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);

        let button = Button::with_label("Update allowed IPs");
        vbox.append(&button);

        let state = Rc::new(actions::AppState {
            window: window.clone(),
            vbox: vbox.clone(),
            button: button.clone(),
            config_path: "~/PW6/WireGuard/PW6.conf".to_string(),
        });

        button.connect_clicked(move |_| {
            actions::update_allowed_ips(&state)
        });

        root.append(&vbox);
        window.set_child(Some(&root));

        window.present();
    });

    app.run()
}
