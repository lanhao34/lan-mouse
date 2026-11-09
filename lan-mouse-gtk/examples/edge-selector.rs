use gtk::prelude::*;
fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("de.feschber.LanMouse.EdgeSelector")
        .build();
    app.connect_startup(|app| {
        let quit = gtk::gio::SimpleAction::new("quit", None);
        let weak = app.downgrade();
        quit.connect_activate(move |_, _| {
            if let Some(app) = weak.upgrade() {
                app.quit();
            }
        });
        app.add_action(&quit);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
        let menu = gtk::gio::Menu::new();
        let file = gtk::gio::Menu::new();
        file.append(Some("Quit"), Some("app.quit"));
        menu.append_submenu(Some("File"), &file);
        app.set_menubar(Some(&menu));
    });
    app.connect_activate(lan_mouse_gtk::edge_editor::show);
    app.run()
}
