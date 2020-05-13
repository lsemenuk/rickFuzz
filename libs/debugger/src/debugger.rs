use nix::sys::wait::{waitpid, WaitStatus};
use nix::sys::signal::Signal;
use nix::sys::ptrace;
use nix::sys::ptrace::AddressType;
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Instant};
use std::ffi::c_void;

pub struct Breakpoint {
    /// Addr in binary at which to apply the breakpoint
    addr: AddressType,

    /// Track if breakpoint is enabled or not
    enabled: bool,

    /// Original byte at breakpoint location so we can reset it after we hit the
    /// breakpoint for the first time
    orig_byte: u8,

    /// Name of the function in which the breakpoint is set. This will mainly
    /// be for coverage data readability
    func_name: String,

    // Offset into the function at which the breakpoint should be set.
    // Not sure if we will need this atm as it will probs be for readability.
    //func_off: usize,

    // Incase we want to track freq
    //freq: u64,
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

    // Construct and set the breakpoint in the target process
    // In order to set a breakpoint with ptrace we need to set the 
    // byte pointed to by the breakpoint address to 0xcc(halt). Then
    // when the breakpoint is hit we replace the the halt with the 
    // original instruction and continue execution by setting eip = eip - 1.
    pub fn set_breakpoint(&mut self, pid: Pid, bp_addr: AddressType) {

        //Breakpoint address
        let addr = bp_addr;

        // Breakpoint should be enable upon creation
        let enabled = true;
        
        // Get the original byte we are swapping 0xCC for
        let mem_word = ptrace::read(pid, addr)
            .expect("Could not read original bye from memory address");
        
        //println!("mem_word: {:x}", mem_word);

        let orig_byte = (mem_word & 0x000000ff) as u8;

        //println!("orig_byte: {:x}", orig_byte);

        // Func name for readability
        let func_name = String::from("Meme");

        // Construct breakpoint obj
        let bp = Breakpoint {
            addr,
            enabled,
            orig_byte,
            func_name,
        };

        let replace_byte = ((mem_word & 0xffffff00) | 0xCC) as *mut c_void;
        //println!("replace_byte: {:x}", replace_byte);
        
        // Write breakpoint into process memory
        ptrace::write(pid, addr, replace_byte)
            .expect("Could not write breakpoint");

        // Add breakpoint to debugger list of all breakpoints
        self.all_breakpoints.push(bp);

    }

    // This function will return a debugger object with all break points
    // initialized in the target process.
    pub fn attach(pid: u32) -> Debugger {
        let  start_time = Instant::now();

        // ptrace should be attached from the traceme()
        // in spawn_traceable_proc
        let pid = Pid::from_raw(pid as i32); 
        
        // Construct debugger
        let mut dbgr = Debugger {
            all_breakpoints: Vec::new(),
            hit_breakpoints: Vec::new(),
            coverage: 0,
            pid: pid,
            start_time: start_time,
        }; 

        //let before_break = ptrace::read(pid, 0x4004e7 as *mut c_void)
        //    .expect("Could not read proc mem")
        //    & 0x000000ff;
        //println!("before_break: {:x}", before_break);
    
        // This will call a method to initialize all breakpoints at addresses
        // of basic blocks enumerated from a program via basic blocks.
        dbgr.set_breakpoint(pid, 0x4004e7 as *mut c_void);

        // Restart process
        ptrace::cont(pid, None);

        dbgr
    }    

}
