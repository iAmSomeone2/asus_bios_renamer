use std::path::PathBuf;
use adw::prelude::*;
use relm4::prelude::*;

const APP_ID: &str = "dev.bdavidson.BiosRenamer";

struct App {
    input_path: Option<PathBuf>,
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = Option<PathBuf>;
    type Input = ();
    type Output = ();


    view! {
        gtk::Window {
            set_title: Some("BIOS Renamer"),
            set_default_size: (300, 100),

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_margin_all: 8,

                gtk::Button {
                    set_label: "Select file..."
                },

                gtk::Separator {
                    set_orientation: gtk::Orientation::Vertical,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,

                },

                gtk::Separator {
                    set_orientation: gtk::Orientation::Vertical,
                },

                gtk::Button {
                    set_label: "Select output folder..."
                },
            }
        }
    }

    fn init(input_path: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = App { input_path };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        todo!()
    }
}

fn main() {
    let app = RelmApp::new(APP_ID);
    app.run::<App>(None);
}