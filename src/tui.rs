// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::actions::instantiate_storage;
use crate::{Asset, Assets, Config, Result};

use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};

pub fn main(config: &Config) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let assets = storage.list()?;
    ratatui::run(|terminal| App::new(assets).run(terminal))
}

struct App {
    assets: Assets,
    selected_index: usize,
}

impl App {
    fn new(assets: Assets) -> Self {
        Self {
            assets,
            selected_index: 0,
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
            .constraints([Constraint::Length(9), Constraint::Fill(1)])
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
