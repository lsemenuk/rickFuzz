use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};

pub struct Breakpoint {
    /// Offset into binary at which to apply the breakpoint
    offset: usize,

    /// Track if breakpoint is enabled or not
    enabled: bool,

    /// Original byte at breakpoint location so we can reset it after we hit the
    /// breakpoint for the first time
    orig_byte: Option<u8>,

    /// Name of the function in which the breakpoint is set. This will mainly
    /// be for coverage data readability
    func_name: String,

    /// Offset into the function at which the breakpoint should be set.
    /// Not sure if we will need this atm as it will probs be for readability.
    func_off: usize,

    /// Incase we want to track freq
    freq: u64,
}

pub struct Debugger {
    /// List of all breakpoints we want to apply to the targeted process
    all_breakpoints: Vec<Breakpoint>,

    /// List of all break points we hit during execution
    coverage: HashMap<usize, (String, usize, String, u64)>,

    /// PID of debugee
    pid: u32,

    /// When we attached to the target
    start_time: Instant,
}

