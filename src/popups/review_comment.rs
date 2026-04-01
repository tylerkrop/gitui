use crate::components::{
	visibility_blocking, CommandBlocking, CommandInfo, Component,
	DrawableComponent, EventState, InputType, TextInputComponent,
};
use crate::{
	app::Environment,
	keys::{key_match, SharedKeyConfig},
	review::{ReviewComment, SharedReviewStore},
};
use anyhow::Result;
use crossterm::event::Event;
use ratatui::{layout::Rect, Frame};

pub struct ReviewCommentPopup {
	input: TextInputComponent,
	review_store: SharedReviewStore,
	key_config: SharedKeyConfig,
	path: String,
	diff_context: Option<String>,
	edit_index: Option<usize>,
}

impl ReviewCommentPopup {
	pub fn new(
		env: &Environment,
		review_store: SharedReviewStore,
	) -> Self {
		Self {
			input: TextInputComponent::new(
				env,
				"Review Comment",
				"enter comment",
				false,
			)
			.with_input_type(InputType::Singleline),
			review_store,
			key_config: env.key_config.clone(),
			path: String::new(),
			diff_context: None,
			edit_index: None,
		}
	}

	pub fn open(
		&mut self,
		path: String,
		diff_context: Option<String>,
	) -> Result<()> {
		self.path = path;
		self.diff_context = diff_context;
		self.edit_index = None;
		self.input.clear();
		self.input.show()?;
		Ok(())
	}

	pub fn open_edit(&mut self, index: usize) -> Result<()> {
		let store = self.review_store.borrow();
		if let Some(comment) = store.get(index) {
			let path = comment.path.clone();
			let diff_context = comment.diff_context.clone();
			let text = comment.comment.clone();
			drop(store);
			self.path = path;
			self.diff_context = diff_context;
			self.edit_index = Some(index);
			self.input.clear();
			self.input.set_text(text);
			self.input.show()?;
		}
		Ok(())
	}

	fn submit(&self) {
		let text = self.input.get_text().to_string();
		if text.is_empty() {
			return;
		}

		let mut store = self.review_store.borrow_mut();
		if let Some(idx) = self.edit_index {
			store.update_comment(idx, text);
		} else {
			store.add(ReviewComment {
				path: self.path.clone(),
				diff_context: self.diff_context.clone(),
				comment: text,
			});
		}
	}
}

impl DrawableComponent for ReviewCommentPopup {
	fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()> {
		if self.is_visible() {
			self.input.draw(f, rect)?;
		}
		Ok(())
	}
}

impl Component for ReviewCommentPopup {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);
		}
		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.is_visible() {
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.exit_popup) {
					self.hide();
					return Ok(EventState::Consumed);
				}

				if key_match(e, self.key_config.keys.enter) {
					self.submit();
					self.hide();
					return Ok(EventState::Consumed);
				}
			}

			// Forward to text input, but consume all key events
			// regardless to prevent background handlers (like
			// help popup) from intercepting keystrokes.
			self.input.event(ev)?;

			if matches!(ev, Event::Key(_)) {
				return Ok(EventState::Consumed);
			}
		}
		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.input.is_visible()
	}

	fn hide(&mut self) {
		self.input.hide();
	}

	fn show(&mut self) -> Result<()> {
		self.input.show()
	}

	fn focused(&self) -> bool {
		self.is_visible()
	}

	fn focus(&mut self, _focus: bool) {}
}
