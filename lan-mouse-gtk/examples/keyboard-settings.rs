use gtk::prelude::*;
fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("de.feschber.LanMouse.KeyboardPreview")
        .build();
    app.connect_activate(lan_mouse_gtk::show_keyboard_settings_preview);
    app.run()
}
