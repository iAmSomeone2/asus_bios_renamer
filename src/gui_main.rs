use crate::bios::BiosInfo;
use adw::prelude::*;
use relm4::adw::gio;
use relm4::prelude::*;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

mod bios;

const APP_ID: &str = "dev.bdavidson.BiosRenamer";

#[derive(Debug)]
enum AppInput {
    SelectFile,
    CopyAndRename,
}

struct App {
    input_path: Option<PathBuf>,
    bios_info: Option<BiosInfo>,

    input_file_dialog: gtk::FileDialog,
    output_folder_dialog: gtk::FileDialog,
    alert_dialog: gtk::AlertDialog,
}

impl Default for App {
    fn default() -> Self {
        let bios_file_filter = gtk::FileFilter::new();
        bios_file_filter.set_name(Some("BIOS Files"));
        bios_file_filter.add_suffix("cap");
        bios_file_filter.add_suffix("CAP");
        bios_file_filter.add_suffix("bin");
        bios_file_filter.add_suffix("BIN");

        let filter_list = gio::ListStore::new::<gtk::FileFilter>();
        filter_list.append(&bios_file_filter);

        let select_bios_dialog = gtk::FileDialog::builder()
            .title("Select BIOS File")
            .modal(true)
            .filters(&filter_list)
            .build();

        let output_folder_dialog = gtk::FileDialog::builder()
            .title("Select Output Folder")
            .modal(true)
            .build();

        let alert_dialog = gtk::AlertDialog::builder()
            .modal(true)
            .build();

        Self {
            input_path: None,
            bios_info: None,

            input_file_dialog: select_bios_dialog,
            output_folder_dialog,
            alert_dialog,
        }
    }
}

impl App {
    fn format_file_name(&self) -> String {
        if let Some(input_path) = self.input_path.as_ref() {
            if let Some(name) = input_path.file_name() {
                String::from(name.to_string_lossy())
            } else {
                String::from("?")
            }
        } else {
            String::from("No file selected")
        }
    }

    fn format_board_name(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => {
                let board_name = bios_info.get_board_name();
                let brand = bios_info.get_brand();
                format!("{brand} {board_name}")
            },
            None => String::new()
        }
    }

    fn format_build_date(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => {
                let build_date = bios_info.get_build_date();
                format!("{build_date}")
            },
            None => String::new()
        }
    }

    fn format_build_number(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => {
                bios_info.get_build_number().clone()
            },
            None => String::new()
        }
    }

    fn format_expected_name(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => {
                bios_info.get_expected_name().clone()
            },
            None => String::new()
        }
    }

    fn show_alert_with_message<E: Display>(&self, msg: E, root: &impl IsA<gtk::Window>) {
        self.alert_dialog.set_message(&format!("{}", msg));
        self.alert_dialog.show(Some(root));
    }

    async fn set_input_file(&mut self, root: &impl IsA<gtk::Window>) -> anyhow::Result<Option<File>> {
        match self.input_file_dialog.open_future(Some(root)).await {
            Ok(selected_file) => {
                let selected_path = if let Some(path) = selected_file.path() {
                    path
                } else {
                    self.input_path = None;
                    return Err(anyhow::Error::msg("Failed to get path to selected file."));
                };

                // Close GioFile
                drop(selected_file);
                let selected_file = match File::open(&selected_path) {
                    Ok(file) => file,
                    Err(err) => {
                        self.input_path = None;
                        return Err(anyhow::Error::msg(format!("Failed to open selected file: {}", err)));
                    }
                };

                match bios::validate_file(&selected_file) {
                    Ok(_) => {
                        self.input_path = Some(selected_path);
                        Ok(Some(selected_file))
                    },
                    Err(err) => {
                        self.input_path = None;
                        Err(anyhow::Error::msg(format!("{}", err)))
                    }
                }
            }
            Err(_) => {
                self.input_path = None;
                Ok(None)
            },
        }
    }

    async fn load_input_file(&mut self, input_file: &mut File) -> anyhow::Result<()> {
        let bios_info = BiosInfo::from_file(input_file)?;

        self.bios_info = Some(bios_info);

        Ok(())
    }

    async fn handle_select_file(&mut self, root: &impl IsA<gtk::Window>) {
        let input_file = match self.set_input_file(root).await {
            Ok(file) => file,
            Err(err) => {
                self.show_alert_with_message(err, root);
                return;
            }
        };

        if input_file.is_none() || self.input_path.is_none() {
            return;
        }

        let mut input_file = input_file.unwrap();

        match self.load_input_file(&mut input_file).await {
            Ok(_) => {},
            Err(err) => {
                self.input_path = None;
                self.show_alert_with_message(err, root);
            }
        }
    }

    async fn handle_select_output_folder(&mut self, root: &impl IsA<gtk::Window>) {
        if let Some(selected_path) = self.input_path.as_ref() {
            if let Some(parent_dir) = selected_path.parent() {
                let gio_folder = gio::File::for_path(parent_dir);
                self.output_folder_dialog.set_initial_folder(Some(&gio_folder));
            }
        } else {
            self.show_alert_with_message("Input file must be selected.", root);
            return;
        };

        let output_folder = match self.output_folder_dialog.select_folder_future(Some(root)).await {
            Ok(selected_folder) => {
                selected_folder.path()
            },
            Err(_) => None,
        };

        if output_folder.is_none() {
            return;
        }
        let output_folder = output_folder.unwrap();

        // Check if we have write permissions
        let can_write: bool = if let Ok(metadata) = fs::metadata(&output_folder) {
            !metadata.permissions().readonly()
        } else {
            false
        };

        if !can_write {
            self.show_alert_with_message("Cannot write to selected folder. Please check permissions or select a different folder.", root);
            return;
        }

        // Copy file to target directory with correct name

        let cap_name: String = if let Some(bios_info) = self.bios_info.as_ref() {
            bios_info.get_expected_name().clone()
        } else {
            self.show_alert_with_message("BIOS info missing.", root);
            return;
        };

        let input_path = self.input_path.as_ref().expect("Input path should be valid.").clone();
        let target_path = output_folder.join(cap_name);

        if input_path == target_path {
            self.show_alert_with_message("Input and output files cannot be the same. Please choose a different location.", root);
            return;
        }

        match fs::copy(input_path, target_path) {
            Ok(_) => {
                self.show_alert_with_message("File copied and renamed!", root);
            },
            Err(err) => self.show_alert_with_message(err, root),
        }
    }
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some("BIOS Renamer"),
            set_resizable: false,

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_margin_all: 8,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_margin_all: 8,

                        gtk::Button::with_label("Select file...") {
                            connect_clicked => Self::Input::SelectFile,
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &model.format_file_name(),
                            set_selectable: false,
                            set_wrap: true,
                        },
                    },

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    // \/ BIOS Info View \/
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                        set_margin_all: 8,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_margin_all: 4,

                            gtk::Label {
                                set_label: "Board model:",
                                set_selectable: false,
                                set_wrap: true,
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &model.format_board_name(),
                                set_selectable: true,
                                set_wrap: true,
                            },
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_margin_all: 4,

                            gtk::Label {
                                set_label: "Build date:",
                                set_selectable: false,
                                set_wrap: true,
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &model.format_build_date(),
                                set_selectable: true,
                                set_wrap: true,
                            },
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_margin_all: 4,

                            gtk::Label {
                                set_label: "Build number:",
                                set_selectable: false,
                                set_wrap: true,
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &model.format_build_number(),
                                set_selectable: true,
                                set_wrap: true,
                            },
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,
                            set_margin_all: 4,

                            gtk::Label {
                                set_label: "Expected name:",
                                set_selectable: false,
                                set_wrap: true,
                            },

                            gtk::Label {
                                #[watch]
                                set_label: &model.format_expected_name(),
                                set_selectable: true,
                                set_wrap: true,
                            },
                        },
                    },
                    // /\ BIOS Info View /\

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    #[name = "select_output_btn"]
                    gtk::Button::with_label("Copy and rename file...") {
                        #[watch]
                        set_sensitive: model.bios_info.is_some(),
                        connect_clicked => Self::Input::CopyAndRename,
                    },
                }
            },
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = App::default();

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppInput::SelectFile => self.handle_select_file(root).await,
            AppInput::CopyAndRename => self.handle_select_output_folder(root).await,
        }
    }
}



fn main() {
    let app = RelmApp::new(APP_ID);
    app.run_async::<App>(());
}
