# Paclog

A tool to query the pacman log file.
Makes the data human-readable.
Read up on statistics and package history.
Useful when fixing problems or cleaning up the system.

```
Usage: paclog <COMMAND>

Commands:
  test
  counts       Print some statistics.
  commands     Print most run commands.
  installs     Print most installed packages.
  removes      Print most removed packages.
  upgrades     Print most upgraded packages.
  downgrades   Print most downgraded packages.
  package      Print package history.
  history      Print pacman history.
  intentional  List currently intentionally installed packages. Bold if never removed.
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

```
Copyright (C) 2023 Cody Bloemhard

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
