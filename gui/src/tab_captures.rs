use gtk::prelude::*;
use gtk::{gdk, glib};

const PADDING: i32 = 10;

pub struct Captures {
    pub grid: gtk::Grid,
}

impl Captures {
    pub fn new(_nb: &gtk::Notebook, receiver: glib::Receiver<String>) -> Self {
        let grid = gtk::Grid::builder()
            .margin_start(PADDING)
            .margin_end(PADDING)
            .margin_top(PADDING)
            .margin_bottom(PADDING)
            .row_spacing(PADDING)
            .column_spacing(PADDING)
            .hexpand(true)
            .build();

        let list = gtk::ListBox::builder()
            .hexpand(true)
            .show_separators(true)
            .build();
        list.set_placeholder(Some(&gtk::Label::builder().label("Requests made for .m3u8 (HLS) and .mpd (Dash) file extensions will be shown here.").build()));

        let scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .build();

        let btn_download = gtk::Button::builder()
            .label("Download")
            .hexpand(true)
            .build();
        let btn_copy = gtk::Button::from_icon_name("edit-copy");

        grid.attach(&scroll, 0, 0, 2, 1);
        grid.attach(&btn_download, 0, 1, 1, 1);
        grid.attach(&btn_copy, 1, 1, 1, 1);

        // LOGIC
        receiver.attach(
            None,
            glib::clone!(@weak list => @default-return glib::Continue(false),
                move |url| {
                    list.append(&gtk::Label::builder().label(&url).halign(gtk::Align::Start).wrap(true).build());
                    list.select_row(list.row_at_index(0).as_ref());
                    glib::Continue(true)
                }
            ),
        );

        let clipboard = gdk::Display::default().unwrap().clipboard();

        btn_copy.connect_clicked(move |_| {
            clipboard.set_text(
                list.selected_row()
                    .unwrap()
                    .child()
                    .unwrap()
                    .downcast_ref::<gtk::Label>()
                    .unwrap()
                    .text()
                    .as_str(),
            );
        });

        Self { grid }
    }
}
