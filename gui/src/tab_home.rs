use gtk::prelude::*;
use gtk::{gdk, gio, glib};

const PADDING: i32 = 10;

pub struct Home {
    pub grid: gtk::Grid,
    pub receiver: glib::Receiver<String>,
}

impl Home {
    pub fn new(nb: &gtk::Notebook) -> Self {
        let grid = gtk::Grid::builder()
            .margin_start(PADDING)
            .margin_end(PADDING)
            .margin_top(PADDING)
            .margin_bottom(PADDING)
            .row_spacing(PADDING)
            .column_spacing(PADDING)
            .hexpand(true)
            .build();

        let en_input = gtk::Entry::builder()
            .placeholder_text("url | .m3u8 | .m3u8")
            .hexpand(true)
            .build();

        let btn_paste = gtk::Button::from_icon_name("edit-paste");
        let btn_capture = gtk::Button::builder().label("Capture").build();

        grid.attach(&en_input, 0, 0, 1, 1);
        grid.attach(&btn_paste, 1, 0, 1, 1);
        grid.attach(&btn_capture, 0, 1, 2, 1);

        // LOGIC
        let clipboard = gdk::Display::default().unwrap().clipboard();

        btn_paste.connect_clicked(glib::clone!(@weak clipboard, @weak en_input => move |_| {
            clipboard.read_text_async(gio::Cancellable::NONE, glib::clone!(@weak en_input => move |res| {
                if let Ok(Some(text)) = res {
                    en_input.set_text(&text);
                }
            }));
        }));

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let nb = nb.clone();

        btn_capture.connect_clicked(move |_| {
            nb.set_current_page(Some(1));
            let url = en_input.text().as_str().to_owned();
            let sender = sender.clone();
            std::thread::spawn(move || crate::capture::run(url, false, sender));
        });

        Self { grid, receiver }
    }
}
