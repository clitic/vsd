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
        list.set_placeholder(Some(&gtk::Label::new(Some("Requests made to fetch .m3u8 (HLS) and .mpd (Dash) files will be shown here."))));

        let scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .build();

        let btn_download = gtk::Button::builder()
            .label("Download")
            .icon_name("edit-copy")
            .sensitive(false)
            .hexpand(true)
            .build();
        let btn_copy = gtk::Button::builder().icon_name("edit-copy").sensitive(false).build();

        grid.attach(&scroll, 0, 0, 2, 1);
        grid.attach(&btn_download, 0, 1, 1, 1);
        grid.attach(&btn_copy, 1, 1, 1, 1);

        // LOGIC

        // Enable and disable download button on type of selected row.
        let btn_download_c = btn_download.clone();
        list.connect_row_selected(move |_, selected_row| {
            if selected_row.unwrap()
            .child()
            .unwrap()
            .downcast_ref::<gtk::Label>()
            .unwrap()
            .text()
            .contains(".m3u") {
                btn_download_c.set_sensitive(true);
            } else {
                btn_download_c.set_sensitive(false);
            }
        });
        
        // Copy text of selected row to clipboard.
        let list_c = list.clone();
        btn_copy.connect_clicked(move |_| {
            gdk::Display::default().unwrap().clipboard().set_text(
                list_c.selected_row()
                    .unwrap()
                    .child()
                    .unwrap()
                    .downcast_ref::<gtk::Label>()
                    .unwrap()
                    .text()
                    .as_str(),
            );
        });

        // Attach receiver for list appends.
        let btn_copy_c = btn_copy.clone();
        receiver.attach(
            None,
            glib::clone!(@weak list => @default-return glib::Continue(false),
                move |url| {
                    list.append(&gtk::Label::builder().label(&url).halign(gtk::Align::Start).wrap(true).build());
                    
                    if !btn_copy_c.is_sensitive() {
                        list.select_row(list.row_at_index(0).as_ref());
                        btn_copy_c.set_sensitive(true);
                    }

                    glib::Continue(true)
                }
            ),
        );

        Self { grid }
    }
}
