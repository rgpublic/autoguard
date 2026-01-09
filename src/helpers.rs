use gtk::gio::resources_register_include;
use gtk::{CssProvider, gdk::Display};

pub fn load_css() {

    // Register the compiled GResource
    resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    let provider = CssProvider::new();
    provider.load_from_resource("/com/example/myapp/style.css");

    let display = Display::default().unwrap();
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
