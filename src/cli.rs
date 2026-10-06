//! Command-line parsing, by hand to keep the binary free of dependencies.

use std::path::PathBuf;

pub const HELP: &str = "\
vitals: print CPU, memory, swap and disk usage plus the local time as one line

Usage: vitals [OPTIONS]

Options:
      --plain             Plain text instead of tmux style markup
      --no-clock          Leave out the date and time
      --alert PERCENT     Show disks at or above PERCENT in the alert colour [default: 90]
      --min-disk-gib GIB  Skip filesystems smaller than GIB GiB [default: 10]
      --color KEY=COLOR   Set a tmux colour; repeatable. Keys: cpu, mem, swap, disk,
                          track, alert, separator, date, time
      --state PATH        CPU sample file [default: $XDG_RUNTIME_DIR/vitals-cpu]
  -h, --help              Print this help
  -V, --version           Print the version

The time zone comes from TZ, then /etc/localtime.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Tmux,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colors {
    pub cpu: String,
    pub mem: String,
    pub swap: String,
    pub disk: String,
    pub track: String,
    pub alert: String,
    pub separator: String,
    pub date: String,
    pub time: String,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            cpu: "colour114".into(),
            mem: "colour141".into(),
            swap: "colour73".into(),
            disk: "colour222".into(),
            track: "colour238".into(),
            alert: "colour196".into(),
            separator: "colour244".into(),
            date: "colour39".into(),
            time: "colour250".into(),
        }
    }
}

impl Colors {
    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        // tmux colours are names (red, colour114, default) or #rrggbb; anything
        // else could break out of the #[...] style block.
        if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '#') {
            return Err(format!("invalid colour {value:?}"));
        }
        let slot = match key {
            "cpu" => &mut self.cpu,
            "mem" => &mut self.mem,
            "swap" => &mut self.swap,
            "disk" => &mut self.disk,
            "track" => &mut self.track,
            "alert" => &mut self.alert,
            "separator" => &mut self.separator,
            "date" => &mut self.date,
            "time" => &mut self.time,
            _ => return Err(format!("unknown colour key {key:?}")),
        };
        *slot = value.to_string();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub style: Style,
    pub clock: bool,
    pub alert: u8,
    pub min_disk_gib: u64,
    pub colors: Colors,
    pub state: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            style: Style::Tmux,
            clock: true,
            alert: 90,
            min_disk_gib: 10,
            colors: Colors::default(),
            state: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Run(Box<Options>),
    Help,
    Version,
}

pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_string(), Some(value.to_string()))
            }
            _ => (arg, None),
        };
        let mut value = |name: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--plain" => options.style = Style::Plain,
            "--no-clock" => options.clock = false,
            "--alert" => {
                let v = value("--alert")?;
                options.alert = v
                    .parse()
                    .ok()
                    .filter(|p| *p <= 100)
                    .ok_or_else(|| format!("--alert: invalid percentage {v:?}"))?;
            }
            "--min-disk-gib" => {
                let v = value("--min-disk-gib")?;
                options.min_disk_gib = v
                    .parse()
                    .map_err(|_| format!("--min-disk-gib: invalid size {v:?}"))?;
            }
            "--color" | "--colour" => {
                let v = value("--color")?;
                let (key, color) = v
                    .split_once('=')
                    .ok_or_else(|| format!("--color: expected KEY=COLOR, got {v:?}"))?;
                options.colors.set(key, color)?;
            }
            "--state" => options.state = Some(PathBuf::from(value("--state")?)),
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    Ok(Command::Run(Box::new(options)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Options, String> {
        match parse(args.iter().map(|s| s.to_string()))? {
            Command::Run(o) => Ok(*o),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn defaults() {
        assert_eq!(run(&[]).unwrap(), Options::default());
    }

    #[test]
    fn parses_every_option() {
        let o = run(&[
            "--plain",
            "--no-clock",
            "--alert",
            "80",
            "--min-disk-gib=2",
            "--color",
            "cpu=red",
            "--colour=time=#ff8800",
            "--state",
            "/tmp/x",
        ])
        .unwrap();
        assert_eq!(o.style, Style::Plain);
        assert!(!o.clock);
        assert_eq!(o.alert, 80);
        assert_eq!(o.min_disk_gib, 2);
        assert_eq!(o.colors.cpu, "red");
        assert_eq!(o.colors.time, "#ff8800");
        assert_eq!(o.state, Some(PathBuf::from("/tmp/x")));
    }

    #[test]
    fn help_and_version() {
        assert_eq!(parse(["-h".to_string()]), Ok(Command::Help));
        assert_eq!(parse(["--version".to_string()]), Ok(Command::Version));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(run(&["--bogus"]).is_err());
        assert!(run(&["--alert"]).is_err());
        assert!(run(&["--alert", "101"]).is_err());
        assert!(run(&["--color", "cpu"]).is_err());
        assert!(run(&["--color", "nope=red"]).is_err());
        assert!(run(&["--color", "cpu=red]#[bg=blue"]).is_err());
    }
}
