# rickFuzz
Lets find some jpeg compressors and libraries on github and fuzz them.

# rickFuzz
Lets find some jpeg compressors and libraries on github and fuzz them.

Main goals for this project:

1). Implement a source-less coverage mechanism to improve fuzzing capability
	

 - Use a  RE scripting tool like Ghidra scripting, Radare(LOL), or IDAPython to extact the address of all basic blocks in a binary.
 - Create a Ptrace wrapper/library to set breakpoints at all the basic blocks.
 - Record when a breakpoint is hit and update coverage accordingly.
 - Change mutation based on coverage metrics

2). See if we can fuzz libjpeg or some other jpeg utilities found floating around on GitHub. 
- This fuzzer is not specific for fuzzing jpeg utilities. jpegs are a common and so many tools are written to parse them which leads to an abundance of test targets :).

# Current Weakpoints:

1). The binary needs to be compiled with -no-pie. For now we are relying on address rather than
binary load base + offset.
 - Should be fixed soon

2). Yeah this is my first ever Rust project so it may look pretty ghetto!
 - Improvements coming soon(tm)
