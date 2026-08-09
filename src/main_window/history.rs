use gpui::{div, prelude::*};

use crate::main_window::MainWindow;
use crate::widgets::history_list;

impl MainWindow {
    pub(super) fn render_history(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .child(history_list(self.history.clone()))
    }
}
