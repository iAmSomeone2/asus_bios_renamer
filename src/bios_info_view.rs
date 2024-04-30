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

use relm4::gtk::prelude::*;
use relm4::prelude::*;

use crate::bios::BiosInfo;
use crate::InfoState;

#[derive(Default)]
pub struct BiosInfoView {
    bios_info: Option<BiosInfo>,
}

impl BiosInfoView {
    fn get_board_name(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => {
                let board_name = bios_info.get_board_name();
                let brand = bios_info.get_brand();
                format!("{brand} {board_name}")
            }
            None => String::default(),
        }
    }

    fn get_build_date(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => {
                let build_date = bios_info.get_build_date();
                format!("{build_date}")
            }
            None => String::new(),
        }
    }

    fn get_build_number(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => bios_info.get_build_number().clone(),
            None => String::new(),
        }
    }

    fn get_expected_name(&self) -> String {
        match self.bios_info.as_ref() {
            Some(bios_info) => bios_info.get_expected_name().clone(),
            None => String::new(),
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for BiosInfoView {
    type Init = ();
    type Input = InfoState;
    type Output = ();

    view! {
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
                    set_label: &model.get_board_name(),
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
                    set_label: &model.get_build_date(),
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
                    set_label: &model.get_build_number(),
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
                    set_label: &model.get_expected_name(),
                    set_selectable: true,
                    set_wrap: true,
                },
            },
        },
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = BiosInfoView::default();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            Self::Input::BiosInfoUpdated(bios_info) => {
                self.bios_info = bios_info;
            }
        }
    }
}
