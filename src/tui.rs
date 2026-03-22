// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::actions::instantiate_storage;
use crate::{Asset, Assets, Config, Result};

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use std::env::current_dir;
use std::sync::mpsc;

pub fn main(config: &Config) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let assets = storage.list()?;
    ratatui::run(|terminal| App::new(assets, config).run(terminal))
}

struct App {
    assets: Assets,
    selected_index: usize,
    picker: Picker,
    image_state: Option<StatefulProtocol>,
    image_loader: Option<ImageLoader>,
    load_generation: u64,
    config: Config,
}

struct ImageLoader {
    receiver: mpsc::Receiver<(u64, Option<StatefulProtocol>)>,
}

impl App {
    fn new(assets: Assets, config: &Config) -> Self {
        Self {
            assets,
            selected_index: 0,
            picker: Picker::from_query_stdio().unwrap(),
            image_state: None,
            image_loader: None,
            load_generation: 0,
            config: config.clone(),
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
        self.trigger_image_load();
    }

    fn select_previous(&mut self) {
        if self.len() > 0 {
            self.selected_index = if self.selected_index == 0 {
                self.len() - 1
            } else {
                self.selected_index - 1
            };
        }
        self.trigger_image_load();
    }

    fn toggle_preview(&mut self) {
        self.config.tui.preview = !self.config.tui.preview;
    }

    fn trigger_image_load(&mut self) {
        self.load_generation = self.load_generation.wrapping_add(1);
        let this_gen = self.load_generation;
        self.image_state = None;
        self.image_loader = None;
        let Some(asset) = self.selected_asset() else {
            return;
        };
        let Some(path) = asset.asset_path(&current_dir().unwrap_or_default()) else {
            return;
        };
        let picker = self.picker.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let img = match image::open(&path) {
                Ok(img) => img,
                Err(_) => {
                    let _ = tx.send((this_gen, None));
                    return;
                }
            };
            let protocol = picker.new_resize_protocol(img);
            let _ = tx.send((this_gen, Some(protocol)));
        });
        self.image_loader = Some(ImageLoader { receiver: rx });
    }

    fn update_image_state(&mut self) {
        let Some(loader) = &self.image_loader else {
            return;
        };
        if let Ok((generation, protocol_opt)) = loader.receiver.try_recv() {
            if generation == self.load_generation {
                self.image_state = protocol_opt;
            }
            self.image_loader = None;
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        self.update_image_state();
        let borders = Borders::ALL;
        let constraints = if self.config.tui.preview {
            [
                Constraint::Length(9),
                Constraint::Fill(3),
                Constraint::Fill(2),
            ]
        } else {
            [
                Constraint::Length(9),
                Constraint::Fill(1),
                Constraint::Fill(0),
            ]
        };
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
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
        let details_pane = panes[1];
        let detail_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(3), Constraint::Fill(2)])
            .split(details_pane);
        let details = match self.selected_asset() {
            Some(asset) => {
                let details_text = format_asset_details(asset);
                Paragraph::new(details_text)
                    .block(Block::default().title("Details").borders(borders))
            }
            None => Paragraph::new("No asset selected"),
        };
        frame.render_widget(details, detail_areas[0]);
        let export_content = match self.selected_asset() {
            Some(asset) => {
                let export_text = format_export_output(&self.config, asset);
                Paragraph::new(export_text)
                    .block(Block::default().title("Export Preview").borders(borders))
            }
            None => Paragraph::new(""),
        };
        frame.render_widget(export_content, detail_areas[1]);
        if self.config.tui.preview {
            let preview_block = Block::default().title("Image Preview").borders(borders);
            frame.render_widget(&preview_block, panes[2]);
            let preview_area = preview_block.inner(panes[2]);
            match &mut self.image_state {
                Some(state) => {
                    frame.render_stateful_widget(StatefulImage::default(), preview_area, state);
                }
                None if self.image_loader.is_some() => {
                    let loading =
                        Paragraph::new("Loading...").style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(loading, preview_area);
                }
                _ => {}
            }
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.trigger_image_load();
        loop {
            terminal.draw(|frame| self.render(frame))?;
            if event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(event::Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
                            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
                            KeyCode::Char('P') => self.toggle_preview(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn format_asset_details(asset: &Asset) -> String {
    format!("{:#?}", asset)
}

fn format_export_output(config: &Config, asset: &Asset) -> String {
    config
        .template
        .render(config, asset)
        .unwrap_or_else(|e| format!("Export error: {}", e))
}
