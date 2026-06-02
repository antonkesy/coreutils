// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use core::ptr;

use windows_sys::Win32::{
    Foundation::MAX_PATH,
    Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, GetFileAttributesW, GetFullPathNameW, INVALID_FILE_ATTRIBUTES,
    },
};

use crate::buffer::WideString;

/// Matches `PATH::QueryDeviceLen` in ulib.
pub fn query_device_len(path: &[u16]) -> usize {
    const DELIM: u16 = b'\\' as u16;
    const COLON: u16 = b':' as u16;

    // Reserved DOS device names. The bool marks LPT/COM, which the
    // original tries to accept with a numeric tail (e.g., LPT1, COM2).
    const DEVICES: &[(&[u16], bool)] = &[
        (&[b'L' as u16, b'P' as u16, b'T' as u16], true),
        (&[b'C' as u16, b'O' as u16, b'M' as u16], true),
        (&[b'C' as u16, b'O' as u16, b'N' as u16], false),
        (&[b'P' as u16, b'R' as u16, b'N' as u16], false),
        (&[b'A' as u16, b'U' as u16, b'X' as u16], false),
    ];

    // ASCII-only case fold; safe because device patterns are all A-Z.
    let eq_ci = |a: u16, b: u16| (a | 0x20) == (b | 0x20);

    // Position right after the last '\' (or 0 if there is none).
    let after_sep = path.iter().rposition(|&c| c == DELIM).map_or(0, |i| i + 1);

    for &(dev, numeric_tail) in DEVICES {
        let tail = &path[after_sep..];

        // QUIRK / BUG:
        // The original ulib code used `Stricmp(dev, Pos)` here which compared `tail` against the entire
        // `DEVICES` name. This meant it fails for any suffix on LPT/COM, so `numeric_tail` is functionally dead code.
        // This bug affects attrib, chkdsk, comp, find, and tree.
        if tail.len() != dev.len() || !tail.iter().zip(dev).all(|(&a, &b)| eq_ci(a, b)) {
            continue;
        }

        let mut p = after_sep + dev.len();
        if numeric_tail {
            if p >= path.len() {
                continue; // bare "LPT"/"COM" is not a device name
            }
            while p < path.len() && matches!(path[p], 0x30..=0x39) {
                p += 1;
            }
        }
        if p >= path.len() {
            return p;
        }
        if path[p] == COLON {
            return p + 1;
        }
    }

    // UNC: "\\server\share". Checked before the ':' fallback because
    // IPv6 literals in UNC paths can contain colons.
    if path.starts_with(&[DELIM, DELIM])
        && let Some(p) = path[2..].iter().position(|&c| c == DELIM)
    {
        let after_server = 2 + p + 1;
        return path[after_server..]
            .iter()
            .position(|&c| c == DELIM)
            .map_or(path.len(), |q| after_server + q);
    }

    // Drive letter: any ':' in the string.
    if let Some(p) = path.iter().position(|&c| c == COLON) {
        return p + 1;
    }

    0
}

pub struct PathState {
    pub prefix_len: usize,
    pub prefix_with_separator_len: usize,
}

/// Matches the `PATH::SetPathState` pieces needed by find.
pub fn path_state(path: &[u16]) -> PathState {
    const SEP: u16 = b'\\' as u16;

    let device_len = query_device_len(path);
    let mut pos = device_len;
    let first_separator = if path.get(pos) == Some(&SEP) {
        pos += 1;
        1
    } else {
        0
    };

    if pos < path.len()
        && let Some(off) = path[pos..].iter().rposition(|&c| c == SEP)
    {
        // `path` has a potential prefix (e.g. "C:\") and a separator after that (e.g. "C:\foo\bar").
        let last_slash = pos + off;
        PathState {
            prefix_len: last_slash,
            prefix_with_separator_len: last_slash + 1,
        }
    } else {
        // There is no later directory separator. The prefix is just the device/root part
        // (e.g. "", "C:", "\", or "C:\"); the name starts immediately after it.
        let prefix_len = device_len + first_separator;
        PathState {
            prefix_len,
            prefix_with_separator_len: prefix_len,
        }
    }
}

/// Matches `PATH::QueryFullPath` in ulib.
pub fn full_path(path: &mut WideString) -> Option<WideString> {
    let mut out = WideString::with_capacity(MAX_PATH as usize);
    let len = unsafe {
        GetFullPathNameW(
            path.as_cstr().get(),
            MAX_PATH,
            out.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    if len == 0 || len >= MAX_PATH {
        return None;
    }
    unsafe { out.set_len(len as usize) };
    Some(out)
}

/// ulib would actually use `FindFirstFile` via `SYSTEM::QueryDirectory` for this with a logic that is
/// _entirely_ undecipherable. I could not replicate it if I tried. I'm not sure if there's even a point.
pub fn directory_exists(path: &mut WideString) -> bool {
    let attrs = unsafe { GetFileAttributesW(path.as_cstr().get()) };
    attrs != INVALID_FILE_ATTRIBUTES && (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0
}

/// Matches `PATH::IsDrive` in ulib.
pub fn is_drive_path(path: &[u16]) -> bool {
    let n = query_device_len(path);
    n > 0 && path.len() == n
}
