//! A keyboard chord: modifiers plus an optional non-modifier key.
//! Stored as `Ctrl + Space` in settings; chips render `labels()`.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    /// Canonical name (`space`, `a`, `f1`). `None` = modifiers only.
    pub key: Option<String>,
}

pub(crate) const DEFAULT_HOLD: &str = "Ctrl + Space";
pub(crate) const DEFAULT_TOGGLE: &str = "Ctrl + Win + Alt";

impl Chord {
    pub(crate) fn hold_default() -> Self {
        Self::parse(DEFAULT_HOLD).expect("default hold")
    }

    pub(crate) fn toggle_default() -> Self {
        Self::parse(DEFAULT_TOGGLE).expect("default toggle")
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let mut chord = Chord::default();
        for raw in s.split('+') {
            let token = raw.trim();
            if token.is_empty() {
                continue;
            }
            apply_token(&mut chord, token)?;
        }
        if chord.is_empty() { None } else { Some(chord) }
    }

    pub(crate) fn parse_or_default(s: &str, fallback: Self) -> Self {
        Self::parse(s).filter(|c| !c.is_empty()).unwrap_or(fallback)
    }

    pub(crate) fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.win && self.key.is_none()
    }

    /// Last key-down while recording: modifiers from the event, key if it isn't a modifier.
    pub(crate) fn from_keystroke(ctrl: bool, alt: bool, shift: bool, win: bool, key: &str) -> Self {
        let mut chord = Self { ctrl, alt, shift, win, key: None };
        if let Some(name) = canonical_key(key) {
            if !is_modifier_name(&name) {
                chord.key = Some(name);
            }
        }
        chord
    }

    pub(crate) fn labels(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.ctrl {
            out.push("Ctrl".into());
        }
        if self.win {
            out.push("Win".into());
        }
        if self.alt {
            out.push("Alt".into());
        }
        if self.shift {
            out.push("Shift".into());
        }
        if let Some(key) = &self.key {
            out.push(display_key(key));
        }
        out
    }

    pub(crate) fn display(&self) -> String {
        self.labels().join(" + ")
    }
}

fn apply_token(chord: &mut Chord, token: &str) -> Option<()> {
    let name = canonical_key(token)?;
    if is_modifier_name(&name) {
        match name.as_str() {
            "ctrl" | "control" => chord.ctrl = true,
            "alt" | "option" => chord.alt = true,
            "shift" => chord.shift = true,
            "win" | "super" | "meta" | "cmd" | "platform" => chord.win = true,
            _ => {}
        }
        Some(())
    } else if chord.key.is_none() {
        chord.key = Some(name);
        Some(())
    } else {
        None
    }
}

fn is_modifier_name(name: &str) -> bool {
    matches!(
        name,
        "ctrl" | "control" | "alt" | "option" | "shift" | "win" | "windows" | "super" | "meta" | "cmd" | "platform"
    )
}

fn canonical_key(token: &str) -> Option<String> {
    let t = token.trim().to_ascii_lowercase();
    let t = t.strip_prefix("key").unwrap_or(&t);
    let name = match t {
        "control" | "ctrl" => "ctrl",
        "alt" | "option" | "opt" => "alt",
        "shift" => "shift",
        "win" | "windows" | "super" | "meta" | "cmd" | "command" | "platform" => "win",
        " " | "space" | "spacebar" => "space",
        "esc" | "escape" => "escape",
        "return" | "enter" => "enter",
        "backspace" | "bksp" => "backspace",
        "del" | "delete" => "delete",
        "pgup" | "pageup" | "page-up" => "pageup",
        "pgdn" | "pagedown" | "page-down" => "pagedown",
        "ins" | "insert" => "insert",
        "caps" | "capslock" | "caps-lock" => "capslock",
        "minus" | "-" => "minus",
        "equal" | "=" | "equals" => "equal",
        "comma" | "," => "comma",
        "period" | "." | "dot" => "period",
        "slash" | "/" => "slash",
        "backslash" | "\\" => "backslash",
        "semicolon" | ";" => "semicolon",
        "quote" | "'" => "quote",
        "backquote" | "`" | "grave" => "backquote",
        "left" | "arrowleft" => "left",
        "right" | "arrowright" => "right",
        "up" | "arrowup" => "up",
        "down" | "arrowdown" => "down",
        "tab" => "tab",
        "home" => "home",
        "end" => "end",
        other if other.len() == 1 => other,
        other if other.starts_with('f') && other[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)) => other,
        other if !other.is_empty() => other,
        _ => return None,
    };
    Some(name.to_string())
}

fn display_key(key: &str) -> String {
    match key {
        "space" => "Space".into(),
        "escape" => "Esc".into(),
        "enter" => "Enter".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Del".into(),
        "pageup" => "PgUp".into(),
        "pagedown" => "PgDn".into(),
        "capslock" => "Caps".into(),
        "insert" => "Ins".into(),
        "minus" => "-".into(),
        "equal" => "=".into(),
        "comma" => ",".into(),
        "period" => ".".into(),
        "slash" => "/".into(),
        "backslash" => "\\".into(),
        "left" => "←".into(),
        "right" => "→".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        other if other.len() == 1 => other.to_ascii_uppercase(),
        other if other.starts_with('f') && other[1..].chars().all(|c| c.is_ascii_digit()) => other.to_ascii_uppercase(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hold_default() {
        let c = Chord::parse("Ctrl + Space").unwrap();
        assert!(c.ctrl && !c.alt && !c.shift && !c.win);
        assert_eq!(c.key.as_deref(), Some("space"));
        assert_eq!(c.labels(), vec!["Ctrl", "Space"]);
        assert_eq!(c.display(), "Ctrl + Space");
    }

    #[test]
    fn parse_modifier_only_toggle() {
        let c = Chord::parse("Ctrl+Win+Alt").unwrap();
        assert!(c.ctrl && c.alt && c.win && !c.shift);
        assert_eq!(c.key, None);
        assert_eq!(c.labels(), vec!["Ctrl", "Win", "Alt"]);
    }

    #[test]
    fn from_keystroke_builds_the_held_combo() {
        let empty = Chord::from_keystroke(true, false, false, false, "control");
        assert!(empty.ctrl && empty.key.is_none());
        let hold = Chord::from_keystroke(true, false, false, false, "space");
        assert_eq!(hold.display(), "Ctrl + Space");
    }

    #[test]
    fn empty_and_junk_do_not_parse() {
        assert_eq!(Chord::parse(""), None);
        assert_eq!(Chord::parse("   "), None);
        assert_eq!(Chord::parse("Ctrl + Space + A"), None);
    }

    #[test]
    fn parse_or_default_falls_back() {
        assert_eq!(
            Chord::parse_or_default("", Chord::hold_default()).display(),
            "Ctrl + Space"
        );
    }
}
