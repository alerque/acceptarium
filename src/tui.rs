// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::actions::instantiate_storage;
use crate::output;
use crate::{Asset, Assets, Config, Result};

use std::sync::mpsc;

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

pub fn main(config: &Config) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let assets = storage.list()?;
    ratatui::run(|terminal| App::new(assets, config).run(terminal))
}

struct App {
    assets: Assets,
    selected_index: usize,
    scroll_offset: usize,
    picker: Picker,
    image_state: Option<StatefulProtocol>,
    image_loader: Option<ImageLoader>,
    load_generation: u64,
    config: Config,
    details_available_height: usize,
}

struct ImageLoader {
    receiver: mpsc::Receiver<(u64, Option<StatefulProtocol>)>,
}

impl App {
    fn new(assets: Assets, config: &Config) -> Self {
        Self {
            assets,
            selected_index: 0,
            scroll_offset: 0,
            picker: Picker::from_query_stdio().unwrap(),
            image_state: None,
            image_loader: None,
            load_generation: 0,
            config: config.clone(),
            details_available_height: 0,
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
            self.scroll_offset = 0;
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
            self.scroll_offset = 0;
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
        let Some(path) = asset.asset_path(&self.config.project) else {
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
            .constraints([Constraint::Fill(2), Constraint::Fill(1)])
            .split(details_pane);
        let available_height = detail_areas[0].height.saturating_sub(2) as usize;
        self.details_available_height = available_height;
        let details_text = match self.selected_asset() {
            Some(asset) => self.format_asset_details(asset),
            None => "No asset selected".to_string(),
        };
        let content_height = details_text.lines().count();
        let max_scroll = content_height.saturating_sub(available_height);
        let clamped_scroll = self.scroll_offset.min(max_scroll);
        let mut details_paragraph =
            Paragraph::new(details_text).block(Block::default().title("Details").borders(borders));
        if content_height > available_height {
            details_paragraph = details_paragraph.scroll((clamped_scroll as u16, 0));
        }
        frame.render_widget(details_paragraph, detail_areas[0]);
        let export_content = match self.selected_asset() {
            Some(asset) => {
                let export_text = self.format_export_output(asset);
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
            if event::poll(std::time::Duration::from_millis(50)).unwrap_or(false)
                && let Ok(event::Event::Key(key)) = event::read()
                && key.kind == KeyEventKind::Press
            {
                let shift = key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SHIFT);
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => match shift {
                        false => self.select_next(),
                        true => self.scroll_down(false),
                    },
                    KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => match shift {
                        false => self.select_previous(),
                        true => self.scroll_up(false),
                    },
                    KeyCode::PageDown => self.scroll_down(true),
                    KeyCode::PageUp => self.scroll_up(true),
                    KeyCode::Char('P') => self.toggle_preview(),
                    _ => {}
                }
            }
        }
    }

    fn format_asset_details(&self, asset: &Asset) -> String {
        output::dump(&self.config, asset).unwrap_or_default()
    }

    fn format_export_output(&self, asset: &Asset) -> String {
        let mut assets = Assets::new();
        assets.add(asset.clone());
        output::export(&self.config, &assets).unwrap_or_default()
    }

    fn max_scroll_offset(&self) -> usize {
        let Some(asset) = self.selected_asset() else {
            return 0;
        };
        let details_text = self.format_asset_details(asset);
        let content_height = details_text.lines().count();
        content_height.saturating_sub(self.details_available_height)
    }

    fn scroll_down(&mut self, page: bool) {
        let max = self.max_scroll_offset();
        let inc = if page {
            self.details_available_height.saturating_sub(1)
        } else {
            1
        };
        self.scroll_offset = (self.scroll_offset + inc).min(max);
    }

    fn scroll_up(&mut self, page: bool) {
        let inc = if page {
            self.details_available_height.saturating_sub(1)
        } else {
            1
        };
        self.scroll_offset = self.scroll_offset.saturating_sub(inc);
    }
}
