use std::fs::File;
use std::io::{ self, BufRead };
use std::path::Path;
use std::collections::HashMap;
use std::num::ParseIntError;

type DT = (u16, u8, u8, u8); // date time (y, m, d, h)
enum Event{
    Command(DT, String), // dt, command
    Removed(DT, String, String), // dt, package, version
    Installed(DT, String, String), // dt, package, version
    Upgraded(DT, String, String), // dt, package, version
    Downgraded(DT, String, String), // dt, package, version
}

fn main() {
    if let Ok(lines) = read_lines("/var/log/pacman.log") {
        run(parse(lines));
    }
}

fn parse(lines: io::Lines<io::BufReader<File>>) -> Vec<Event>{
    let mut res = Vec::new();

    for line in lines.flatten() {
        let parts = line.split(' ').collect::<Vec<_>>();
        if parts.len() < 4 { continue; }

        let dt = if let Ok(dt) = parse_dt(parts[0]) { dt } else { continue; };

        if parts[1] == "[PACMAN]" && parts[2] == "Running"{
            let command = parts[3..].join(" ");
            res.push(Event::Command(dt, command));
        } else if parts[1] == "[ALPM]" && parts[2] == "removed"{
            let version = parts[4..].join(" ");
            res.push(Event::Removed(dt, parts[3].to_string(), version));
        } else if parts[1] == "[ALPM]" && parts[2] == "installed"{
            let version = parts[4..].join(" ");
            res.push(Event::Installed(dt, parts[3].to_string(), version));
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

fn run(events: Vec<Event>){
    let mut command_map = HashMap::new();
    let mut update_map = HashMap::new();

    let nevents = events.len();
    let mut ncommands = 0usize;
    let mut packages = 0usize;
    let mut upgrades = 0usize;
    let mut downgrades = 0usize;
    let mut updates = 0usize;
    let mut last_command_update = false;

    for event in events{
        match event{
            Event::Command(_, com) => {
                let freq = if let Some(freq) = command_map.get(&com){ freq + 1 } else { 1 };
                command_map.insert(com, freq);
                ncommands += 1;
                last_command_update = false;
            },
            Event::Removed(_, _, _) => {
                packages -= 1;
            },
            Event::Installed(_, _, _) => {
                packages += 1;
            },
            Event::Upgraded(_, prog, _) => {
                upgrades += 1;
                if !last_command_update{
                    last_command_update = true;
                    updates += 1;
                }
                let freq = if let Some(freq) = update_map.get(&prog){ freq + 1 } else { 1 };
                update_map.insert(prog, freq);
            },
            Event::Downgraded(_, _, _) => {
                downgrades += 1;
            },
        }
    }

    println!("Events: {}", nevents);
    println!("Commands: {}", ncommands);
    println!("Packages: {}", packages);
    println!("Upgrades: {}", upgrades);
    println!("Downgrades: {}", downgrades);
    println!("Update: {}", updates);

    println!();
    let mut command_map = command_map.into_iter().collect::<Vec<_>>();
    command_map.sort_unstable_by(|(_, f0), (_, f1)| f1.partial_cmp(f0).unwrap());
    for (command, freq) in command_map.into_iter().take(10){
        println!("{}: {} times", command, freq);
    }

    println!();
    let mut update_map = update_map.into_iter().collect::<Vec<_>>();
    update_map.sort_unstable_by(|(_, f0), (_, f1)| f1.partial_cmp(f0).unwrap());
    for (prog, freq) in update_map.into_iter().take(10){
        println!("{}: {} times", prog, freq);
    }
}

fn parse_dt(s: &str) -> Result<(u16, u8, u8, u8), ParseIntError>{
    let year: u16 = s[1..5].parse()?;
    let month: u8 = s[6..8].parse()?;
    let day: u8 = s[9..11].parse()?;
    let hour: u8 = s[12..14].parse()?;
    let offset: u8 = s[21..23].parse()?;
    Ok((year, month, day, (hour + offset) % 24))
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
