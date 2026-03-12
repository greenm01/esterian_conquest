use std::path::PathBuf;

use super::paths::resolve_repo_path;

pub(crate) fn parse_u8_arg(value: &str, label: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let parsed = if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        u8::from_str_radix(hex, 16)?
    } else {
        value.parse::<u8>()?
    };
    let _ = label;
    Ok(parsed)
}

pub(crate) fn parse_u16_arg(
    value: &str,
    label: &str,
) -> Result<u16, Box<dyn std::error::Error>> {
    let parsed = if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        u16::from_str_radix(hex, 16)?
    } else {
        value.parse::<u16>()?
    };
    let _ = label;
    Ok(parsed)
}

pub(crate) fn parse_usize_1_based(
    value: &str,
    label: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let parsed = value.parse::<usize>()?;
    if parsed == 0 {
        return Err(format!("{label} must be >= 1").into());
    }
    Ok(parsed)
}

pub(crate) fn parse_optional_source_and_target(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf)> {
    match args.as_slice() {
        [target] => Some((default_source, PathBuf::from(target))),
        [source, target] => Some((resolve_repo_path(source), PathBuf::from(target))),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_name(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, String)> {
    match args.as_slice() {
        [target, scenario_name] => Some((default_source, PathBuf::from(target), scenario_name.clone())),
        [source, target, scenario_name] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            scenario_name.clone(),
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_xy(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u8, u8)> {
    match args.as_slice() {
        [target, x, y] => Some((
            default_source,
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
        )),
        [source, target, x, y] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            parse_u8_arg(x, "target_x").ok()?,
            parse_u8_arg(y, "target_y").ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_count(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, u16)> {
    match args.as_slice() {
        [target, count] => Some((
            default_source,
            PathBuf::from(target),
            count.parse::<u16>().ok()?,
        )),
        [source, target, count] => Some((
            resolve_repo_path(source),
            PathBuf::from(target),
            count.parse::<u16>().ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_optional_source_target_and_count_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, Vec<u16>)> {
    match args.as_slice() {
        [target_root, counts @ ..] if !counts.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            counts.iter().map(|value| value.parse::<u16>().ok()).collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, counts @ ..] if !counts.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            counts.iter().map(|value| value.parse::<u16>().ok()).collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

pub(crate) fn parse_coord_pair(value: &str) -> Option<[u8; 2]> {
    let (x, y) = value.split_once(':')?;
    Some([
        parse_u8_arg(x, "target_x").ok()?,
        parse_u8_arg(y, "target_y").ok()?,
    ])
}

pub(crate) fn parse_optional_source_target_and_coord_list(
    args: Vec<String>,
    default_source: PathBuf,
) -> Option<(PathBuf, PathBuf, Vec<[u8; 2]>)> {
    match args.as_slice() {
        [target_root, coords @ ..] if !coords.is_empty() => Some((
            default_source,
            PathBuf::from(target_root),
            coords.iter().map(|value| parse_coord_pair(value)).collect::<Option<Vec<_>>>()?,
        )),
        [source, target_root, coords @ ..] if !coords.is_empty() => Some((
            resolve_repo_path(source),
            PathBuf::from(target_root),
            coords.iter().map(|value| parse_coord_pair(value)).collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}
