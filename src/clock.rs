//! Local date and time.
//!
//! tmux runs `#()` jobs with the tmux server's environment, which is often
//! not the user's shell environment, so the zone comes from the `TZ` variable
//! when set and from `/etc/localtime` otherwise, through the C library's
//! `localtime_r`. Set `TZ` on the command line in the tmux config to pin a
//! zone.

use std::ffi::{c_char, c_int, c_long};
use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl LocalTime {
    pub fn date(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn time(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

/// `struct tm` as laid out by glibc and musl on 64-bit Linux.
#[repr(C)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

unsafe extern "C" {
    fn tzset();
    fn localtime_r(time: *const i64, result: *mut Tm) -> *mut Tm;
}

pub fn now() -> Option<LocalTime> {
    let seconds =
        i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs()).ok()?;
    let mut tm = MaybeUninit::<Tm>::uninit();
    // SAFETY: tzset takes no arguments; localtime_r gets a valid time_t
    // pointer and writable memory for a struct tm, and is thread-safe.
    let result = unsafe {
        tzset();
        localtime_r(&seconds, tm.as_mut_ptr())
    };
    if result.is_null() {
        return None;
    }
    // SAFETY: localtime_r returned non-null, so it filled the struct.
    let tm = unsafe { tm.assume_init() };
    Some(LocalTime {
        year: tm.tm_year + 1900,
        month: u8::try_from(tm.tm_mon + 1).ok()?,
        day: u8::try_from(tm.tm_mday).ok()?,
        hour: u8::try_from(tm.tm_hour).ok()?,
        minute: u8::try_from(tm.tm_min).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_padding() {
        let t = LocalTime {
            year: 2026,
            month: 1,
            day: 2,
            hour: 3,
            minute: 4,
        };
        assert_eq!(t.date(), "2026-01-02");
        assert_eq!(t.time(), "03:04");
    }

    #[test]
    fn reads_a_plausible_time() {
        let t = now().expect("localtime_r");
        assert!(t.year >= 2024);
        assert!((1..=12).contains(&t.month));
        assert!(t.hour < 24 && t.minute < 60);
    }
}
