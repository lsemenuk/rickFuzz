use nix::sys::wait::{waitpid, WaitStatus};
use nix::sys::signal::Signal;
use nix::sys::ptrace;
use nix::sys::ptrace::AddressType;
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::{Instant};
use std::ffi::c_void;
use std::io::{self,BufRead, BufReader};
use std::fs::File;

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
                //println!("Recieved correct state")
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
    pub fn set_breakpoint(&mut self, pid: Pid, bp_addr: AddressType, fname: &str) {

        //Breakpoint address
        let addr = bp_addr;

        // Breakpoint should be enable upon creation
        let enabled = true;
        
        // Read the orginal word from memory so we can add the int 3 to it
        let mem_word = ptrace::read(pid, addr)
            .expect("Could not read original bye from memory address");
        
        // Save the bye we over wrote 
        let orig_byte = (mem_word & 0x000000ff) as u8;

        // Func name for readability
        let func_name = String::from(fname);

        // Construct breakpoint obj
        let bp = Breakpoint {
            addr,
            enabled,
            orig_byte,
            func_name,
        };

        // The word with int 3 inserted into it
        let replace_byte = ((mem_word & !0xff) | 0xCC) as *mut c_void;
        
        // Write the newly formed breakpoint into process memory
        ptrace::write(pid, addr, replace_byte)
            .expect("Could not write breakpoint");

        // Add breakpoint to debugger list of all breakpoints
        self.all_breakpoints.push(bp);

    }

    // Resume execution after hitting a bp
    pub fn resume(&mut self) {
        // Get curr regs state
        let mut curr_regs = ptrace::getregs(self.pid)
                         .expect("Failed getting regs in debug resume");
 
        // Get addr of breakpoint so we know if we hit it
        let curr_bp = (curr_regs.rip - 1) as *mut c_void;

        for bp in &self.all_breakpoints {
            // Replace the byte with the original
            if bp.addr == curr_bp {
                println!("Hit breakpoint: {:?} in function: {}",
                         curr_bp, bp.func_name);
                
                // Write back original byte
                let bytes = ptrace::read(self.pid, curr_bp)
                    .expect("Could not read original bye from memory address");
                
                let replace_bytes = ((bytes & !0xff) | bp.orig_byte as i64) 
                    as *mut c_void;

                ptrace::write(self.pid, curr_bp, replace_bytes)
                    .expect("Failed replacing bytes to cont execution");

                // Reset rip
                curr_regs.rip = curr_regs.rip - 1;
                ptrace::setregs(self.pid, curr_regs)
                    .expect("Could not reset rip in resume");

                break;
            }
        }
        ptrace::cont(self.pid, None);
    }

    // Takes the file containing the breakpoints and func names and populates
    // the debugger list of breakpoints
    pub fn init_breakpoints(&mut self, infile: &str) {
        //Open the breakpoint file
        let f = File::open(infile).expect("Could not open breakpoint file");

        let f = BufReader::new(f);

        for line in f.lines() {
            let tmpstr = line.unwrap();
            let bp_info: Vec<&str> = tmpstr.split(' ').collect();
            //println!("{}:{}", &bp_info[0], &bp_info[1]);

            let addr = &bp_info[0].trim_start_matches("0x");
            let addr = i64::from_str_radix(addr, 16).unwrap() as *mut c_void;
            //println!("cleaned addr {:?}", addr);

           self.set_breakpoint(self.pid, addr, &bp_info[1]); 
        }
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

        // This will call a method to initialize all breakpoints at addresses
        // of basic blocks enumerated from a program via basic blocks.
        //dbgr.set_breakpoint(pid, 0x4004e7 as *mut c_void);

        dbgr.init_breakpoints("breakpoints.txt");

        // Restart process
        ptrace::cont(pid, None);

        waitpid(pid, None).expect("failed waiting");

        dbgr.resume();

        //waitpid(pid, None).expect("failed waiting");

        dbgr
    }    

}
