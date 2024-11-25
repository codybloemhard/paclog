use std::{
    collections::{ HashMap, HashSet },
    num::ParseIntError,
    hash::Hash,
    fmt::{ self, Display, Write },
};

use clap::{
    Parser,
    Subcommand,
};

use zen_colour::*;
use vec_string::*;
use simpleio::read_lines;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args{
    #[clap(subcommand)]
    command: Commands,
    #[clap(short = 'l', default_value = "/var/log/pacman.log", help = "Path to logfile.")]
    path: String,
}

#[derive(Subcommand, Debug)]
enum Commands{
    #[clap(short_flag = 's', about = "Print some statistics.")]
    Summary,
    #[allow(clippy::enum_variant_names)]
    #[clap(short_flag = 'c', about = "List most run commands.")]
    Commands{
        #[clap(short, default_value_t = 16, help = "Amount of commands to show.")]
        n: usize,
        #[clap(short, default_value_t = false, help = "Show all.")]
        a: bool,
    },
    #[clap(short_flag = 'i', about = "List most installed packages.")]
    Installs{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
        #[clap(short, default_value_t = false, help = "Show all.")]
        a: bool,
    },
    #[clap(short_flag = 'r', about = "List most removed packages.")]
    Removes{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
        #[clap(short, default_value_t = false, help = "Show all.")]
        a: bool,
    },
    #[clap(short_flag = 'u', about = "List most upgraded packages.")]
    Upgrades{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
        #[clap(short, default_value_t = false, help = "Show all.")]
        a: bool,
    },
    #[clap(short_flag = 'd', about = "List most downgraded packages.")]
    Downgrades{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
        #[clap(short, default_value_t = false, help = "Show all.")]
        a: bool,
    },
    #[clap(short_flag = 'p', about = "List package history.")]
    Package{
        package: String,
        #[clap(long, help = "Show command used to do upgrades.")]
        upgrade_command: bool,
    },
    #[clap(short_flag = 'H', about = "List pacman history.")]
    History{
        #[clap(short, default_value_t = 32, help = "Amount of items to show.")]
        n: usize,
        #[clap(short = 'f', help = "List out every event with version.")]
        full: bool,
        #[clap(short = 'u', help = "Ignore updates in full mode.")]
        no_upgrades: bool,
        #[clap(short = 'c', help = "Focus on package count.")]
        count: bool,
    },
    #[clap(
        short_flag = 'I',
        about = "List currently intentionally installed packages. Bold if never removed."
    )]
    Intentional{
        #[clap(short = 'l', help = "List one package per line.")]
        list: bool,
    },
    #[clap(short_flag = 't', about = "Print some statistics regarding time and dates.")]
    Time{
        #[clap(short = 'a', help = "Print stats for all categories.")]
        all: bool,
        #[clap(short = 'y', help = "Print stats per year.")]
        year: bool,
        #[clap(short = 'm', help = "Print stats per month.")]
        month: bool,
        #[clap(short = 'd', help = "Print stats per day.")]
        day: bool,
        #[clap(short = 'H', help = "Print stats per hour.")]
        hour: bool,
    },
}

fn main() {
    let args = Args::parse();
    let lines = read_lines(&args.path);
    if lines.is_empty() {
        panic!("Error: could not read '{}'!", args.path);
    };
    let parsed = parse(lines);

    match args.command{
        Commands::Summary => {
            summary(parsed);
        },
        Commands::Commands{ n, a } => {
            top_commands(parsed, n, a);
        },
        Commands::Installs{ n, a } => {
            top_installs(parsed, n, a);
        },
        Commands::Removes{ n, a } => {
            top_removes(parsed, n, a);
        },
        Commands::Upgrades{ n, a } => {
            top_upgrades(parsed, n, a);
        },
        Commands::Downgrades{ n, a } => {
            top_downgrades(parsed, n, a);
        },
        Commands::Package{ package, upgrade_command } => {
            package_history(parsed, package, upgrade_command);
        },
        Commands::History{ n, full, no_upgrades, count } => {
            if full {
                history_full(parsed, n, no_upgrades);
            } else if let Err(e) = history_compact(parsed, n, count){
                println!("{:?}", e);
            }
        },
        Commands::Intentional { list }=> {
            intentional(parsed, list);
        },
        Commands::Time { all, year, month, day, hour } => {
            time(parsed, all, year, month, day, hour);
        },
    }
}

type DT = (u16, u8, u8, u8); // date time (y, m, d, h)
enum Event{
    Command(DT, String), // dt, command
    Installed(DT, String, String), // dt, package, version
    Removed(DT, String, String), // dt, package, version
    Upgraded(DT, String, String), // dt, package, version
    Downgraded(DT, String, String), // dt, package, version
}
type Events = Vec<Event>;

fn parse(lines: Vec<String>) -> Vec<Event>{
    let mut res = Vec::new();

    for line in lines {
        let parts = line.split(' ').collect::<Vec<_>>();
        if parts.len() < 4 { continue; }

        let dt = if let Ok(dt) = parse_dt(parts[0]) { dt } else { continue; };

        if parts[1] == "[PACMAN]" && parts[2] == "Running"{
            let mut command = parts[3..].join(" ");
            command.retain(|c| c != '\'');
            res.push(Event::Command(dt, command));
        } else if parts[1] == "[ALPM]" && parts[2] == "installed"{
            let version = parts[4..].join(" ");
            res.push(Event::Installed(dt, parts[3].to_string(), version));
        } else if parts[1] == "[ALPM]" && parts[2] == "removed"{
            let version = parts[4..].join(" ");
            res.push(Event::Removed(dt, parts[3].to_string(), version));
        } else if parts[1] == "[ALPM]" && parts[2] == "upgraded"{
            let version = parts[4..].join(" ");
            res.push(Event::Upgraded(dt, parts[3].to_string(), version));
        } else if parts[1] == "[ALPM]" && parts[2] == "downgraded"{
            let version = parts[4..].join(" ");
            res.push(Event::Downgraded(dt, parts[3].to_string(), version));
        }
    }

    res
}

fn summary(events: Events){
    let nevents = events.len();
    let mut packages = 0usize;
    let mut updates = 0usize;
    let mut installs = 0usize;
    let mut removes = 0usize;
    let mut upgrades = 0usize;
    let mut downgrades = 0usize;
    let mut last_command_update = false;
    let mut y_map = FreqMap::new();

    if let Some(Event::Command(dt, command)) = events.first() {
        println!(
            "First command ran on {}:\n\t\"{}{}{}\"",
            format_dt(*dt), MAGENTA, command, RESET
        );
    }

    for event in events {
        match event{
            Event::Command((y, _, _, _), _) => {
                last_command_update = false;
                y_map.inc(y);
            },
            Event::Installed(_, _, _) => {
                packages += 1;
                installs += 1;
            },
            Event::Removed(_, _, _) => {
                packages -= 1;
                removes += 1;
            },
            Event::Upgraded(_, _, _) => {
                if !last_command_update{
                    last_command_update = true;
                    updates += 1;
                }
                upgrades += 1;
            },
            Event::Downgraded(_, _, _) => {
                downgrades += 1;
            },
        }
    }

    println!("Packages installed: {}{}{}\n", RED, packages, RESET);
    println!("Events: {}{}{}", RED, nevents, RESET);
    println!("Updates: {}{}{}", RED, updates, RESET);
    println!("Installs: {}{}{}", RED, installs, RESET);
    println!("Removes: {}{}{}", RED, removes, RESET);
    println!("Upgrades: {}{}{}", RED, upgrades, RESET);
    println!("Downgrades: {}{}{}", RED, downgrades, RESET);
    println!();
    print_map(y_map, "Commands", 100, false);
}

macro_rules! impl_top{
    ($fn_name:ident, $enum_match:ident, $msg:expr, $obj:expr) => {
        fn $fn_name(events: Events, n: usize, all: bool){
            let mut map = FreqMap::new();
            for event in events{
                if let Event::$enum_match(_, prog, ..) = event {
                    map.inc(prog);
                }
            }

            let n = if all { map.len() } else { n };
            print_map(map, $msg, n, true);
            if all {
                println!("Number of {}: {RED}{BOLD}{n}{RESET}", $obj);
            }
        }
    };
}

impl_top!(top_commands, Command, "Commands", "commands");
impl_top!(top_installs, Installed, "Installs", "packages");
impl_top!(top_removes, Removed, "Removes", "packages");
impl_top!(top_upgrades, Upgraded, "Upgrades", "packages");
impl_top!(top_downgrades, Downgraded, "Downgrades", "packages");

fn package_history(events: Events, target_package: String, upgrade_command: bool){
    let mut last_command = String::new();
    for event in events{
        match event{
            // date time (y, m, d, h)
            Event::Command(_, command) => {
                last_command = command;
            },
            Event::Installed(dt, package, version) => {
                if target_package != package { continue; }
                println!(
                    "{} - {}{}installed{} version {}{}{}{}{} with: {}{}{}{}",
                    format_dt(dt), BOLD, GREEN, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                    ITALIC, MAGENTA, last_command, RESET
                );
            },
            Event::Removed(dt, package, version) => {
                if target_package != package { continue; }
                println!(
                    "{} - {}{}removed{} version {}{}{}{}{} with: {}{}{}{}",
                    format_dt(dt), BOLD, RED, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                    ITALIC, MAGENTA, last_command, RESET
                );
            },
            Event::Upgraded(dt, package, version) => {
                if target_package != package { continue; }
                println!(
                    "{} - {}upgraded{} {}from{} version{} to{} {}{}{}{}{}",
                    format_dt(dt), GREEN, RESET,
                    FAINT, RESET, FAINT, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
                if upgrade_command {
                    println!(
                        " with: {}{}{}{}",
                        ITALIC, MAGENTA, last_command, RESET
                    );
                }
            },
            Event::Downgraded(dt, package, version) => {
                if target_package != package { continue; }
                println!(
                    "{} - {}{}downgraded{} {}from{} version{} to{} {}{}{}{}{} with: {}{}{}{}",
                    format_dt(dt), RED, UNDERLINED, RESET,
                    FAINT, RESET, FAINT, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                    ITALIC, MAGENTA, last_command, RESET
                );
            },
        }
    }
}

fn history_full(events: Events, n: usize, no_upgrades: bool) {
    let mut filtered = Vec::new();
    let mut m = 0;
    let mut last_ok = false;
    for event in events.into_iter().rev(){
        match event{
            c@Event::Command(_, _) => {
                if last_ok {
                    filtered.push(c);
                    last_ok = false;
                    m += 1;
                    if m >= n { break; }
                }
            },
            u@Event::Upgraded(_, _, _) => {
                if m >= n { continue; }
                if !no_upgrades {
                    filtered.push(u);
                    last_ok = true;
                    m += 1;
                } else {
                    last_ok = false;
                }
            },
            other => {
                if m >= n { continue; }
                last_ok = true;
                filtered.push(other);
                m += 1;
            }
        }
    }
    for event in filtered.into_iter().rev()
    {
        match event{
            // date time (y, m, d, h)
            Event::Command(dt, command) => {
                println!(
                    "{} - {}{}command{}: {}{}{}{}",
                    format_dt(dt), BOLD, MAGENTA, RESET,
                    BOLD, ITALIC, command, RESET,
                );
            },
            Event::Installed(dt, package, version) => {
                println!(
                    "{} - {}{}installed{} {}{}{} version {}{}{}{}{}",
                    format_dt(dt), BOLD, GREEN, RESET,
                    BOLD, package, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
            Event::Removed(dt, package, version) => {
                println!(
                    "{} - {}{}removed{} {}{}{} version {}{}{}{}{}",
                    format_dt(dt), BOLD, RED, RESET,
                    BOLD, package, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
            Event::Upgraded(dt, package, version) => {
                println!(
                    "{} - {}upgraded{} {}{}{} {}from{} version{} to{} {}{}{}{}{}",
                    format_dt(dt), GREEN, RESET,
                    BOLD, package, RESET,
                    FAINT, RESET, FAINT, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
            Event::Downgraded(dt, package, version) => {
                println!(
                    "{} - {}{}downgraded{} {}{}{} {}from{} version{} to{} {}{}{}{}{}",
                    format_dt(dt), RED, UNDERLINED, RESET,
                    BOLD, package, RESET,
                    FAINT, RESET, FAINT, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
        }
    }
}

fn history_compact(events: Events, mut n: usize, count: bool) -> Result<(), fmt::Error> {
    let mut named = Vec::new();
    let mut unnamed = Vec::new();
    let mut install: Vec<String> = Vec::new();
    let mut remove: Vec<String> = Vec::new();
    let mut upgrade: Vec<String> = Vec::new();
    let mut downgrade: Vec<String> = Vec::new();
    let mut strings = Vec::new();
    let mut packages = 0;
    for event in &events {
        match event{
            Event::Installed(_, _, _) => { packages += 1; },
            Event::Removed(_, _, _) => { packages -= 1; },
            _ => { }
        }
    }
    for event in events.into_iter().rev(){
        match event{
            Event::Command(dt, command) => {
                let singular =
                    install.len().min(1) +
                    remove.len().min(1) +
                    // upgrade.len().min(1) +
                    downgrade.len().min(1) < 2;
                let words = command.split(' ').collect::<Vec<_>>();
                for package in install.iter()
                    .chain(remove.iter())
                    .chain(upgrade.iter())
                    .chain(downgrade.iter())
                {
                    let package = package.to_string();
                    if words.contains(&package.as_ref()) {
                        named.push(package);
                    } else {
                        unnamed.push(package);
                    }
                }
                let mut string = String::new();
                let mut done_something = false;
                let (hu, hd, hi, hr) = (
                    !upgrade.is_empty(), !downgrade.is_empty(),
                    !install.is_empty(), !remove.is_empty()
                );
                if count && (hi || hr || hd) {
                    write!(string, "{} - ", format_dt(dt))?;
                    let diff = install.len() as i32 - remove.len() as i32;
                    // green for negative because removing is good
                    let (dcol, dchar) = match diff.cmp(&0) {
                        std::cmp::Ordering::Less => (GREEN, '-'),
                        std::cmp::Ordering::Equal => (YELLOW, '+'),
                        std::cmp::Ordering::Greater => (RED, '+'),
                    };
                    write!(string,
                        "{BOLD}{dcol}{:>4}{RESET} -> {:<4} ",
                        format!("{dchar}{}", diff.abs()), packages
                    )?;
                    packages -= diff;
                    write!(string, "{BOLD}")?;
                    match (hi, hr, hu | hd) {
                        (true, false, false) => write!(string, "{GREEN}install{RESET}"),
                        (false, true, false) => write!(string, "{RED}remove{RESET} "),
                        _ => write!(string, "{MAGENTA}complex{RESET}"),
                    }?;
                    write!(string, "{RESET}")?;
                    if !named.is_empty() {
                        write!(string, " {}", named.vec_string_inner())?;
                    } else {
                        write!(string, " {MAGENTA}{}{RESET}", command)?;
                    }
                    done_something = true;
                } else if singular && !hu && !named.is_empty() {
                    write!(string, "{} - ", format_dt(dt))?;
                    if hi { write!(string, "{}{}install{} ", BOLD, GREEN, RESET)?; }
                    if hr { write!(string, "{}{}remove{} ", BOLD, RED, RESET)?; }
                    if hu { write!(string, "{}{}upgrade{} ", BOLD, GREEN, RESET)?; }
                    if hd { write!(string, "{}{}{}downgrade{} ", BOLD, UNDERLINED, RED, RESET)?; }
                    write!(string, "{}", named.vec_string_inner())?;
                    if !unnamed.is_empty() {
                        write!(string, ", {}{}{}", FAINT, unnamed.vec_string_inner(), RESET)?;
                    }
                    done_something = true;
                } else if !singular {
                    write!(string, "{} - ", format_dt(dt))?;
                    write!(string, "{}{}complex{} ", BOLD, MAGENTA, RESET)?;
                    write!(string, "{}{}{}{}",
                        GREEN, UNDERLINED, upgrade.vec_string_inner(), RESET)?;
                    if hu && (hd || hi || hr) { write!(string, ", ")?; }
                    write!(string, "{}{}{}{}",
                        RED, UNDERLINED, downgrade.vec_string_inner(), RESET)?;
                    if hd && (hi || hr) { write!(string, ", ")?; }
                    write!(string, "{}{}{}", GREEN, install.vec_string_inner(), RESET)?;
                    if hi && hr { write!(string, ", ")?; }
                    write!(string, "{}{}{}", RED, remove.vec_string_inner(), RESET)?;
                    write!(string, ", ")?;
                    write!(string, "{}{}{}", MAGENTA, command, RESET)?;
                    done_something = true;
                }
                if done_something {
                    writeln!(string)?;
                    strings.push(string);
                    n -= 1;
                    if n == 0 { break; }
                }
                named.clear();
                unnamed.clear();
                install.clear();
                remove.clear();
                upgrade.clear();
                downgrade.clear();
            },
            Event::Installed(_, package, _) => {
                install.push(package.to_string());
            },
            Event::Removed(_, package, _) => {
                remove.push(package.to_string());
            },
            Event::Upgraded(_, package, _) => {
                upgrade.push(package.to_string());
            },
            Event::Downgraded(_, package, _) => {
                downgrade.push(package.to_string());
            },
        }
    }
    for string in strings.into_iter().rev(){
        print!("{}", string);
    }
    Ok(())
}

fn intentional(events: Events, list: bool) {
    let mut install: Vec<String> = Vec::new();
    let mut remove: Vec<String> = Vec::new();
    let mut upgrade: Vec<String> = Vec::new();
    let mut downgrade: Vec<String> = Vec::new();
    let mut irlines = Vec::new();
    for event in events.into_iter().rev(){
        match event{
            Event::Command(_, command) => {
                let words = command.split(' ').collect::<Vec<_>>();
                for package in install.iter()
                {
                    let package = package.to_string();
                    if words.contains(&package.as_ref()) {
                        irlines.push(('i', package));
                    }
                }
                for package in remove.iter()
                {
                    let package = package.to_string();
                    if words.contains(&package.as_ref()) {
                        irlines.push(('r', package));
                    }
                }
                install.clear();
                remove.clear();
                upgrade.clear();
                downgrade.clear();
            },
            Event::Installed(_, package, _) => {
                install.push(package.to_string());
            },
            Event::Removed(_, package, _) => {
                remove.push(package.to_string());
            },
            Event::Upgraded(_, package, _) => {
                upgrade.push(package.to_string());
            },
            Event::Downgraded(_, package, _) => {
                downgrade.push(package.to_string());
            },
        }
    }
    let mut current = HashSet::new();
    let mut removed = HashSet::new();
    for ir in irlines.into_iter().rev(){
        if ir.0 == 'i' {
            current.insert(ir.1);
        } else {
            current.remove(&ir.1);
            removed.insert(ir.1);
        }
    }
    let cs = term_size::dimensions().unwrap_or((0, 0)).0;
    let l = current.iter().map(|p| p.chars().count()).max().unwrap_or(0);
    let l = if cs != 0 {
        let cols = cs / l;
        l + ((cs - cols * l) / cols).min(1)
    } else {
        0
    };
    let mut c = 0;
    for package in current.iter(){
        if list {
            println!("{}", package);
            continue;
        }
        if c + l >= cs {
            println!();
            c = l;
        } else {
            c += l;
        }
        if !removed.contains(package) {
            print!("{}", BOLD);
        } else {
            print!("{}", RESET);
        }
        print!("{:<l$}", package);
    }
}

fn time(events: Events, all: bool, year: bool, month: bool, day: bool, hour: bool){
    let (year, month, day, hour) = (
        year | all | !(month | day | hour), month | all, day | all, hour | all
    );

    let fmn = FreqMap::new;
    let ma = || [fmn(), fmn(), fmn(), fmn()];
    let [mut cy, mut cm, mut cd, mut ch] = ma();
    let [mut uy, mut um, mut ud, mut uh] = ma();
    let [mut dy, mut dm, mut dd, mut dh] = ma();
    let [mut iy, mut im, mut id, mut ih] = ma();
    let [mut ry, mut rm, mut rd, mut rh] = ma();

    type FMmr<'a> = &'a mut FreqMap<u16>;
    let inc = |my: FMmr, mm: FMmr, md: FMmr, mh: FMmr, (y, m, d, h): DT| {
        if year { my.inc(y); }
        if month { mm.inc(m as u16); }
        if day { md.inc(d as u16); }
        if hour { mh.inc(h as u16); }
    };

    for event in events{
        match event {
            Event::Command(dt, _) => inc(&mut cy, &mut cm, &mut cd, &mut ch, dt),
            Event::Installed(dt, _, _) => inc(&mut iy, &mut im, &mut id, &mut ih, dt),
            Event::Removed(dt, _, _) => inc(&mut ry, &mut rm, &mut rd, &mut rh, dt),
            Event::Upgraded(dt, _, _) => inc(&mut uy, &mut um, &mut ud, &mut uh, dt),
            Event::Downgraded(dt, _, _) => inc(&mut dy, &mut dm, &mut dd, &mut dh, dt),
        }
    }

    type FM = FreqMap<u16>;
    let pm = |condition: bool, c: FM, i: FM, r: FM, u: FM, d: FM, n: usize, msg: &str| {
        if !condition { return; }
        println!(" {}\n", msg);
        print_map(c, "Commands", n, false);
        print_map(i, "Installs", n, false);
        print_map(r, "Removes", n, false);
        print_map(u, "Upgrades", n, false);
        print_map(d, "Downgrades", n, false);
        println!();
    };

    pm(year, cy, iy, ry, uy, dy, 100, "- Per year -");
    pm(month, cm, im, rm, um, dm, 12, "- Per month -");
    pm(day, cd, id, rd, ud, dd, 31, "- Per day -");
    pm(hour, ch, ih, rh, uh, dh, 24, "- Per hour -");
}

fn print_map<T: Display + PartialEq + Eq + PartialOrd + Hash>(
    map: FreqMap<T>, msg: &str, n: usize, sort_by_freq: bool
){
    let total = map.total();
    println!("{}: {}{}{}{}", msg, BOLD, RED, total, RESET);
    let vec = if sort_by_freq {
        map.sorted_by_freq()
    } else {
        map.sorted_by_key()
    };
    for (to_display, freq) in vec.into_iter().take(n){
        println!(
            "\t{}{}{: >2}{}: {}{}{} times ({}{:.2}%{})",
            BOLD, GREEN, to_display, RESET, RED, freq, RESET, YELLOW,
            freq as f32 / total as f32 * 100.0, RESET,
        );
    }
}

struct FreqMap<T>{
    map: HashMap<T, usize>,
    total: usize,
}

impl<T: PartialEq + Eq + Hash> FreqMap<T>{
    pub fn new() -> Self{
        Self{
            map: HashMap::new(),
            total: 0,
        }
    }

    pub fn inc(&mut self, key: T){
        let freq = if let Some(freq) = self.map.get(&key){ freq + 1 } else { 1 };
        self.map.insert(key, freq);
        self.total += 1;
    }

    pub fn sorted_by_freq(self) -> Vec<(T, usize)>{
        let mut vec = self.map.into_iter().collect::<Vec<_>>();
        vec.sort_unstable_by(|(_, f0), (_, f1)| f1.partial_cmp(f0).unwrap());
        vec
    }

    pub fn total(&self) -> usize{
        self.total
    }

    pub fn len(&self) -> usize{
        self.map.iter().len()
    }
}

impl<T: PartialOrd> FreqMap<T>{
    pub fn sorted_by_key(self) -> Vec<(T, usize)>{
        let mut vec = self.map.into_iter().collect::<Vec<_>>();
        vec.sort_unstable_by(|(k0, _), (k1, _)| k0.partial_cmp(k1).unwrap());
        vec
    }
}

fn parse_dt(s: &str) -> Result<(u16, u8, u8, u8), ParseIntError>{
    let year: u16 = s[1..5].parse()?;
    let month: u8 = s[6..8].parse()?;
    let day: u8 = s[9..11].parse()?;
    let hour: u8 = s[12..14].parse()?;
    // [2023-06-30T02:12:34+0200] [ALPM] transaction completed
    // actually did run at around 2:12?
    // let offset: u8 = s[21..23].parse()?;
    Ok((year, month, day, hour))
}

fn format_dt((y, m, d, h): DT) -> String {
    format!(
        "{}{}/{}{:0>2}{}/{}{:0>2} {}{:0>2}{}:{}{}00{}",
        y, FAINT, RESET, m, FAINT, RESET, d, FAINT, h, BLACK, RESET, FAINT, RESET
    )
}

