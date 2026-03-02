#[derive(Clone, PartialEq)]
pub enum WifiState {
    Connected,
    Disconnected,
    Unknown,
}

#[cfg(target_os = "linux")]
pub fn wifi_state() -> WifiState {
    use std::fs;
    use std::path::Path;

    let net = Path::new("/sys/class/net");
    let iface = fs::read_dir(net)
        .ok()
        .and_then(|entries| entries.flatten().find(|e| e.path().join("wireless").exists()));

    let Some(iface) = iface else {
        return WifiState::Unknown;
    };

    match fs::read_to_string(iface.path().join("operstate")) {
        Ok(s) if s.trim() == "up" => WifiState::Connected,
        Ok(_) => WifiState::Disconnected,
        Err(_) => WifiState::Unknown,
    }
}

#[cfg(not(target_os = "linux"))]
pub fn wifi_state() -> WifiState {
    WifiState::Unknown
}

pub fn local_time() -> String {
    #[cfg(unix)]
    {
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm = std::mem::MaybeUninit::<libc::tm>::zeroed();
            libc::localtime_r(&now, tm.as_mut_ptr());
            let tm = tm.assume_init();
            format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
        }
    }
    #[cfg(not(unix))]
    {
        String::from("--:--")
    }
}
