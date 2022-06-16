use gtk::prelude::*;

fn main() {
    let app = gtk::Application::builder()
        .application_id("com.clitic.vsd")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &gtk::Application) {
    let nb = gtk::Notebook::builder()
        .tab_pos(gtk::PositionType::Top)
        .build();

    let home = gui::tab_home::Home::new(&nb);
    let captures = gui::tab_captures::Captures::new(&nb, home.receiver);

    nb.append_page(&home.grid, Some(&gtk::Label::new(Some("Home"))));
    nb.append_page(&captures.grid, Some(&gtk::Label::new(Some("Captures"))));

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Video Stream Downloader")
        .default_height(400)
        .default_width(600)
        .child(&nb)
        .build();

    window.present();
}
