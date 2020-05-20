use nix::sys::wait::{waitpid, WaitStatus};
use nix::sys::signal::{Signal, SIGTRAP};
use nix::sys::ptrace;
use nix::sys::ptrace::AddressType;
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Instant};
use std::ffi::c_void;
use std::io::{BufRead, BufReader};
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

pub struct Debugger<'a> {
    /// Program and argv
    program: &'a [String],
    
    /// Path to file containing bp's
    bp_file: String,

    /// List of all breakpoints we want to apply to the targeted process
    all_breakpoints: Vec<Breakpoint>,

    /// Total breakpoints we started with so we can get percentage coverage
    total_original_breakpoints: usize,

    /// # of hit breakpoints so we can compare to total and use as coverage metric
    hit_breakpoints: usize,

    /// List of all break points we hit during execution
    coverage: f64,

    /// PID of debugee
    pid: Option<Pid>,

    /// When we attached to the target
    start_time: Option<Instant>,
}

impl<'a> Debugger<'a> {
    // Init a debugger object
    pub fn new(program: &'a [String], bpfile: String) -> Debugger<'a> {
        // Create new dbgr and fill in values
        let mut dbgr = Debugger {
            program: program,
            bp_file: bpfile,
            all_breakpoints: Vec::new(),
            total_original_breakpoints: 0,
            hit_breakpoints: 0,
            coverage: 0.0,
            pid: None, // This is a tmp val until we run the program
            start_time: None,
        }; 

        // We should return a debugger with only bp list populated
        dbgr.one_time_populate_bp();
        dbgr.total_original_breakpoints = dbgr.all_breakpoints.len();

        dbgr
    }

    // This is kinda hacky to populate the list of bp but its only run once
    pub fn one_time_populate_bp(&mut self) {
        self.spawn_traceable_proc();
        self.init_breakpoints();
        ptrace::kill(self.pid.unwrap()).expect("could not kill init proc");

    }

    // We want to start a new instance of the process and attach to it. We
    // then run traceme() as a pre exec so the tracer does not miss anything 
    // the child does. This probably doesn't matter but its good practice I guess.
    pub fn spawn_traceable_proc(&mut self) {
        //println!("Command: {}", &self.program);
        let mut tracee_pid: u32 = 0;
        unsafe {
            let child = Command::new(&self.program[0]).pre_exec(||{
                // Set ability to use ptrace on child
                ptrace::traceme()
                .expect("Pre exec call to traceme failed");
                Ok(())
            })
            .args(&self.program[1..])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed spawning debugee process");
            tracee_pid = child.id();
        }

        // Update our child pid
        self.pid = Some(Pid::from_raw(tracee_pid as i32));

        // Check to make sure child is stopped in exec 
        match waitpid(self.pid.unwrap(), None) {
            Ok(WaitStatus::Stopped(_, Signal::SIGTRAP)) => {
                //println!("Recieved correct state")
            },
            _ => panic!("Child in wrong state after traceme()")
        }

        // Bail if we try to attach to something we probably do not want to
        assert!(tracee_pid != 0, "Are you sure you want to trace pid: 0?");
        //println!("Tracee pid: {}", tracee_pid);
    }

    // Construct and set the breakpoint in the target process
    // In order to set a breakpoint with ptrace we need to set the 
    // byte pointed to by the breakpoint address to 0xcc(halt). Then
    // when the breakpoint is hit we replace the the halt with the 
    // original instruction and continue execution by setting eip = eip - 1.
    pub fn append_breakpoint(&mut self, bp_addr: AddressType, fname: &str) {

        //Breakpoint address
        let addr = bp_addr;
        //println!("addr {:?}", addr);

        // Breakpoint should be enable upon creation
        let enabled = true;
        
        // Read the orginal word from memory so we can add the int 3 to it
        let mem_word = ptrace::read(self.pid.unwrap(), addr)
            .expect("Could not read original bye from memory address");

        //println!("mem_word: {:x}", mem_word);
        
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
    
        /*
        // The word with int 3 inserted into it
        let replace_byte = ((mem_word & !0xff) | 0xCC) as *mut c_void;
        
        // Write the newly formed breakpoint into process memory
        ptrace::write(pid, addr, replace_byte)
            .expect("Could not write breakpoint");

        */
        // Add breakpoint to debugger list of all breakpoints
        self.all_breakpoints.push(bp);
    }

    pub fn update_coverage(&mut self) {
        self.hit_breakpoints += 1;
        self.coverage = (self.hit_breakpoints as f64 / self.total_original_breakpoints as f64) * 100.0;
        println!("Current coverage: {:.1}%", self.coverage);
    }

    // Resume execution after hitting a bp
    pub fn resume(&mut self) {
        // Get curr regs state
        let mut curr_regs = ptrace::getregs(self.pid.unwrap())
                         .expect("Failed getting regs in debug resume");
 
        // Get addr of breakpoint so we know if we hit it
        let curr_bp = (curr_regs.rip - 1) as *mut c_void;

        let mut del_ind = 0;
        for (i, bp) in self.all_breakpoints.iter().enumerate() {
            // Replace the byte with the original
            if bp.addr == curr_bp {
                println!("Hit breakpoint: {:?} in function: {}",
                         curr_bp, bp.func_name);
                
                // Write back original byte
                let bytes = ptrace::read(self.pid.unwrap(), curr_bp)
                    .expect("Could not read original bye from memory address");
                
                let replace_bytes = ((bytes & !0xff) | bp.orig_byte as i64) 
                    as *mut c_void;

                ptrace::write(self.pid.unwrap(), curr_bp, replace_bytes)
                    .expect("Failed replacing bytes to cont execution");

                // Reset rip
                curr_regs.rip = curr_regs.rip - 1;
                ptrace::setregs(self.pid.unwrap(), curr_regs)
                    .expect("Could not reset rip in resume");

                del_ind = i;
                break;
            }
        }
        // Update coverage metrics
        self.update_coverage();

        // Remove hit bp so its not set on the next run
        self.all_breakpoints.remove(del_ind);

        // Continue from where breakpoint
        ptrace::cont(self.pid.unwrap(), None);
    }

    // Takes the file containing the breakpoints and func names and populates
    // the debugger list of breakpoints
    pub fn init_breakpoints(&mut self) {
        //Open the breakpoint file
        let f = File::open(&self.bp_file).expect("Could not open breakpoint file");

        let f = BufReader::new(f);

        for line in f.lines() {
            let tmpstr = line.unwrap();
            let bp_info: Vec<&str> = tmpstr.split(' ').collect();
            //println!("{}:{}", &bp_info[0], &bp_info[1]);

            let addr = &bp_info[0].trim_start_matches("0x");
            let addr = i64::from_str_radix(addr, 16).unwrap() as *mut c_void;
            //println!("cleaned addr {:?}", addr);

           self.append_breakpoint(addr, &bp_info[1]); 
        }
    }
    
    // Write the breakpoints we initialized into the process
    pub fn set_bp_from_list(&mut self) {
        for bp in &self.all_breakpoints {
            // Read the orginal word from memory so we can add the int 3 to it
            let mem_word = ptrace::read(self.pid.unwrap(), bp.addr)
                .expect("Could not read original bye from memory address");
            
            // The word with int 3 inserted into it
            let replace_byte = ((mem_word & !0xff) | 0xCC) as *mut c_void;
            //println!("Replace byte: {:?}", replace_byte);
            
            // Write the newly formed breakpoint into process memory
            ptrace::write(self.pid.unwrap(), bp.addr, replace_byte)
                .expect("Could not write breakpoint");
        }
    }

    // This function spawns a new process and sets the appropriate
    // break points. Once set the process is run until it exits and
    // collects coverage while its running.
    pub fn attach_and_run(&mut self) {
        // Set time we attached
        self.start_time = Some(Instant::now());

        // Spawn the process
        self.spawn_traceable_proc();

        // Now we can set the breakpoints
        self.set_bp_from_list();

        // Restart process
        ptrace::cont(self.pid.unwrap(), None);

        // Loop and wait for signals that we hit a breakpoint
        while(waitpid(self.pid.unwrap(), None).expect("failed waiting") 
              == WaitStatus::Stopped(self.pid.unwrap(), SIGTRAP))
        {
            self.resume();
        }

    }    

}
