//! Embedded assets (SVG icons) served to GPUI's `svg().path(..)`.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub(crate) struct Assets;

const FILES: &[(&str, &[u8])] = &[
    ("icons/home.svg", include_bytes!("../assets/icons/home.svg")),
    ("icons/book.svg", include_bytes!("../assets/icons/book.svg")),
    ("icons/keyboard.svg", include_bytes!("../assets/icons/keyboard.svg")),
    ("icons/settings.svg", include_bytes!("../assets/icons/settings.svg")),
    ("icons/copy.svg", include_bytes!("../assets/icons/copy.svg")),
    ("icons/check.svg", include_bytes!("../assets/icons/check.svg")),
    ("icons/bolt.svg", include_bytes!("../assets/icons/bolt.svg")),
    ("icons/text.svg", include_bytes!("../assets/icons/text.svg")),
    ("icons/flame.svg", include_bytes!("../assets/icons/flame.svg")),
    ("icons/search.svg", include_bytes!("../assets/icons/search.svg")),
    ("icons/plus.svg", include_bytes!("../assets/icons/plus.svg")),
    ("icons/x.svg", include_bytes!("../assets/icons/x.svg")),
    ("icons/shortcut-arrow.svg", include_bytes!("../assets/icons/shortcut-arrow.svg")),
    ("logo/icon.svg", include_bytes!("../assets/logo/icon-dark.svg")),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}
