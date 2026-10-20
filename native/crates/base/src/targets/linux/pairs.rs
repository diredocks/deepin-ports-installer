//! `lsblk --pairs` reader, used when the installed `lsblk` has no JSON output.

use std::collections::HashMap;
use std::path::PathBuf;

use super::{Device, is_virtual, partition_table, run_lsblk, text_is_true};
use crate::targets::{Enumerator, Mountpoint, Target};
use crate::{Error, Result};

/// One `--pairs` row, keyed by lowercased column name.
type Fields = HashMap<String, String>;

pub(super) fn scan() -> Result<Vec<Target>> {
    let output = run_lsblk(&["--bytes", "--all", "--pairs", "-o", "+pttype"])
        .or_else(|_| run_lsblk(&["--bytes", "--all", "--pairs"]))?;

    parse(&output)
}

fn parse(output: &str) -> Result<Vec<Target>> {
    let rows = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(parse_line)
        .collect::<Result<Vec<_>>>()?;

    Ok(rows
        .iter()
        .filter(|row| value(row, "type") == Some("disk"))
        .filter_map(|row| target(row, &rows))
        .collect())
}

/// Parse a single `KEY="value" KEY="value"` line.
fn parse_line(line: &str) -> Result<Fields> {
    let mut fields = Fields::new();
    let mut rest = line.trim();

    while !rest.is_empty() {
        let Some((key, value)) = rest.split_once('=') else {
            return Err(Error::LsblkOutput(line.to_owned()));
        };
        let Some((parsed, consumed)) = parse_quoted(value) else {
            return Err(Error::LsblkOutput(line.to_owned()));
        };

        fields.insert(key.trim().to_ascii_lowercase(), parsed);
        rest = value[consumed..].trim_start();
    }

    Ok(fields)
}

/// Parse a leading quoted value, returning it and the number of bytes read.
fn parse_quoted(input: &str) -> Option<(String, usize)> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for (offset, ch) in chars {
        match ch {
            _ if escaped => {
                value.push(ch);
                escaped = false;
            }
            '\\' => escaped = true,
            '"' => return Some((value, offset + ch.len_utf8())),
            _ => value.push(ch),
        }
    }

    None
}

fn target(row: &Fields, rows: &[Fields]) -> Option<Target> {
    let name = value(row, "name")?;
    let mut device = Device::new(name)?;

    // `--pairs` prints disks and partitions as a flat list, so partitions are
    // matched back to their disk by name prefix, like `drivelist` does.
    device.mountpoints = rows
        .iter()
        .filter(|child| matches!(value(child, "type"), Some("disk" | "part")))
        .filter(|child| value(child, "name").is_some_and(|child| child.starts_with(name)))
        .filter_map(mountpoint)
        .collect();

    device.label = value(row, "label").map(str::to_owned);
    device.vendor = value(row, "vendor").map(str::to_owned);
    device.model = value(row, "model").map(str::to_owned);
    device.size = number(row, "size");
    device.block_size = number(row, "phy-sec");
    device.logical_block_size = number(row, "log-sec");
    device.partition_table = partition_table(value(row, "pttype"));
    device.is_virtual = is_virtual(value(row, "subsystems"));
    device.set_transport(value(row, "tran"));
    device.set_flags(
        value(row, "ro").is_some_and(text_is_true),
        value(row, "rm").is_some_and(text_is_true),
        value(row, "hotplug").is_some_and(text_is_true),
    );

    Some(device.into_target(Enumerator::LsblkPairs))
}

fn mountpoint(row: &Fields) -> Option<Mountpoint> {
    Some(Mountpoint {
        path: PathBuf::from(value(row, "mountpoint")?),
        label: value(row, "label")
            .or_else(|| value(row, "partlabel"))
            .map(str::to_owned),
    })
}

fn value<'a>(row: &'a Fields, key: &str) -> Option<&'a str> {
    row.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

fn number(row: &Fields, key: &str) -> Option<u64> {
    value(row, key)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::{BusType, PartitionTable};

    const PAIRS: &str = r#"NAME="sda" MAJ:MIN="8:0" RM="0" SIZE="240065183744" RO="0" TYPE="disk" MOUNTPOINT="" PTTYPE="gpt"
NAME="sda1" MAJ:MIN="8:1" RM="0" SIZE="209715200" RO="0" TYPE="part" MOUNTPOINT="/boot/efi" LABEL="EFI" PTTYPE="gpt"
NAME="sda2" MAJ:MIN="8:2" RM="0" SIZE="115835142144" RO="0" TYPE="part" MOUNTPOINT="" PTTYPE="gpt"
NAME="sdb" MAJ:MIN="8:16" RM="1" SIZE="31457280000" RO="0" TYPE="disk" MOUNTPOINT="" PTTYPE="" TRAN="usb" VENDOR="SanDisk" MODEL="Ultra USB"
NAME="sdb1" MAJ:MIN="8:17" RM="1" SIZE="31457279000" RO="0" TYPE="part" MOUNTPOINT="/media/card" LABEL="DEEPIN" PTTYPE=""
NAME="sr0" MAJ:MIN="11:0" RM="1" SIZE="1048576" RO="0" TYPE="rom" MOUNTPOINT=""
"#;

    #[test]
    fn parses_pairs_output() {
        let targets = parse(PAIRS).unwrap();
        assert_eq!(targets.len(), 2);

        let system = &targets[0];
        assert_eq!(system.device, PathBuf::from("/dev/sda"));
        assert_eq!(system.enumerator, Enumerator::LsblkPairs);
        assert_eq!(system.size, Some(240_065_183_744));
        assert_eq!(system.block_size, 512);
        assert_eq!(system.partition_table, Some(PartitionTable::Gpt));
        assert!(system.is_system);
        assert!(!system.is_removable);
        assert_eq!(system.description, "/dev/sda");
        assert_eq!(system.mountpoints.len(), 1);

        let usb = &targets[1];
        assert_eq!(usb.device, PathBuf::from("/dev/sdb"));
        assert_eq!(usb.bus_type, BusType::Usb);
        assert!(usb.is_removable);
        assert!(!usb.is_system);
        assert_eq!(usb.description, "SanDisk Ultra USB");
        assert_eq!(usb.mountpoints[0].path, PathBuf::from("/media/card"));
    }

    #[test]
    fn parses_escaped_quotes_in_pairs_values() {
        let line = r#"NAME="sda" MODEL="Some \"Quoted\" Model" EMPTY="" TYPE="disk""#;
        let fields = parse_line(line).unwrap();

        assert_eq!(fields["model"], r#"Some "Quoted" Model"#);
        assert_eq!(fields["empty"], "");
        assert_eq!(fields["type"], "disk");
    }

    #[test]
    fn rejects_malformed_pairs_lines() {
        assert!(matches!(
            parse_line("NAME=sda TYPE=\"disk\""),
            Err(Error::LsblkOutput(_))
        ));
        assert!(matches!(
            parse_line("NAME=\"sda"),
            Err(Error::LsblkOutput(_))
        ));
    }

    #[test]
    fn empty_pairs_output_yields_no_targets() {
        assert!(parse("").unwrap().is_empty());
        assert!(parse("  \n \n").unwrap().is_empty());
    }
}
