use nix::sys::wait::{waitpid, WaitStatus};
use nix::sys::signal::Signal;
use nix::sys::ptrace;
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Instant};

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

    /// Hit breakpoints so we can compare to total and use as coverage metric
    hit_breakpoints: Vec<Breakpoint>,

    /// List of all break points we hit during execution
    coverage: usize,

    /// PID of debugee
    pid: Pid,

    /// When we attached to the target
    start_time: Instant,
}

impl Debugger {
    // We want to start a new instance of the process and attach to it. We
    // then run traceme() as a pre exec so the tracer does not miss anything 
    // the child does. This probably doesn't matter but its good practice I guess.
    pub fn spawn_traceable_proc(args: &[String]) -> Debugger {
        println!("Command: {}", args[0]);
        let mut tracee_pid: u32 = 0;
        unsafe {
            let child = Command::new(&args[0]).pre_exec(||{
                // Set ability to use ptrace on child
                ptrace::traceme()
                .expect("Pre exec call to traceme failed");
                Ok(())
            }).spawn()
            .expect("Failed spawning debugee process");
            tracee_pid = child.id();
        }

        // Check to make sure child is stopped in exec 
        match waitpid(Pid::from_raw(tracee_pid as i32), None) {
            Ok(WaitStatus::Stopped(_, Signal::SIGTRAP)) => {
                println!("Recieved correct state")
            },
            _ => panic!("Child in wrong state after traceme()")
        }
        
        // Bail if we try to attach to something we probably do not want to
        assert!(tracee_pid != 0, "Are you sure you want to trace pid: 0?");
        println!("Tracee pid: {}", tracee_pid);
        Debugger::attach(tracee_pid)
    }

    pub fn attach(pid: u32) -> Debugger {
        let  start_time = Instant::now();

        // ptrace attach
        let pid = Pid::from_raw(pid as i32); 
        ptrace::attach(pid).expect("Failed to attach ptrace");
        println!("Attached to {}", pid);

        // Here if we are not lazy we should:
        // 1. See if target is x86 or x86_64

        Debugger {
            all_breakpoints: Vec::new(),
            hit_breakpoints: Vec::new(),
            coverage: 0,
            pid: pid,
            start_time: start_time,
        }        
    }    

}
