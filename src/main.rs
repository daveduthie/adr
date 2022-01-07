#![feature(iter_intersperse)]

use comfy_table::presets::ASCII_MARKDOWN;
use comfy_table::Table;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

static ADR_EXTENSIONS: [&str; 2] = ["md", "org"];
static ADR_NAME_PREFIX: &str = "adr-";

// Copied from the docs
fn visit_dirs(dir: &Path, cb: &mut dyn for<'r> FnMut(&'r DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
enum Action {
    Default,
    Trial,
    Retire,
    CelebrateRetirement,
}

impl From<&str> for Action {
    fn from(tag: &str) -> Self {
        match tag {
            "default" => Action::Default,
            "trial" => Action::Trial,
            "retire" => Action::Retire,
            "celebrate" => Action::CelebrateRetirement,
            _ => panic!("Unknown action: {}", tag),
        }
    }
}

#[derive(Debug)]
struct Event {
    action: Action,
    tech: String,
    category: String,
    stack: String,
}

type Events = Vec<Event>;

#[derive(Debug)]
struct Adr {
    id: usize,
    events: Events,
}

fn parse_events(path: &Path) -> io::Result<Events> {
    lazy_static! {
        static ref EVENT_REGEX: Regex =
            Regex::new(r"(?P<stack>.*) (?P<cat>.*) (?P<action>default|trial|retire): (?P<tech>.*)")
                .unwrap();
    }
    let events = EVENT_REGEX
        .captures_iter(&fs::read_to_string(path)?)
        .map(|capture| Event {
            action: Action::from(&capture["action"]),
            stack: capture["stack"].to_string(),
            category: capture["cat"].to_string(),
            tech: capture["tech"].to_string(),
        })
        .collect();

    Ok(events)
}

fn collect_adrs() -> Vec<Adr> {
    let mut results = Vec::new();

    visit_dirs(&PathBuf::from("./"), &mut |entry| {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            if ADR_EXTENSIONS.contains(&ext) {
                let adr_no = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| (stem.strip_prefix(ADR_NAME_PREFIX)))
                    .map(|stem| usize::from_str(stem).expect("ADR number to be a small integer"));

                if let Some(id) = adr_no {
                    let events = parse_events(&path).expect("Read path to succeed");
                    results.push(Adr { id, events });
                }
            }
        }
    })
    .expect("Visit dir not to fail");

    results.sort_by_key(|adr| adr.id);
    results
}

#[derive(Default, Debug)]
struct TechCategory {
    default: BTreeSet<String>,
    trial: BTreeSet<String>,
    retire: BTreeSet<String>,
}

#[derive(Default)]
struct Stack(BTreeMap<String, TechCategory>);

#[derive(Default)]
struct Stacks(BTreeMap<String, Stack>);

impl Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hash_set_to_string = |hs: &BTreeSet<String>| -> String {
            let mut ret = String::new();
            let separator = String::from(", ");
            for s in hs.iter().intersperse(&separator) {
                ret += s;
            }
            ret
        };
        let mut table = Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_header(vec!["Tech", "Default", "Trial", "Retire"]);
        for (name, category) in self.0.iter() {
            table.add_row(vec![
                name,
                &hash_set_to_string(&category.default),
                &hash_set_to_string(&category.trial),
                &hash_set_to_string(&category.retire),
            ]);
        }

        f.write_fmt(format_args!("{}", table))
    }
}

impl Display for Stacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, stack) in self.0.iter() {
            f.write_fmt(format_args!("## {}\n\n", name))?;
            f.write_fmt(format_args!("{}\n\n", stack))?;
        }

        Ok(())
    }
}

fn main() {
    let results = collect_adrs();
    let mut stacks: Stacks = Default::default();
    for adr in results {
        for event in adr.events {
            let stack = stacks.0.entry(event.stack).or_default();
            let category = stack.0.entry(event.category).or_default();
            category.default.remove(&event.tech);
            category.trial.remove(&event.tech);
            category.retire.remove(&event.tech);
            match event.action {
                Action::Default => category.default.insert(event.tech),
                Action::Trial => category.trial.insert(event.tech),
                Action::Retire => category.retire.insert(event.tech),
                Action::CelebrateRetirement => category.retire.remove(&event.tech),
            };
        }
    }

    println!("{}", stacks);
}
