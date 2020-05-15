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
 
 # Sample Coverage Output:
 ```
 Hit breakpoint: 0x405bae in function: finish_output_ppm
Current coverage: 9.4%
Hit breakpoint: 0x4015e0 in function: fflush
Current coverage: 9.5%
Hit breakpoint: 0x4015e6 in function: 004015e6
Current coverage: 9.6%
Hit breakpoint: 0x4014b0 in function: ferror
Current coverage: 9.8%
Hit breakpoint: 0x4014b6 in function: 004014b6
Current coverage: 9.9%
Hit breakpoint: 0x405c03 in function: LAB_00405c03
Current coverage: 10.0%
Hit breakpoint: 0x401630 in function: jpeg_finish_decompress
Current coverage: 10.1%
Hit breakpoint: 0x401636 in function: 00401636
Current coverage: 10.2%
Hit breakpoint: 0x401550 in function: jpeg_destroy_decompress
Current coverage: 10.4%
Hit breakpoint: 0x401556 in function: 00401556
Current coverage: 10.5%
Hit breakpoint: 0x40351d in function: 0040351d
Current coverage: 10.6%
Hit breakpoint: 0x4014e0 in function: fclose
Current coverage: 10.7%
Hit breakpoint: 0x4014e6 in function: 004014e6
Current coverage: 10.9%
Hit breakpoint: 0x40352c in function: LAB_0040352c
Current coverage: 11.0%
Hit breakpoint: 0x40354b in function: LAB_0040354b
Current coverage: 11.1%
Hit breakpoint: 0x403564 in function: LAB_00403564
Current coverage: 11.2%
Hit breakpoint: 0x403577 in function: LAB_00403577
Current coverage: 11.3%
Hit breakpoint: 0x40357c in function: LAB_0040357c
Current coverage: 11.5%
Hit breakpoint: 0x401670 in function: exit
Current coverage: 11.6%
Hit breakpoint: 0x401676 in function: 00401676
Current coverage: 11.7%
Hit breakpoint: 0x403570 in function: 00403570
Current coverage: 11.8%
 ```
