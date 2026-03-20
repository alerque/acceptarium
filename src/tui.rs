// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::actions::instantiate_storage;
use crate::{Asset, Assets, Config, Result};

use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::env::current_dir;

pub fn main(config: &Config) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let assets = storage.list()?;
    ratatui::run(|terminal| App::new(assets).run(terminal))
}

struct App {
    assets: Assets,
    selected_index: usize,
    picker: Picker,
    image_state: Option<StatefulProtocol>,
}

impl App {
    fn new(assets: Assets) -> Self {
        Self {
            assets,
            selected_index: 0,
            picker: Picker::from_query_stdio().unwrap(),
            image_state: None,
        }
    }

    fn asset_list(&self) -> Vec<&Asset> {
        self.assets.iter().map(|(_, asset)| asset).collect()
    }

    fn selected_asset(&self) -> Option<&Asset> {
        let list = self.asset_list();
        list.get(self.selected_index).copied()
    }

    fn len(&self) -> usize {
        self.assets.iter().count()
    }

    fn select_next(&mut self) {
        if self.len() > 0 {
            self.selected_index = (self.selected_index + 1) % self.len();
        }
    }

    fn select_previous(&mut self) {
        if self.len() > 0 {
            self.selected_index = if self.selected_index == 0 {
                self.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let borders = Borders::ALL;
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(9),
                Constraint::Fill(3),
                Constraint::Fill(2),
            ])
            .split(frame.area());
        let asset_list: Vec<ListItem> = self
            .asset_list()
            .iter()
            .enumerate()
            .map(|(i, asset)| {
                let style = if i == self.selected_index {
                    Style::default().fg(Color::LightBlue).bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{}", asset.id())).style(style)
            })
            .collect();
        let asset_picker = List::new(asset_list)
            .block(Block::default().title("Assets").borders(borders))
            .highlight_style(Style::default().fg(Color::LightBlue).bg(Color::DarkGray));
        frame.render_widget(asset_picker, panes[0]);
        let details = match self.selected_asset() {
            Some(asset) => {
                let details_text = format_asset_details(asset);
                Paragraph::new(details_text)
                    .block(Block::default().title("Details").borders(borders))
            }
            None => Paragraph::new("No asset selected"),
        };
        frame.render_widget(details, panes[1]);
        let preview_block = Block::default().title("Preview").borders(borders);
        frame.render_widget(&preview_block, panes[2]);
        let preview_area = preview_block.inner(panes[2]);
        if let Some(asset) = self.selected_asset() {
            let cwd = current_dir().unwrap_or_default();
            if let Some(path) = asset.asset_path(&cwd)
                && let Ok(dyn_img) = image::open(&path)
            {
                self.image_state = Some(self.picker.new_resize_protocol(dyn_img));
            }
        }
        if let Some(ref mut state) = self.image_state {
            let image_widget = StatefulImage::default();
            frame.render_stateful_widget(image_widget, preview_area, state);
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => self.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
                    _ => {}
                }
            }
        }
    }
}

fn format_asset_details(asset: &Asset) -> String {
    format!("{:#?}", asset)
}
