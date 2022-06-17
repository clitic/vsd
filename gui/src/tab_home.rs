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
        let en_command = gtk::Entry::builder()
            .text("vsd --help")
            .placeholder_text("Updated command will be shown here")
            .editable(false)
            .hexpand(true)
            .build();

        let btn_paste = gtk::Button::from_icon_name("edit-paste");
        let btn_execute = gtk::Button::builder().label("Capture").build();
        let btn_copy = gtk::Button::from_icon_name("edit-copy");

        let grid2 = gtk::Grid::builder()
            .margin_start(PADDING)
            .margin_end(PADDING)
            .margin_top(PADDING)
            .margin_bottom(PADDING)
            .row_spacing(PADDING)
            .column_spacing(PADDING)
            .hexpand(true)
            .build();

        let alternative = gtk::Button::builder()
            .label("Download Only Alternative Streams")
            .build();
        let skip = gtk::Button::builder()
            .label("Skip Downloading Alternative Streams")
            .build();

        alternative.connect_clicked(|btn| {
            if !(btn.opacity() == 1.0) {
                btn.set_opacity(1.0);
            } else {
                btn.set_opacity(0.5);
            }
        });

        grid2.attach(
            &gtk::Label::builder()
                .label("Options")
                .halign(gtk::Align::Start)
                .build(),
            0,
            0,
            2,
            1,
        );
        grid2.attach(&alternative, 0, 1, 1, 1);
        grid2.attach(&skip, 1, 1, 1, 1);

        let scroll = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&grid2)
            .build();

        grid.attach(&en_input, 0, 0, 1, 1);
        grid.attach(&btn_paste, 1, 0, 1, 1);
        grid.attach(&btn_execute, 0, 1, 2, 1);
        grid.attach(&scroll, 0, 2, 2, 1);
        grid.attach(&en_command, 0, 3, 1, 1);
        grid.attach(&btn_copy, 1, 3, 1, 1);

        // let question_dialog = gtk::MessageDialog::builder()
        // .modal(true)
        // .buttons(gtk::ButtonsType::OkCancel)
        // .text("What is your answer?")
        // .build();

        // question_dialog.run_async(|obj, answer| {
        //     obj.close();
        //     println!("Answer: {:#?}", answer);
        // });

        // LOGIC

        // Change execute button label on basis of input text.
        let btn_execute_c = btn_execute.clone();
        let re = regex::Regex::new(r"(https|ftp|http)://([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-]\.(m3u8|m3u|mpd))").unwrap();
        en_input.connect_changed(move |x| {
            let url = x.text().to_string().clone();

            if crate::utils::find_hls_dash_links(&url, &re).len() == 0 {
                if !std::path::Path::new(&url).exists() {
                    btn_execute_c.set_label("Capture");
                }
            } else {
                btn_execute_c.set_label("Download");
            }
        });

        // Paste input from clipboard text
        let clipboard = gdk::Display::default().unwrap().clipboard();
        btn_paste.connect_clicked(glib::clone!(@weak clipboard, @weak en_input => move |_| {
            clipboard.read_text_async(gio::Cancellable::NONE, glib::clone!(@weak en_input => move |res| {
                if let Ok(Some(text)) = res {
                    en_input.set_text(&text);
                }
            }));
        }));

        // Copy command to clipboard.
        let en_command_c = en_command.clone();
        btn_copy.connect_clicked(move |btn| {
            btn.clipboard().set_text(en_command_c.text().as_str());
        });

        // Execute button logic.
        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let nb = nb.clone();
        btn_execute.connect_clicked(move |btn| {
            let url = en_input.text().as_str().to_owned();

            if btn.label().unwrap() == "Capture" {
                nb.set_current_page(Some(1));
                let sender = sender.clone();
                std::thread::spawn(move || crate::capture::run(url, false, sender));
            }
        });

        Self { grid, receiver }
    }
}
