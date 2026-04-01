use std::{cell::RefCell, rc::Rc};

pub type SharedReviewStore = Rc<RefCell<ReviewStore>>;

pub struct ReviewComment {
	pub path: String,
	pub diff_context: Option<String>,
	pub comment: String,
}

impl ReviewComment {
	pub fn format(&self) -> String {
		self.diff_context.as_ref().map_or_else(
			|| format!("@{} {}", self.path, self.comment),
			|ctx| format!("{ctx}\n\n{}", self.comment),
		)
	}
}

#[derive(Default)]
pub struct ReviewStore {
	comments: Vec<ReviewComment>,
}

impl ReviewStore {
	pub fn add(&mut self, comment: ReviewComment) {
		self.comments.push(comment);
	}

	pub fn remove(&mut self, index: usize) {
		if index < self.comments.len() {
			self.comments.remove(index);
		}
	}

	pub fn update_comment(&mut self, index: usize, new_comment: String) {
		if let Some(c) = self.comments.get_mut(index) {
			c.comment = new_comment;
		}
	}

	pub fn clear(&mut self) {
		self.comments.clear();
	}

	pub fn comments(&self) -> &[ReviewComment] {
		&self.comments
	}

	pub fn get(&self, index: usize) -> Option<&ReviewComment> {
		self.comments.get(index)
	}

	pub const fn len(&self) -> usize {
		self.comments.len()
	}

	pub const fn is_empty(&self) -> bool {
		self.comments.is_empty()
	}

	pub fn format_all(&self) -> String {
		self.comments
			.iter()
			.map(ReviewComment::format)
			.collect::<Vec<_>>()
			.join("\n\n---\n\n")
	}
}
