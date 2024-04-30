// MIT License
//
// Copyright (c) 2021-2024 Brenden Davidson
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use adw::prelude::*;
use relm4::adw::gio;
use relm4::prelude::*;
use relm4::MessageBroker;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;

use crate::bios::BiosInfo;
use crate::bios_info_view::BiosInfoView;

mod bios;
mod bios_info_view;

const APP_ID: &str = "dev.bdavidson.BiosRenamer";

#[derive(Debug)]
enum AppInput {
    SelectFile,
    CopyAndRename,
}

#[derive(Debug)]
pub enum InfoState {
    BiosInfoUpdated(Option<BiosInfo>),
}

#[derive(Debug)]
pub enum CommandMsg {
    LoadInputFile(Result<BiosInfo, std::io::Error>),
}

struct App {
    input_path: Option<PathBuf>,
    bios_info: Option<BiosInfo>,
    has_valid_input: bool,
    loading: bool,

    bios_info_view: Controller<BiosInfoView>,

    input_file_dialog: gtk::FileDialog,
    output_folder_dialog: gtk::FileDialog,
    alert_dialog: gtk::AlertDialog,
}

static INFO_STATE_BROKER: MessageBroker<InfoState> = MessageBroker::new();

impl Default for App {
    fn default() -> Self {
        let bios_file_filter = gtk::FileFilter::new();
        bios_file_filter.set_name(Some("BIOS Files"));
        bios_file_filter.add_suffix("cap");
        bios_file_filter.add_suffix("CAP");
        bios_file_filter.add_suffix("bin");
        bios_file_filter.add_suffix("BIN");

        let any_file_filter = gtk::FileFilter::new();
        any_file_filter.set_name(Some("All Files"));
        any_file_filter.add_mime_type("application/octet-stream");

        let filter_list = gio::ListStore::new::<gtk::FileFilter>();
        filter_list.append(&bios_file_filter);
        filter_list.append(&any_file_filter);

        let select_bios_dialog = gtk::FileDialog::builder()
            .title("Select BIOS File")
            .modal(true)
            .filters(&filter_list)
            .default_filter(&bios_file_filter)
            .build();

        let output_folder_dialog = gtk::FileDialog::builder()
            .title("Select Output Folder")
            .modal(true)
            .build();

        let alert_dialog = gtk::AlertDialog::builder().modal(true).build();

        let bios_info_view = BiosInfoView::builder()
            .launch_with_broker((), &INFO_STATE_BROKER)
            .detach();

        Self {
            input_path: None,
            bios_info: None,
            has_valid_input: false,
            loading: false,

            bios_info_view,

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

    fn show_alert_with_message<E: Display>(&self, msg: E, root: &impl IsA<gtk::Window>) {
        self.alert_dialog.set_message(&format!("{}", msg));
        self.alert_dialog.show(Some(root));
    }

    fn clear_input(&mut self) {
        self.input_path = None;
        self.bios_info = None;
        self.has_valid_input = false;
    }

    async fn set_input_file(
        &mut self,
        root: &impl IsA<gtk::Window>,
    ) -> anyhow::Result<Option<File>> {
        match self.input_file_dialog.open_future(Some(root)).await {
            Ok(selected_file) => {
                let selected_path = if let Some(path) = selected_file.path() {
                    path
                } else {
                    self.clear_input();
                    return Err(anyhow::Error::msg("Failed to get path to selected file."));
                };

                // Close GioFile
                drop(selected_file);
                let selected_file = match File::open(&selected_path) {
                    Ok(file) => file,
                    Err(err) => {
                        self.clear_input();
                        return Err(anyhow::Error::msg(format!(
                            "Failed to open selected file: {}",
                            err
                        )));
                    }
                };

                match bios::validate_file(&selected_file) {
                    Ok(_) => {
                        self.input_path = Some(selected_path);
                        Ok(Some(selected_file))
                    }
                    Err(err) => {
                        self.clear_input();
                        Err(anyhow::Error::msg(format!("{}", err)))
                    }
                }
            }
            Err(_) => {
                self.clear_input();
                Ok(None)
            }
        }
    }

    async fn handle_select_file(&mut self, root: &impl IsA<gtk::Window>) -> Option<File> {
        match self.set_input_file(root).await {
            Ok(file) => file,
            Err(err) => {
                self.show_alert_with_message(err, root);
                None
            }
        }
    }

    async fn handle_select_output_folder(&mut self, root: &impl IsA<gtk::Window>) {
        if let Some(selected_path) = self.input_path.as_ref() {
            if let Some(parent_dir) = selected_path.parent() {
                let gio_folder = gio::File::for_path(parent_dir);
                self.output_folder_dialog
                    .set_initial_folder(Some(&gio_folder));
            }
        } else {
            self.show_alert_with_message("Input file must be selected.", root);
            return;
        };

        let output_folder = match self
            .output_folder_dialog
            .select_folder_future(Some(root))
            .await
        {
            Ok(selected_folder) => selected_folder.path(),
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

        let input_path = self
            .input_path
            .as_ref()
            .expect("Input path should be valid.")
            .clone();
        let target_path = output_folder.join(cap_name);

        if input_path == target_path {
            self.show_alert_with_message(
                "Input and output files cannot be the same. Please choose a different location.",
                root,
            );
            return;
        }

        match fs::copy(input_path, target_path) {
            Ok(_) => {
                self.show_alert_with_message("File copied and renamed!", root);
            }
            Err(err) => self.show_alert_with_message(err, root),
        }
    }
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppInput;
    type Output = InfoState;
    type CommandOutput = CommandMsg;

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
                            #[watch]
                            set_sensitive: !model.loading,
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
                    // TODO: Add working progress bar

                    append = model.bios_info_view.widget(),

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    #[name = "select_output_btn"]
                    gtk::Button::with_label("Copy and rename file...") {
                        #[watch]
                        set_sensitive: model.has_valid_input && !model.loading,
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
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppInput::SelectFile => {
                if let Some(mut input_file) = self.handle_select_file(root).await {
                    self.loading = true;
                    sender.spawn_oneshot_command(move || {
                        CommandMsg::LoadInputFile(BiosInfo::from_file(&mut input_file))
                    });
                }
            }
            AppInput::CopyAndRename => self.handle_select_output_folder(root).await,
        }
    }

    async fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::LoadInputFile(bios_info_result) => {
                self.loading = false;
                match bios_info_result {
                    Ok(bios_info) => {
                        self.bios_info = Some(bios_info);
                        self.has_valid_input = true;
                        INFO_STATE_BROKER.send(InfoState::BiosInfoUpdated(self.bios_info.clone()));
                    }
                    Err(err) => {
                        if err.kind() == ErrorKind::InvalidData {
                            self.show_alert_with_message(
                                "Selected file does not appear to be a valid ASUS BIOS.",
                                root,
                            );
                        } else {
                            self.show_alert_with_message(
                                format!("An error occurred while reading the file: {}", err),
                                root,
                            );
                        }
                        self.clear_input();
                        INFO_STATE_BROKER.send(InfoState::BiosInfoUpdated(self.bios_info.clone()));
                    }
                }
            }
        }
    }
}

fn main() {
    let app = RelmApp::new(APP_ID);
    app.run_async::<App>(());
}
