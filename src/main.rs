use std::{
    fs::File,
    io::{ self, BufRead },
    path::Path,
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

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args{
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands{
    Test,
    Counts,
    Commands{
        #[clap(short, default_value_t = 16, help = "Amount of commands to show.")]
        n: usize,
    },
    Installs{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
    },
    Removes{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
    },
    Upgrades{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
    },
    Downgrades{
        #[clap(short, default_value_t = 16, help = "Amount of packages to show.")]
        n: usize,
    },
    Package{
        package: String,
        #[clap(long, help = "Show command used to do upgrades.")]
        upgrade_command: bool,
    },
    History{
        #[clap(short, default_value_t = 32, help = "Amount of items to show.")]
        n: usize,
        #[clap(short='f', help = "List out every event with version.")]
        full: bool,
        #[clap(short='u', help = "Ignore updates in full mode.")]
        no_upgrades: bool,
    },
}

fn main() {
    let args = Args::parse();
    let lines = if let Ok(lines) = read_lines("/var/log/pacman.log") {
        lines
    } else {
        panic!("Error: could not read '/var/log/pacman.log'!");
    };
    let parsed = parse(lines);

    match args.command{
        Commands::Test => {
            //run(parsed);
            lingering(parsed);
        },
        Commands::Counts => {
            counts(parsed);
        },
        Commands::Commands{ n } => {
            top_commands(parsed, n);
        },
        Commands::Installs{ n } => {
            top_installs(parsed, n);
        },
        Commands::Removes{ n } => {
            top_removes(parsed, n);
        },
        Commands::Upgrades{ n } => {
            top_upgrades(parsed, n);
        },
        Commands::Downgrades{ n } => {
            top_downgrades(parsed, n);
        },
        Commands::Package{ package, upgrade_command } => {
            package_history(parsed, package, upgrade_command);
        },
        Commands::History{ n, full, no_upgrades } => {
            if full {
                history_full(parsed, n, no_upgrades);
            } else if let Err(e) = history_compact(parsed, n){
                println!("{:?}", e);
            }
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

fn parse(lines: io::Lines<io::BufReader<File>>) -> Vec<Event>{
    let mut res = Vec::new();

    for line in lines.flatten() {
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

fn counts(events: Events){
    let nevents = events.len();
    let mut packages = 0usize;
    let mut commands = 0usize;
    let mut updates = 0usize;
    let mut installs = 0usize;
    let mut removes = 0usize;
    let mut upgrades = 0usize;
    let mut downgrades = 0usize;
    let mut last_command_update = false;

    for event in events{
        match event{
            Event::Command(_, _) => {
                last_command_update = false;
                commands += 1;
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

    println!("Events: {}{}{}", RED, nevents, RESET);
    println!("Packages: {}{}{}", RED, packages, RESET);
    println!("Updates: {}{}{}", RED, updates, RESET);
    println!("Commands: {}{}{}", RED, commands, RESET);
    println!("Installs: {}{}{}", RED, installs, RESET);
    println!("Removes: {}{}{}", RED, removes, RESET);
    println!("Upgrades: {}{}{}", RED, upgrades, RESET);
    println!("Downgrades: {}{}{}", RED, downgrades, RESET);
}

fn top_commands(events: Events, n: usize){
    let mut command_map = FreqMap::new();
    for event in events{
        if let Event::Command(_, com) = event {
            command_map.inc(com);
        }
    }

    print_map(command_map, "Commands:", n, true);
}

fn top_installs(events: Events, n: usize){
    let mut install_map = FreqMap::new();
    for event in events{
        if let Event::Installed(_, prog, _) = event {
            install_map.inc(prog);
        }
    }

    print_map(install_map, "Installs:", n, true);
}

fn top_removes(events: Events, n: usize){
    let mut remove_map = FreqMap::new();
    for event in events{
        if let Event::Removed(_, prog, _) = event {
            remove_map.inc(prog);
        }
    }

    print_map(remove_map, "Removes:", n, true);
}

fn top_upgrades(events: Events, n: usize){
    let mut upgrade_map = FreqMap::new();
    for event in events{
        if let Event::Upgraded(_, prog, _) = event{
            upgrade_map.inc(prog);
        }
    }

    print_map(upgrade_map, "Upgrades:", n, true);
}

fn top_downgrades(events: Events, n: usize){
    let mut downgrade_map = FreqMap::new();
    for event in events{
        if let Event::Downgraded(_, prog, _) = event{
            downgrade_map.inc(prog);
        }
    }

    print_map(downgrade_map, "Downgrades:", n, true);
}


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
                    "{} - {}{}Installed{} version {}{}{}{}{} with: {}{}{}{}",
                    format_dt(dt), BOLD, GREEN, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                    ITALIC, MAGENTA, last_command, RESET
                );
            },
            Event::Removed(dt, package, version) => {
                if target_package != package { continue; }
                println!(
                    "{} - {}{}Removed{} version {}{}{}{}{} with: {}{}{}{}",
                    format_dt(dt), BOLD, RED, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                    ITALIC, MAGENTA, last_command, RESET
                );
            },
            Event::Upgraded(dt, package, version) => {
                if target_package != package { continue; }
                println!(
                    "{} - {}Upgraded{} {}from{} version{} to{} {}{}{}{}{}",
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
                    "{} - {}{}Downgraded{} {}from{} version{} to{} {}{}{}{}{} with: {}{}{}{}",
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
                    "{} - {}{}Command{}: {}{}{}{}",
                    format_dt(dt), BOLD, MAGENTA, RESET,
                    BOLD, ITALIC, command, RESET,
                );
            },
            Event::Installed(dt, package, version) => {
                println!(
                    "{} - {}{}Installed{} {}{}{} version {}{}{}{}{}",
                    format_dt(dt), BOLD, GREEN, RESET,
                    BOLD, package, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
            Event::Removed(dt, package, version) => {
                println!(
                    "{} - {}{}Removed{} {}{}{} version {}{}{}{}{}",
                    format_dt(dt), BOLD, RED, RESET,
                    BOLD, package, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
            Event::Upgraded(dt, package, version) => {
                println!(
                    "{} - {}Upgraded{} {}{}{} {}from{} version{} to{} {}{}{}{}{}",
                    format_dt(dt), GREEN, RESET,
                    BOLD, package, RESET,
                    FAINT, RESET, FAINT, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
            Event::Downgraded(dt, package, version) => {
                println!(
                    "{} - {}{}Downgraded{} {}{}{} {}from{} version{} to{} {}{}{}{}{}",
                    format_dt(dt), RED, UNDERLINED, RESET,
                    BOLD, package, RESET,
                    FAINT, RESET, FAINT, RESET,
                    FAINT, ITALIC, CYAN, version, RESET,
                );
            },
        }
    }
}

fn history_compact(events: Events, mut n: usize) -> Result<(), fmt::Error> {
    let mut named = Vec::new();
    let mut unnamed = Vec::new();
    let mut install: Vec<String> = Vec::new();
    let mut remove: Vec<String> = Vec::new();
    let mut upgrade: Vec<String> = Vec::new();
    let mut downgrade: Vec<String> = Vec::new();
    let mut strings = Vec::new();
    for event in events.into_iter().rev(){
        match event{
            Event::Command(dt, command) => {
                let singular =
                    install.len().min(1) +
                    remove.len().min(1) +
                    upgrade.len().min(1) +
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
                if !named.is_empty() {
                    let mut string = String::new();
                    if singular && upgrade.is_empty() {
                        write!(string, "{} - ", format_dt(dt))?;
                        if !install.is_empty() {
                            write!(string, "{}{}Install{} ", BOLD, GREEN, RESET)?;
                        }
                        if !remove.is_empty() {
                            write!(string, "{}{}Remove{} ", BOLD, RED, RESET)?;
                        }
                        if !upgrade.is_empty() {
                            write!(string, "{}{}Upgrade{} ", BOLD, GREEN, RESET)?;
                        }
                        if !downgrade.is_empty() {
                            write!(string, "{}{}{}Downgrade{} ", BOLD, UNDERLINED, RED, RESET)?;
                        }
                        write!(string, "{}", named.vec_string_inner())?;
                        if !unnamed.is_empty() {
                            write!(string, ", {}{}{}", FAINT, unnamed.vec_string_inner(), RESET)?;
                        }
                    } else if !singular {
                        write!(string, "{} - ", format_dt(dt))?;
                        write!(string, "{}{}Complex{} ", BOLD, MAGENTA, RESET)?;
                        write!(string, "{}{}{}{}",
                            GREEN, UNDERLINED, upgrade.vec_string_inner(), RESET)?;
                        if !upgrade.is_empty() && !downgrade.is_empty() { write!(string, ", ")?; }
                        write!(string, "{}{}{}{}",
                            RED, UNDERLINED, downgrade.vec_string_inner(), RESET)?;
                        if !downgrade.is_empty() && !install.is_empty() { write!(string, ", ")?; }
                        write!(string, "{}{}{}", GREEN, install.vec_string_inner(), RESET)?;
                        if !install.is_empty() && !remove.is_empty() { write!(string, ", ")?; }
                        write!(string, "{}{}{}", RED, remove.vec_string_inner(), RESET)?;
                        write!(string, ", ")?;
                        write!(string, "{}{}{}", MAGENTA, command, RESET)?;
                    }
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

fn lingering(events: Events) {
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
    for package in current.iter(){
        if !removed.contains(package) {
            println!("{}", package);
        }
    }
    println!("--------");
    for package in current.into_iter(){
        if removed.contains(&package) {
            println!("{}", package);
        }
    }
}

fn run(events: Events){
    let mut y_map = FreqMap::new();
    let mut m_map = FreqMap::new();
    let mut d_map = FreqMap::new();
    let mut h_map = FreqMap::new();

    for event in events{
        if let Event::Command((y, m, d, h), _) = event {
            y_map.inc(y);
            m_map.inc(m as u16);
            d_map.inc(d as u16);
            h_map.inc(h as u16);
        }
    }

    // print_bargraph(into_graphdata(y_map), 32, 4);
    print_bargraph(into_graphdata(m_map), 32, 5);
    print_bargraph(into_graphdata(d_map), 32, 3);
    print_bargraph(into_graphdata(h_map), 32, 4);
    // print_map(y_map, "Commands (Year):", 10, false);
    // print_map(m_map, "Commands (Month):", 12, false);
    // print_map(d_map, "Commands (Day):", 31, false);
    // print_map(h_map, "Commands (Hour):", 24, false);
}

fn print_map<T: Display + PartialEq + Eq + PartialOrd + Hash>(
    map: FreqMap<T>, msg: &str, n: usize, sort_by_freq: bool
){
    println!("{} {}{}{}{}", msg, BOLD, RED, map.get_total(), RESET);
    let vec = if sort_by_freq {
        map.sorted_by_freq()
    } else {
        map.sorted_by_key()
    };
    for (to_display, freq) in vec.into_iter().take(n){
        println!("\t{}{}{}{}: {}{}{} times", BOLD, GREEN, to_display, RESET, RED, freq, RESET);
    }
}

fn into_graphdata(map: FreqMap<u16>) -> Vec<(u16, usize)>{
    // fill holes
    let mut filled = Vec::new();
    let mut prev = 0;
    for (k, f) in map.sorted_by_key().into_iter(){
        while k > prev + 1{
            filled.push((prev + 1, 0));
            prev += 1;
        }
        filled.push((k, f));
        prev = k;
    }
    filled
}

fn print_bargraph(data: Vec<(u16, usize)>, h: usize, bw: usize){
    let max = *data.iter().map(|(_, f)| f).max().unwrap();
    for i in 0..h{
        let j = h - i;
        for (_, f) in &data{
            let c = if *f as f32 / max as f32 >= j as f32 / h as f32{
                "x"
            } else {
                " "
            };
            for _ in 0..bw{
                print!("{}", c);
            }
        }
        println!();
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

    pub fn get_total(&self) -> usize{
        self.total
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
        "{}{}/{}{:0>2}{}/{}{:0>2} {}{:0>2}{}:{}00{}",
        y, FAINT, RESET, m, FAINT, RESET, d, FAINT, h, BLACK, WHITE, RESET
    )
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
