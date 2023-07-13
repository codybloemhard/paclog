use std::{
    fs::File,
    io::{ self, BufRead },
    path::Path,
    collections::HashMap,
    num::ParseIntError,
    hash::Hash,
    fmt::Display,
};


use clap::{
    Parser,
    Subcommand,
};

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
        #[clap(short, default_value_t = 10, help = "Amount of commands to show.")]
        n: usize,
    },
    Installs{
        #[clap(short, default_value_t = 10, help = "Amount of packages to show.")]
        n: usize,
    },
    Removes{
        #[clap(short, default_value_t = 10, help = "Amount of packages to show.")]
        n: usize,
    },
    Upgrades{
        #[clap(short, default_value_t = 10, help = "Amount of packages to show.")]
        n: usize,
    },
    Downgrades{
        #[clap(short, default_value_t = 10, help = "Amount of packages to show.")]
        n: usize,
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
            run(parsed);
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
            let command = parts[3..].join(" ");
            res.push(Event::Command(dt, command));
        } else if parts[1] == "[ALPM]" && parts[2] == "installed"{
            let version = parts[4..].join(" ");
            res.push(Event::Installed(dt, parts[3].to_string(), version));
        } else if parts[1] == "[ALPM]" && parts[2] == "removed"{
            let version = parts[4..].join(" ");
            res.push(Event::Removed(dt, parts[3].to_string(), version));
        }
        else if parts[1] == "[ALPM]" && parts[2] == "upgraded"{
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

    println!("Events: {}", nevents);
    println!("Packages: {}", packages);
    println!("Updates: {}", updates);
    println!("Commands: {}", commands);
    println!("Installs: {}", installs);
    println!("Removes: {}", removes);
    println!("Upgrades: {}", upgrades);
    println!("Downgrades: {}", downgrades);
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
    println!("{} {}", msg, map.get_total());
    let vec = if sort_by_freq {
        map.sorted_by_freq()
    } else {
        map.sorted_by_key()
    };
    for (to_display, freq) in vec.into_iter().take(n){
        println!("\t{}: {} times", to_display, freq);
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

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
