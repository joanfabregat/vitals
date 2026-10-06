//! Turns readings into one status line, as tmux style markup or plain text.

use crate::cli::{Colors, Style};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Readings {
    pub cpu: Option<u8>,
    pub memory: Option<u8>,
    pub swap: Option<u8>,
    pub disks: Vec<u8>,
    /// Date and time, already formatted.
    pub clock: Option<(String, String)>,
}

/// A vertical bar in eighths. Anything under 19% still draws the lowest
/// eighth, so an idle metric keeps a visible sliver.
pub fn bar_glyph(percent: u8) -> char {
    match (u32::from(percent) * 8 + 50) / 100 {
        0 | 1 => '▁',
        2 => '▂',
        3 => '▃',
        4 => '▄',
        5 => '▅',
        6 => '▆',
        7 => '▇',
        _ => '█',
    }
}

/// A pie in quarters.
pub fn pie_glyph(percent: u8) -> char {
    match percent {
        88.. => '●',
        63.. => '◕',
        38.. => '◑',
        13.. => '◔',
        _ => '○',
    }
}

pub struct Renderer<'a> {
    pub style: Style,
    pub colors: &'a Colors,
    pub alert: u8,
}

impl Renderer<'_> {
    fn fg(&self, color: &str) -> String {
        match self.style {
            Style::Tmux => format!("#[fg={color}]"),
            Style::Plain => String::new(),
        }
    }

    fn metric(&self, label: &str, color: &str, percent: u8) -> String {
        let glyph = bar_glyph(percent);
        match self.style {
            Style::Tmux => format!(
                "#[fg={color}]{label} #[fg={color},bg={track}]{glyph}#[bg=default] {percent}%",
                track = self.colors.track
            ),
            Style::Plain => format!("{label} {glyph} {percent}%"),
        }
    }

    pub fn render(&self, r: &Readings) -> String {
        let c = self.colors;
        let mut parts = Vec::new();
        if let Some(p) = r.cpu {
            parts.push(self.metric("cpu", &c.cpu, p));
        }
        if let Some(p) = r.memory {
            parts.push(self.metric("mem", &c.mem, p));
        }
        if let Some(p) = r.swap {
            parts.push(self.metric("swap", &c.swap, p));
        }
        if !r.disks.is_empty() {
            parts.push(format!("{}disk", self.fg(&c.disk)));
            for &p in &r.disks {
                let color = if p >= self.alert { &c.alert } else { &c.disk };
                parts.push(format!("{}{} {p}%", self.fg(color), pie_glyph(p)));
            }
        }
        if let Some((date, time)) = &r.clock {
            parts.push(format!("{}·", self.fg(&c.separator)));
            parts.push(format!("{}{date}", self.fg(&c.date)));
            parts.push(format!("{}{time}", self.fg(&c.time)));
        }
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readings() -> Readings {
        Readings {
            cpu: Some(26),
            memory: Some(7),
            swap: Some(0),
            disks: vec![65, 93],
            clock: Some(("2026-10-05".into(), "21:01".into())),
        }
    }

    #[test]
    fn bar_eighths() {
        assert_eq!(bar_glyph(0), '▁');
        assert_eq!(bar_glyph(18), '▁');
        assert_eq!(bar_glyph(19), '▂');
        assert_eq!(bar_glyph(50), '▄');
        assert_eq!(bar_glyph(93), '▇');
        assert_eq!(bar_glyph(94), '█');
        assert_eq!(bar_glyph(100), '█');
    }

    #[test]
    fn pie_quarters() {
        assert_eq!(pie_glyph(12), '○');
        assert_eq!(pie_glyph(13), '◔');
        assert_eq!(pie_glyph(38), '◑');
        assert_eq!(pie_glyph(63), '◕');
        assert_eq!(pie_glyph(88), '●');
    }

    #[test]
    fn tmux_line() {
        let colors = Colors::default();
        let r = Renderer {
            style: Style::Tmux,
            colors: &colors,
            alert: 90,
        };
        assert_eq!(
            r.render(&readings()),
            "#[fg=colour114]cpu #[fg=colour114,bg=colour238]▂#[bg=default] 26% \
             #[fg=colour141]mem #[fg=colour141,bg=colour238]▁#[bg=default] 7% \
             #[fg=colour73]swap #[fg=colour73,bg=colour238]▁#[bg=default] 0% \
             #[fg=colour222]disk #[fg=colour222]◕ 65% #[fg=colour196]● 93% \
             #[fg=colour244]· #[fg=colour39]2026-10-05 #[fg=colour250]21:01"
        );
    }

    #[test]
    fn plain_line() {
        let colors = Colors::default();
        let r = Renderer {
            style: Style::Plain,
            colors: &colors,
            alert: 90,
        };
        assert_eq!(
            r.render(&readings()),
            "cpu ▂ 26% mem ▁ 7% swap ▁ 0% disk ◕ 65% ● 93% · 2026-10-05 21:01"
        );
    }

    #[test]
    fn missing_readings_are_left_out() {
        let colors = Colors::default();
        let r = Renderer {
            style: Style::Plain,
            colors: &colors,
            alert: 90,
        };
        let only_cpu = Readings {
            cpu: Some(5),
            ..Readings::default()
        };
        assert_eq!(r.render(&only_cpu), "cpu ▁ 5%");
        assert_eq!(r.render(&Readings::default()), "");
    }
}
