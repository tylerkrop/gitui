use crate::{
	app::Environment,
	components::{
		CommandBlocking, CommandInfo, Component, DrawableComponent,
		EventState,
	},
	keys::{key_match, SharedKeyConfig},
	queue::{InternalEvent, Queue},
	review::SharedReviewStore,
	strings, try_or_popup,
	ui::style::SharedTheme,
};
use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
	Frame,
};
use std::borrow::Cow;

pub struct ReviewTab {
	visible: bool,
	review_store: SharedReviewStore,
	selection: usize,
	queue: Queue,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl ReviewTab {
	pub fn new(
		env: &Environment,
		review_store: SharedReviewStore,
	) -> Self {
		Self {
			visible: false,
			review_store,
			selection: 0,
			queue: env.queue.clone(),
			theme: env.theme.clone(),
			key_config: env.key_config.clone(),
		}
	}

	fn move_selection(&mut self, up: bool) {
		let count = self.review_store.borrow().len();
		if count == 0 {
			return;
		}
		let max = count.saturating_sub(1);
		if up {
			self.selection = self.selection.saturating_sub(1);
		} else {
			self.selection = std::cmp::min(self.selection + 1, max);
		}
	}

	fn copy_all(&self) {
		let store = self.review_store.borrow();
		if store.is_empty() {
			return;
		}
		let text = store.format_all();
		try_or_popup!(
			self,
			"copy to clipboard error:",
			crate::clipboard::copy_string(&text)
		);
	}

	fn delete_selected(&mut self) {
		let count = self.review_store.borrow().len();
		if count > 0 && self.selection < count {
			self.review_store.borrow_mut().remove(self.selection);
			let new_count = self.review_store.borrow().len();
			if self.selection >= new_count && new_count > 0 {
				self.selection = new_count - 1;
			}
		}
	}

	fn clear_all(&mut self) {
		self.review_store.borrow_mut().clear();
		self.selection = 0;
	}
}

impl ReviewTab {
	fn draw_preview_lines(
		&self,
		comment: &crate::review::ReviewComment,
	) -> Vec<Line<'_>> {
		comment.diff_context.as_ref().map_or_else(
			|| {
				vec![Line::from(Span::styled(
					String::from(
						"(file comment \u{2014} no diff context)",
					),
					self.theme.text(false, false),
				))]
			},
			|ctx| {
				let mut in_block = false;
				ctx.lines()
					.filter(|l| {
						if l.starts_with("```diff") {
							in_block = true;
							return false;
						}
						if in_block && l.starts_with("```") {
							in_block = false;
							return false;
						}
						in_block
					})
					.map(|l| {
						let style = if l.starts_with('+')
							&& !l.starts_with("+++")
						{
							self.theme.diff_line(
								asyncgit::DiffLineType::Add,
								false,
							)
						} else if l.starts_with('-')
							&& !l.starts_with("---")
						{
							self.theme.diff_line(
								asyncgit::DiffLineType::Delete,
								false,
							)
						} else if l.starts_with("@@") {
							self.theme.diff_line(
								asyncgit::DiffLineType::Header,
								false,
							)
						} else {
							self.theme.text(true, false)
						};
						Line::from(Span::styled(
							l.to_string(),
							style,
						))
					})
					.collect()
			},
		)
	}
}

impl DrawableComponent for ReviewTab {
	fn draw(&self, f: &mut Frame, r: Rect) -> Result<()> {
		let store = self.review_store.borrow();
		let count = store.len();

		let chunks = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([
				Constraint::Percentage(40),
				Constraint::Percentage(60),
			])
			.split(r);

		// Left pane: comment list with aligned columns
		let max_path_len = store
			.comments()
			.iter()
			.map(|c| c.path.len())
			.max()
			.unwrap_or(0);

		let items: Vec<ListItem> = store
			.comments()
			.iter()
			.enumerate()
			.map(|(i, comment)| {
				let selected = i == self.selection && self.focused();
				let style = self.theme.text(true, selected);
				let kind = if comment.diff_context.is_some() {
					"diff"
				} else {
					"file"
				};
				let first_line =
					comment.comment.lines().next().unwrap_or("");
				let label = format!(
					"{kind} @{path:<width$}  {first_line}",
					path = comment.path,
					width = max_path_len,
				);
				ListItem::new(Line::from(vec![Span::styled(
					Cow::from(label),
					style,
				)]))
			})
			.collect();

		let title = format!("Review Comments ({count})");

		f.render_widget(
			List::new(items).block(
				Block::default()
					.title(Span::styled(
						title,
						self.theme.title(self.focused()),
					))
					.borders(Borders::ALL)
					.border_style(self.theme.block(self.focused())),
			),
			chunks[0],
		);

		// Right pane: diff preview (only for diff comments)
		let preview_lines: Vec<Line> =
			if count > 0 && self.selection < count {
				let comment = &store.comments()[self.selection];
				self.draw_preview_lines(comment)
			} else {
				vec![Line::from(Span::styled(
					String::from("No comment selected"),
					self.theme.text(false, false),
				))]
			};

		f.render_widget(
			Paragraph::new(preview_lines)
				.block(
					Block::default()
						.title(Span::styled(
							"Diff Preview",
							self.theme.title(self.focused()),
						))
						.borders(Borders::ALL)
						.border_style(
							self.theme.block(self.focused()),
						),
				)
				.wrap(Wrap { trim: false }),
			chunks[1],
		);

		Ok(())
	}
}

impl Component for ReviewTab {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		let has_comments = !self.review_store.borrow().is_empty();

		out.push(CommandInfo::new(
			strings::commands::review_copy_all(&self.key_config),
			has_comments,
			self.focused(),
		));
		out.push(CommandInfo::new(
			strings::commands::review_delete(&self.key_config),
			has_comments,
			self.focused(),
		));
		out.push(CommandInfo::new(
			strings::commands::review_edit(&self.key_config),
			has_comments,
			self.focused(),
		));
		out.push(CommandInfo::new(
			strings::commands::review_clear(&self.key_config),
			has_comments,
			self.focused(),
		));

		CommandBlocking::PassingOn
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.focused() {
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.move_down) {
					self.move_selection(false);
					return Ok(EventState::Consumed);
				} else if key_match(e, self.key_config.keys.move_up) {
					self.move_selection(true);
					return Ok(EventState::Consumed);
				} else if key_match(e, self.key_config.keys.copy) {
					self.copy_all();
					return Ok(EventState::Consumed);
				} else if key_match(
					e,
					self.key_config.keys.review_delete,
				) {
					self.delete_selected();
					return Ok(EventState::Consumed);
				} else if key_match(
					e,
					self.key_config.keys.review_edit,
				) {
					let count = self.review_store.borrow().len();
					if count > 0 && self.selection < count {
						self.queue.push(
							InternalEvent::EditReviewComment(
								self.selection,
							),
						);
					}
					return Ok(EventState::Consumed);
				} else if key_match(
					e,
					self.key_config.keys.review_clear,
				) {
					self.clear_all();
					return Ok(EventState::Consumed);
				}
			}
		}
		Ok(EventState::NotConsumed)
	}

	fn focused(&self) -> bool {
		self.visible
	}

	fn focus(&mut self, focus: bool) {
		self.visible = focus;
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;
		Ok(())
	}
}
